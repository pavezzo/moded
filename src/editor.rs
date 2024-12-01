use std::{env, fs, io::Write, path::{Path, PathBuf}, sync::atomic};

use crate::{command_bar::{match_cmd, CommandBarAction}, gap_buffer::{LinePos, LineView, TextBuffer}, indent::indent_wanted, search::search, vim_commands::*, CursorPos, SpecialKey, State};

static LAST_BUFFER_ID: atomic::AtomicUsize = atomic::AtomicUsize::new(0);
pub fn next_buffer_id() -> usize {
    LAST_BUFFER_ID.fetch_add(1, atomic::Ordering::Relaxed)
}


#[derive(PartialEq, Eq, Clone, Copy)]
pub enum EditorMode {
    Insert,
    Normal,
    Visual,
    VisualLine,
    CommandBar,
    Search,
}

pub struct Editor {
    pub buffers: Vec<TextBuffer>,
    pub cursors: Vec<CursorPos>,
    pub current_buffer: usize,
    pub root_folder: PathBuf,
    pub search_results: Vec<LinePos>,
    pub command_bar_input: String,
    pub visual_range_anchor: LinePos,
    pub motion: Motion,
    pub mode: EditorMode,
}


impl Editor {
    pub fn from_path(path: &Path) -> Self {
        println!("{path:?}");
        let buf = TextBuffer::from_path(next_buffer_id(), path);
        let cursor = CursorPos::new(buf.id);
        let root = env::current_dir().expect("Didn't find current dir");

        Self { 
            buffers: vec![buf],
            cursors: vec![cursor],
            current_buffer: 0,
            root_folder: root,
            mode: EditorMode::Normal,
            motion: Motion::new(),
            visual_range_anchor: LinePos { line: 0, col: 0 },
            command_bar_input: String::new(),
            search_results: Vec::new(),
        }
    }

    pub fn save_to_file(&mut self) {
        let Some(buffer) = self.buffers.get_mut(self.current_buffer) else { return };
        let view = buffer.full_view();
        let Some(file_path) = &buffer.file_path else { return };
        let mut file = std::fs::File::create(file_path).unwrap();
        match view {
            LineView::Contiguous(s) => {
                file.write_all(s.as_bytes()).unwrap();
            },
            LineView::Parts(s1, s2) => {
                file.write_all(s1.as_bytes()).unwrap();
                file.write_all(s2.as_bytes()).unwrap();
            },
        }
    }

    pub fn handle_input(&mut self, state: &mut State) {
        let Some(buffer) = self.buffers.get_mut(self.current_buffer) else { return };
        let Some(cursor) = self.cursors.get_mut(self.current_buffer) else { return };
        if self.mode ==  EditorMode::Insert {
            let line = cursor.y - 1;
            if !state.io.chars.is_empty() {
                buffer.insert_into_line(line, cursor.x - 1, state.io.chars.as_bytes());
                cursor.x += state.io.chars.chars().count();
            }
            if state.io.pressed_special(SpecialKey::Enter) {
                let line_len = buffer.line_len(line);
                if line_len - (cursor.x - 1) > 0 {
                    buffer.split_line_at_index(line, cursor.x - 1);
                } else {
                    buffer.insert_empty_line(cursor.y);
                }
                cursor.y += 1;
                cursor.x = 1;

                let indent = indent_wanted(line + 1, &buffer);
                if let Some(indent) = indent {
                    if indent > 0 {
                        buffer.insert_into_line(line + 1, 0, " ".repeat(indent).as_bytes());
                        cursor.x = indent + 1;
                        cursor.wanted_x = cursor.x;
                    }
                }
            }
            if state.io.pressed_special(SpecialKey::Tab) {
                buffer.insert_into_line(line, cursor.x - 1, " ".repeat(4).as_bytes());
                cursor.x += 4;
                cursor.wanted_x = cursor.x;
            }
            if state.io.pressed_special(SpecialKey::Escape) {
                self.mode = EditorMode::Normal;
                cursor.x -= 1;
                cursor.x = cursor.x.max(1);
                cursor.wanted_x = cursor.x;
            }
            if state.io.pressed_special(SpecialKey::Backspace) {
                let row_len = buffer.line_len(line);
                if row_len > 0 && cursor.x > 1 {
                    buffer.remove_from_line(line, cursor.x as usize - 2, 1);
                    cursor.x -= 1;
                    cursor.wanted_x = cursor.x;
                } else if cursor.x == 1 && cursor.y > 1 {
                    let next_cursor_pos = buffer.line_len(line - 1);
                    buffer.remove_line_sep(line - 1);
                    cursor.x = next_cursor_pos + 1;
                    cursor.wanted_x = cursor.x;
                    cursor.y -= 1;
                }
            }
        } else if self.mode == EditorMode::CommandBar {
            if !state.io.chars.is_empty() {
                self.command_bar_input.push_str(&state.io.chars);
                state.cmd_bar_cursor_x += state.io.chars.chars().count();
            }
            if state.io.pressed_special(SpecialKey::Enter) {
                let parts = self.command_bar_input.splitn(2, " ").collect::<Vec<_>>();
                let func = match_cmd(&parts[0][1..]);
                let Some(func) = func else { return };
                let res = if parts.len() > 1 {
                    func(state, &self, parts[1])
                } else {
                    func(state, &self, &"")
                };

                match res {
                    Ok(CommandBarAction::NewBuffer(buf)) => {
                        self.cursors.push(CursorPos::new(buf.id));
                        self.buffers.push(buf);
                        self.current_buffer = self.buffers.len() - 1;
                    },
                    Ok(CommandBarAction::SwitchToBuffer(buf)) => {
                        self.current_buffer = buf;
                    },
                    Ok(CommandBarAction::None) => {}, 
                    Err(_) => todo!(),
                    _ => todo!(),
                }

                //println!("executing cmd: {}", self.command_bar_input);
                state.cmd_bar_cursor_x = 1;
                self.command_bar_input.clear();
                self.mode = EditorMode::Normal;
            }
            if state.io.pressed_special(SpecialKey::Backspace) {
                self.command_bar_input.pop();
                state.cmd_bar_cursor_x -= 1;
            }
            if state.io.pressed_special(SpecialKey::Escape) {
                self.command_bar_input.clear();
                self.mode = EditorMode::Normal;
            }
            if self.command_bar_input.is_empty() {
                self.mode = EditorMode::Normal;
            }
        } else if self.mode == EditorMode::Search {
            if !state.io.chars.is_empty() {
                self.command_bar_input.push_str(&state.io.chars);
                state.cmd_bar_cursor_x += 1;
                let positions = search(&self.command_bar_input.as_bytes()[1..], &buffer);
                self.search_results = positions;
            }
            if state.io.pressed_special(SpecialKey::Backspace) {
                self.command_bar_input.pop();
                state.cmd_bar_cursor_x -= 1;
            }
            if state.io.pressed_special(SpecialKey::Enter) {
                if let Some(pos) = closest_position(cursor.to_linepos(), &self.search_results) {
                    cursor.from_linepos(pos);
                }
                self.command_bar_input.clear();
                self.mode = EditorMode::Normal;
            }
            if state.io.pressed_special(SpecialKey::Escape) {
                self.command_bar_input.clear();
                self.mode = EditorMode::Normal;
            }
            if self.command_bar_input.is_empty() {
                self.mode = EditorMode::Normal;
            }
        } else {
            let chars = state.io.chars.chars().collect::<Vec<_>>();
            for char in chars {
                self.motion.parse(&state, char, self.mode);
                if self.execute_cmd(state) {
                    self.motion.clear();
                }
            }
            //self.execute_commands(state);
            if state.io.pressed_special(SpecialKey::Escape) {
                self.motion.clear();
                self.mode = EditorMode::Normal;
            }
        } 
    }

    fn execute_cmd(&mut self, state: &mut State) -> bool {
        let Some(buffer) = self.buffers.get_mut(self.current_buffer) else { return true };
        let Some(current_cursor) = self.cursors.get_mut(self.current_buffer) else { return true };
        let Some(obj) = self.motion.object else { return false };
        let cursor = current_cursor.to_linepos();

        match obj {
            Object::BackWord => 'b: {
                let Some(pos) = find_previous_word_start(cursor, &buffer) else { break 'b };
                if self.motion.action == Some(Action::Delete) {
                    buffer.remove_by_range(pos, cursor);
                }
                current_cursor.from_linepos(pos);
            },
            Object::BackWORD => {
                todo!();
                //let Some(pos) = find_previous_WORD_start(cursor, &buffer) else { break 'b };
            },
            Object::Word => 'b: {
                if self.motion.modifier == Some(Modifier::Inside) {
                    let Some(start) = find_current_word_start(cursor, &buffer) else { break 'b };
                    let Some(end) = find_current_word_end(cursor, &buffer) else { break 'b };
                    if self.motion.action == Some(Action::Delete) {
                        buffer.remove_from_line(cursor.line, start.col, end.col - start.col + 1);
                        current_cursor.x = ((start.col + 1).min(buffer.line_len(cursor.line))).max(1);
                        current_cursor.wanted_x = current_cursor.x;
                    } else if self.mode == EditorMode::Visual {
                        self.visual_range_anchor = start;
                        current_cursor.from_linepos(end);
                    }
                } else {
                    let pos = if let Some(Modifier::Count(n)) = self.motion.modifier {
                        count(cursor, &buffer, n, find_next_word_start)
                    } else {
                        find_next_word_start(cursor, &buffer)
                    };
                    let Some(mut pos) = pos else { break 'b };

                    if self.motion.action == Some(Action::Delete) {
                        if pos.col > 0 {
                            pos.col -= 1;
                        } else {
                            pos.line -= 1;
                            pos.col = buffer.line_len(pos.line);
                        }
                        buffer.remove_by_range(cursor, pos);
                    } else {
                        current_cursor.from_linepos(pos);
                    }
                }
            },
            Object::WordEnd => 'b: {
                let pos = if let Some(Modifier::Count(n)) = self.motion.modifier {
                    count(cursor, &buffer, n, find_next_word_end)
                } else {
                    find_next_word_end(cursor, &buffer)
                };
                let Some(pos) = pos else { break 'b };

                if self.motion.action == Some(Action::Delete) {
                    buffer.remove_by_range(cursor, pos);
                } else {
                    current_cursor.from_linepos(pos);
                }
            },
            Object::WORDEnd => todo!(),
            Object::WORD => 'b: {
                if self.motion.modifier == Some(Modifier::Inside) {
                    let Some(start) = find_current_WORD_start(cursor, &buffer) else { break 'b };
                    let Some(end) = find_current_WORD_end(cursor, &buffer) else { break 'b };
                    if self.motion.action == Some(Action::Delete) {
                        buffer.remove_by_range(start, end);
                        current_cursor.x = ((start.col + 1).min(buffer.line_len(cursor.line))).max(1);
                        current_cursor.wanted_x = current_cursor.x;
                    } else if self.mode == EditorMode::Visual {
                        self.visual_range_anchor = start;
                        current_cursor.from_linepos(end);
                    }
                } else {
                    let pos = if let Some(Modifier::Count(n)) = self.motion.modifier {
                        count(cursor, &buffer, n, find_next_WORD_start)
                    } else {
                        find_next_WORD_start(cursor, &buffer)
                    };
                    let Some(mut pos) = pos else { break 'b };

                    if self.motion.action == Some(Action::Delete) {
                        if pos.col > 0 {
                            pos.col -= 1;
                        } else {
                            pos.line -= 1;
                            pos.col = buffer.line_len(pos.line);
                        }
                        buffer.remove_by_range(cursor, pos);
                    } else {
                        current_cursor.from_linepos(pos);
                    }
                }
            },
            Object::Append => {
                self.mode = EditorMode::Insert;
                let line_len = buffer.line_len(cursor.line);
                if line_len > 0 {
                    current_cursor.x += 1;
                }
            },
            Object::Insert => self.mode = EditorMode::Insert,
            Object::NormalMode => self.mode = EditorMode::Normal,
            Object::VisualMode => {
                self.mode = EditorMode::Visual;
                self.visual_range_anchor = cursor;
            },
            Object::VisualLineMode => {
                self.mode = EditorMode::VisualLine;
                self.visual_range_anchor = cursor;
            },
            Object::VisualSelection => {
                if self.motion.action == Some(Action::Delete) {
                    if self.mode == EditorMode::Visual {
                        let min = self.visual_range_anchor.min(cursor);
                        let max = self.visual_range_anchor.max(cursor);
                        buffer.remove_by_range(min, max);

                        current_cursor.from_linepos(min);
                        self.mode = EditorMode::Normal;
                    } else if self.mode == EditorMode::VisualLine {
                        let mut start = self.visual_range_anchor.min(cursor);
                        let end = self.visual_range_anchor.max(cursor);
                        for _ in start.line..(end.line + 1) {
                            buffer.remove_line(start.line);
                        }

                        start.line = start.line.min(buffer.total_lines() - 1);
                        let line_len = buffer.line_len(start.line);
                        start.col = start.col.min(line_len);
                        current_cursor.from_linepos(start);
                        
                        self.mode = EditorMode::Normal;
                    }
                }
            },
            Object::CommandBarMode => {
                self.mode = EditorMode::CommandBar;
                self.command_bar_input.push(':');
                state.cmd_bar_cursor_x = 1;
            },
            Object::Up => {
                if cursor.line > 0 {
                    current_cursor.y -= 1;
                    let max_x = (buffer.line_len(cursor.line - 1)).max(1);
                    if current_cursor.wanted_x > max_x {
                        current_cursor.x = max_x;
                    } else {
                        current_cursor.x = current_cursor.wanted_x;
                    }
                }
            },
            Object::Down => {
                if cursor.line < buffer.total_lines() - 1 {
                    current_cursor.y += 1;
                    let max_x = buffer.line_len(cursor.line + 1).max(1);
                    if current_cursor.wanted_x > max_x {
                        current_cursor.x = max_x;
                    } else {
                        current_cursor.x = current_cursor.wanted_x;
                    }
                }
            },
            Object::Left => {
                if cursor.col > 0 {
                    current_cursor.x -= 1;
                    current_cursor.wanted_x = current_cursor.x;
                }
            },
            Object::Right => {
                let line_len = buffer.line_len(cursor.line);
                if cursor.col + 1 < line_len {
                    current_cursor.x += 1;
                    current_cursor.wanted_x += 1;
                } else if self.mode == EditorMode::Visual && current_cursor.x == line_len {
                    // go one over like in vim to delete whole line + newline
                    current_cursor.x += 1;
                    current_cursor.wanted_x += 1;
                }
            },
            Object::Line => 'b: {
                if self.motion.action == Some(Action::Delete) {
                    buffer.remove_line(cursor.line);
                    if cursor.line == buffer.total_lines() && cursor.line > 0 {
                        current_cursor.y -= 1;
                    }
                    let line_len = buffer.line_len(current_cursor.y - 1);
                    if cursor.col >= line_len {
                        current_cursor.x = line_len.max(1);
                    }
                    break 'b
                }

                if self.motion.action == Some(Action::Goto) {
                    let line = if let Some(Modifier::Count(n)) = self.motion.modifier { n as usize } else { 1 };
                    let total_lines = buffer.total_lines();
                    let line = line.min(total_lines);
                    let line_len = buffer.line_len(line - 1);
                    current_cursor.y = line;
                    current_cursor.x = current_cursor.x.min(line_len + 1);
                    break 'b
                }

                if self.motion.action == Some(Action::GOTO) {
                    if let Some(Modifier::Count(n)) = self.motion.modifier {
                        let line = n as usize;
                        let total_lines = buffer.total_lines();
                        let line = line.min(total_lines);
                        let line_len = buffer.line_len(line - 1);
                        current_cursor.y = line;
                        current_cursor.x = current_cursor.x.min(line_len);
                    } else {
                        let last_line = buffer.total_lines() - 1;
                        let line_len = buffer.line_len(last_line);
                        current_cursor.y = last_line + 1;
                        current_cursor.x = current_cursor.wanted_x.min(line_len);
                    }
                }
            },
            Object::LineStart => {
                if self.motion.action == Some(Action::Delete) {
                    buffer.remove_from_line(cursor.line, 0, cursor.col);
                }
                current_cursor.x = 1;
                current_cursor.wanted_x = 1;
            },
            Object::LineEnd => 'b: {
                if self.motion.action == Some(Action::Delete) {
                    let line_len = buffer.line_len(cursor.line);
                    buffer.remove_from_line(cursor.line, cursor.col, line_len - cursor.col);
                    if cursor.col > 0 {
                        current_cursor.x -= 1;
                        current_cursor.wanted_x = current_cursor.x;
                    }
                    break 'b
                }

                // go one over like in vim
                if self.mode == EditorMode::Visual {
                    current_cursor.x = (buffer.line_len(current_cursor.y as usize - 1) + 1).max(1);
                } else {
                    current_cursor.x = (buffer.line_len(current_cursor.y as usize - 1)).max(1);
                }
                current_cursor.wanted_x = current_cursor.x;
            },
            Object::CharUnderCursor => {
                let n = if let Some(Modifier::Count(n)) = self.motion.modifier { n } else { 1 };
                let line_len = buffer.line_len(cursor.line);
                if line_len > 0 {
                    buffer.remove_from_line(cursor.line, cursor.col, (n as usize).min(line_len - cursor.col));
                    if (current_cursor.x - 1) as usize >= (line_len - 1) && current_cursor.x > 1 {
                        current_cursor.x -= 1;
                        current_cursor.wanted_x = current_cursor.x;
                    }
                }
            },
            Object::SearchMode => {
                self.mode = EditorMode::Search;
                self.command_bar_input.push('/');
                state.cmd_bar_cursor_x = 1;
            },
            Object::NextSearchResult => 'b: {
                let Some(pos) = next_position(cursor, &self.search_results) else { break 'b };
                current_cursor.from_linepos(pos);
            },
            Object::PreviousSearchResult => 'b: {
                let Some(pos) = previous_position(cursor, &self.search_results) else { break 'b };
                current_cursor.from_linepos(pos);
            },
            Object::PageTop => 'b: {
                if self.motion.action == Some(Action::Scroll) {
                    state.start_line = cursor.line;
                    break 'b
                }
            },
            Object::PageMiddle => 'b: {
                if self.motion.action == Some(Action::Scroll) {
                    let middle = state.max_rows() / 2 + state.start_line;
                    let offset = middle.max(cursor.line) - middle.min(cursor.line);
                    if middle > cursor.line {
                        state.start_line -= offset.min(state.start_line);
                    } else {
                        state.start_line += offset;
                    }
                    break 'b
                }
            },
            Object::PageBot => 'b: {
                if self.motion.action == Some(Action::Scroll) {
                    if cursor.line > state.max_rows() {
                        state.start_line = state.start_line + state.max_rows() - cursor.line;
                    } else {
                        state.start_line = 0;
                    }
                    break 'b
                }
            },
            Object::HalfScreenUp => {
                if self.motion.action == Some(Action::Scroll) {
                    let half = state.max_rows() / 2;
                    current_cursor.y -= current_cursor.y.min(half);
                    current_cursor.y = current_cursor.y.max(1);
                    current_cursor.x = current_cursor.wanted_x;
                    current_cursor.x = current_cursor.x.min(buffer.line_len(current_cursor.y - 1).max(1));
                }
            },
            Object::HalfScreenDown => {
                if self.motion.action == Some(Action::Scroll) {
                    let half = state.max_rows() / 2;
                    current_cursor.y += half;
                    current_cursor.y = current_cursor.y.min(buffer.total_lines());
                    current_cursor.x = current_cursor.wanted_x;
                    current_cursor.x = current_cursor.x.min(buffer.line_len(current_cursor.y - 1).max(1));
                }
            },
            Object::InsertLineUp => {
                buffer.insert_empty_line(cursor.line);
                let indent = indent_wanted(cursor.line, &buffer);
                if let Some(indent) = indent {
                    buffer.insert_into_line(cursor.line, 0, " ".repeat(indent).as_bytes());
                    current_cursor.x = indent + 1;
                    current_cursor.wanted_x = current_cursor.x;
                } else {
                    current_cursor.x = 1;
                    current_cursor.wanted_x = current_cursor.x;
                }
                self.mode = EditorMode::Insert;
            },
            Object::InsertLineDown => {
                buffer.insert_empty_line(cursor.line + 1);
                let indent = indent_wanted(cursor.line + 1, &buffer);
                if let Some(indent) = indent {
                    buffer.insert_into_line(cursor.line + 1, 0, " ".repeat(indent).as_bytes());
                    current_cursor.x = indent + 1;
                    current_cursor.wanted_x = current_cursor.x;
                } else {
                    current_cursor.x = 0;
                    current_cursor.wanted_x = current_cursor.x;
                }
                current_cursor.y += 1;
                self.mode = EditorMode::Insert;
            },
        }

        true
    }
}

fn closest_position(cursor: LinePos, positions: &[LinePos]) -> Option<LinePos> {
    if positions.is_empty() {
        return None
    }
    let mut pos = positions.binary_search(&cursor).unwrap_or_else(|e| e);
    if pos == positions.len() {
        pos = 0;
    }
    Some(positions[pos])
}

fn next_position(cursor: LinePos, positions: &[LinePos]) -> Option<LinePos> {
    if positions.is_empty() {
        return None
    }

    let pos = match positions.binary_search(&cursor) {
        Ok(n) => n + 1,
        Err(n) => n,
    };

    if let Some(pos) = positions.get(pos) {
        return Some(*pos)
    }

    Some(positions[0])
}

fn previous_position(cursor: LinePos, positions: &[LinePos]) -> Option<LinePos> {
    if positions.is_empty() {
        return None
    }

    let mut pos = positions.binary_search(&cursor).unwrap_or_else(|n| n);
    if pos == 0 {
        pos = positions.len() - 1
    } else {
        pos -= 1;
    }

    Some(positions[pos])
}
