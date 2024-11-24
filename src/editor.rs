use std::{fs, io::{self, Read, Write}, path::{Path, PathBuf}};

use crate::{gap_buffer::{LinePos, LineView, TextBuffer}, vim_commands::*, SpecialKey, State};

#[derive(PartialEq, Eq, Clone, Copy)]
pub enum EditorMode {
    Insert,
    Normal,
    Visual,
    VisualLine,
    CommandBar,
}

pub struct Editor {
    pub buffer: TextBuffer,
    pub file_path: PathBuf,
    pub mode: EditorMode,
    pub motion: Motion,
    pub visual_range_anchor: LinePos,
    pub command_bar_input: String,
}


impl Editor {
    pub fn from_path(path: &Path) -> Self {
        println!("{path:?}");
        let mut lines: Vec<_> = Vec::new();
        if path.is_file() {
            let file = fs::File::open(path).unwrap();
            let mut reader = io::BufReader::new(file);
            reader.read_to_end(&mut lines).expect("can't read file to end");

            return Self { 
                buffer: TextBuffer::from_data(lines),
                file_path: path.to_owned(),
                mode: EditorMode::Normal,
                motion: Motion::new(),
                visual_range_anchor: LinePos { line: 0, col: 0 },
                command_bar_input: String::new(),
            }
        }
        todo!();
    }

    pub fn save_to_file(&self) {
        let view = self.buffer.full_view();
        let mut file = std::fs::File::create(&self.file_path).unwrap();
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
        if self.mode ==  EditorMode::Insert {
            let line = state.cursor.y as usize - 1;
            if !state.io.chars.is_empty() {
                self.buffer.insert_into_line(line, state.cursor.x as usize - 1, state.io.chars.as_bytes());
                state.cursor.x += state.io.chars.chars().count();
            }
            if state.io.pressed_special(SpecialKey::Enter) {
                let line_len = self.buffer.line_len(line);
                if line_len - (state.cursor.x as usize - 1) > 0 {
                    self.buffer.split_line_at_index(line, state.cursor.x as usize - 1);
                } else {
                    self.buffer.insert_empty_line(state.cursor.y as usize);
                }
                state.cursor.y += 1;
                state.cursor.x = 1;
            }
            if state.io.pressed_special(SpecialKey::Escape) {
                self.mode = EditorMode::Normal;
                state.cursor.x -= 1;
                state.cursor.x = state.cursor.x.max(1);
                state.cursor.wanted_x = state.cursor.x;
            }
            if state.io.pressed_special(SpecialKey::Backspace) {
                let row_len = self.buffer.line_len(line);
                if row_len > 0 && state.cursor.x > 1 {
                    self.buffer.remove_from_line(line, state.cursor.x as usize - 2, 1);
                    state.cursor.x -= 1;
                    state.cursor.wanted_x = state.cursor.x;
                } else if state.cursor.x == 1 && state.cursor.y > 1 {
                    let next_cursor_pos = self.buffer.line_len(line - 1);
                    self.buffer.remove_line_sep(line - 1);
                    state.cursor.x = next_cursor_pos + 1;
                    state.cursor.wanted_x = state.cursor.x;
                    state.cursor.y -= 1;
                }
            }
        } else if self.mode == EditorMode::CommandBar {
            if !state.io.chars.is_empty() {
                self.command_bar_input.push_str(&state.io.chars);
                state.cmd_bar_cursor_x += 1;
            }
            if state.io.pressed_special(SpecialKey::Enter) {
                println!("executing cmd: {}", self.command_bar_input);
                state.cmd_bar_cursor_x = 1;
                self.command_bar_input.clear();
                self.mode = EditorMode::Normal;
            }
            if state.io.pressed_special(SpecialKey::Backspace) {
                self.command_bar_input.pop();
                state.cmd_bar_cursor_x -= 1;
            }
            if self.command_bar_input.is_empty() {
                self.mode = EditorMode::Normal;
            }
        } else {
            let chars = state.io.chars.chars().collect::<Vec<_>>();
            for char in chars {
                self.motion.parse(char, self.mode);
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
        let Some(obj) = self.motion.object else { return false };
        let cursor = state.cursor.to_linepos();

        match obj {
            Object::BackWord => 'b: {
                let Some(pos) = find_previous_word_start(cursor, &self.buffer) else { break 'b };
                if self.motion.action == Some(Action::Delete) {
                    self.buffer.remove_by_range(pos, cursor);
                }
                state.cursor.from_linepos(pos);
            },
            Object::BackWORD => {
                todo!();
                //let Some(pos) = find_previous_WORD_start(cursor, &self.buffer) else { break 'b };
            },
            Object::Word => 'b: {
                if self.motion.modifier == Some(Modifier::Inside) {
                    let Some(start) = find_current_word_start(cursor, &self.buffer) else { break 'b };
                    let Some(end) = find_current_word_end(cursor, &self.buffer) else { break 'b };
                    if self.motion.action == Some(Action::Delete) {
                        self.buffer.remove_from_line(cursor.line, start.col, end.col - start.col + 1);
                        state.cursor.x = ((start.col + 1).min(self.buffer.line_len(cursor.line))).max(1);
                        state.cursor.wanted_x = state.cursor.x;
                    } else if self.mode == EditorMode::Visual {
                        self.visual_range_anchor = start;
                        state.cursor.from_linepos(end);
                    }
                } else {
                    let pos = if let Some(Modifier::Count(n)) = self.motion.modifier {
                        count(cursor, &self.buffer, n, find_next_word_start)
                    } else {
                        find_next_word_start(cursor, &self.buffer)
                    };
                    let Some(mut pos) = pos else { break 'b };

                    if self.motion.action == Some(Action::Delete) {
                        if pos.col > 0 {
                            pos.col -= 1;
                        } else {
                            pos.line -= 1;
                            pos.col = self.buffer.line_len(pos.line);
                        }
                        self.buffer.remove_by_range(cursor, pos);
                    } else {
                        state.cursor.from_linepos(pos);
                    }
                }
            },
            Object::WordEnd => 'b: {
                let pos = if let Some(Modifier::Count(n)) = self.motion.modifier {
                    count(cursor, &self.buffer, n, find_next_word_end)
                } else {
                    find_next_word_end(cursor, &self.buffer)
                };
                let Some(pos) = pos else { break 'b };

                if self.motion.action == Some(Action::Delete) {
                    self.buffer.remove_by_range(cursor, pos);
                } else {
                    state.cursor.from_linepos(pos);
                }
            },
            Object::WORDEnd => todo!(),
            Object::WORD => 'b: {
                if self.motion.modifier == Some(Modifier::Inside) {
                    let Some(start) = find_current_WORD_start(cursor, &self.buffer) else { break 'b };
                    let Some(end) = find_current_WORD_end(cursor, &self.buffer) else { break 'b };
                    if self.motion.action == Some(Action::Delete) {
                        self.buffer.remove_by_range(start, end);
                        state.cursor.x = ((start.col + 1).min(self.buffer.line_len(cursor.line))).max(1);
                        state.cursor.wanted_x = state.cursor.x;
                    } else if self.mode == EditorMode::Visual {
                        self.visual_range_anchor = start;
                        state.cursor.from_linepos(end);
                    }
                } else {
                    let pos = if let Some(Modifier::Count(n)) = self.motion.modifier {
                        count(cursor, &self.buffer, n, find_next_WORD_start)
                    } else {
                        find_next_WORD_start(cursor, &self.buffer)
                    };
                    let Some(mut pos) = pos else { break 'b };

                    if self.motion.action == Some(Action::Delete) {
                        if pos.col > 0 {
                            pos.col -= 1;
                        } else {
                            pos.line -= 1;
                            pos.col = self.buffer.line_len(pos.line);
                        }
                        self.buffer.remove_by_range(cursor, pos);
                    } else {
                        state.cursor.from_linepos(pos);
                    }
                }
            },
            Object::Append => {
                self.mode = EditorMode::Insert;
                let line_len = self.buffer.line_len(cursor.line);
                if line_len > 0 {
                    state.cursor.x += 1;
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
                        self.buffer.remove_by_range(min, max);

                        state.cursor.from_linepos(min);
                        self.mode = EditorMode::Normal;
                    } else if self.mode == EditorMode::VisualLine {
                        let mut start = self.visual_range_anchor.min(cursor);
                        let end = self.visual_range_anchor.max(cursor);
                        for _ in start.line..(end.line + 1) {
                            self.buffer.remove_line(start.line);
                        }

                        start.line = start.line.min(self.buffer.total_lines() - 1);
                        let line_len = self.buffer.line_len(start.line);
                        start.col = start.col.min(line_len);
                        state.cursor.from_linepos(start);
                        
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
                    state.cursor.y -= 1;
                    let max_x = (self.buffer.line_len(cursor.line - 1)).max(1);
                    if state.cursor.wanted_x > max_x {
                        state.cursor.x = max_x;
                    } else {
                        state.cursor.x = state.cursor.wanted_x;
                    }
                }
            },
            Object::Down => {
                if cursor.line < self.buffer.total_lines() - 1 {
                    state.cursor.y += 1;
                    let max_x = self.buffer.line_len(cursor.line + 1).max(1);
                    if state.cursor.wanted_x > max_x {
                        state.cursor.x = max_x;
                    } else {
                        state.cursor.x = state.cursor.wanted_x;
                    }
                }
            },
            Object::Left => {
                if cursor.col > 0 {
                    state.cursor.x -= 1;
                    state.cursor.wanted_x = state.cursor.x;
                }
            },
            Object::Right => {
                let line_len = self.buffer.line_len(cursor.line);
                if cursor.col + 1 < line_len {
                    state.cursor.x += 1;
                    state.cursor.wanted_x += 1;
                } else if self.mode == EditorMode::Visual && state.cursor.x == line_len {
                    // go one over like in vim to delete whole line + newline
                    state.cursor.x += 1;
                    state.cursor.wanted_x += 1;
                }
            },
            Object::Line => 'b: {
                if self.motion.action == Some(Action::Delete) {
                    self.buffer.remove_line(cursor.line);
                    if cursor.line == self.buffer.total_lines() && cursor.line > 0 {
                        state.cursor.y -= 1;
                    }
                    let line_len = self.buffer.line_len(state.cursor.y - 1);
                    if cursor.col >= line_len {
                        state.cursor.x = line_len.max(1);
                    }
                    break 'b
                }

                if self.motion.action == Some(Action::Goto) {
                    let line = if let Some(Modifier::Count(n)) = self.motion.modifier { n as usize } else { 1 };
                    let total_lines = self.buffer.total_lines();
                    let line = line.min(total_lines);
                    let line_len = self.buffer.line_len(line - 1);
                    state.cursor.y = line;
                    state.cursor.x = state.cursor.x.min(line_len);
                    break 'b
                }

                if self.motion.action == Some(Action::GOTO) {
                    if let Some(Modifier::Count(n)) = self.motion.modifier {
                        let line = n as usize;
                        let total_lines = self.buffer.total_lines();
                        let line = line.min(total_lines);
                        let line_len = self.buffer.line_len(line - 1);
                        state.cursor.y = line;
                        state.cursor.x = state.cursor.x.min(line_len);
                    } else {
                        let last_line = self.buffer.total_lines() - 1;
                        let line_len = self.buffer.line_len(last_line);
                        state.cursor.y = last_line + 1;
                        state.cursor.x = state.cursor.wanted_x.min(line_len);
                    }
                }
            },
            Object::LineStart => {
                if self.motion.action == Some(Action::Delete) {
                    self.buffer.remove_from_line(cursor.line, 0, cursor.col);
                }
                state.cursor.x = 1;
                state.cursor.wanted_x = 1;
            },
            Object::LineEnd => 'b: {
                if self.motion.action == Some(Action::Delete) {
                    let line_len = self.buffer.line_len(cursor.line);
                    self.buffer.remove_from_line(cursor.line, cursor.col, line_len - cursor.col);
                    if cursor.col > 0 {
                        state.cursor.x -= 1;
                        state.cursor.wanted_x = state.cursor.x;
                    }
                    break 'b
                }

                // go one over like in vim
                if self.mode == EditorMode::Visual {
                    state.cursor.x = (self.buffer.line_len(state.cursor.y as usize - 1) + 1).max(1);
                } else {
                    state.cursor.x = (self.buffer.line_len(state.cursor.y as usize - 1)).max(1);
                }
                state.cursor.wanted_x = state.cursor.x;
            },
            Object::CharUnderCursor => {
                let n = if let Some(Modifier::Count(n)) = self.motion.modifier { n } else { 1 };
                let line_len = self.buffer.line_len(cursor.line);
                if line_len > 0 {
                    self.buffer.remove_from_line(cursor.line, cursor.col, (n as usize).min(line_len - cursor.col));
                    if (state.cursor.x - 1) as usize >= (line_len - 1) && state.cursor.x > 1 {
                        state.cursor.x -= 1;
                        state.cursor.wanted_x = state.cursor.x;
                    }
                }
            },
        }

        true
    }
}

