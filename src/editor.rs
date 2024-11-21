use std::{fs, io::{self, Read}, path::{Path, PathBuf}};

use glfw::CursorMode;

use crate::{gap_buffer::{LinePos, TextBuffer}, vim_commands::*, SpecialKey, State};

#[derive(PartialEq, Eq, Clone, Copy)]
pub enum EditorMode {
    Insert,
    Normal,
    Visual,
}

pub struct Editor {
    pub buffer: TextBuffer,
    pub file_path: PathBuf,
    pub mode: EditorMode,
    pub motion_commands: Vec<MotionCmd>,
    pub visual_range_anchor: LinePos,
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
                motion_commands: Vec::new(),
                visual_range_anchor: LinePos { line: 0, col: 0 },
            }
        }
        todo!();
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
        } else {
            for char in state.io.chars.chars() {
                let Some(cmd) = MotionCmd::from_char(&mut self.motion_commands, char, self.mode) else { continue; };
                self.motion_commands.push(cmd);
            }
            self.execute_commands(state);
            if state.io.pressed_special(SpecialKey::Escape) {
                self.motion_commands.clear();
                self.mode = EditorMode::Normal;
            }
        } 
    }

    fn execute_commands(&mut self, state: &mut State) {
        let mut executed = 0;
        let line = state.cursor.y as usize - 1;

        for i in 0..self.motion_commands.len() {
            macro_rules! previous_cmd {
                () => {
                    if i > 0 {
                        Some(&self.motion_commands[i - 1])
                    } else {
                        None
                    }
                };
            }
            macro_rules! two_previous_cmds {
                () => {
                    if i > 1 {
                        (Some(&self.motion_commands[i - 2]), Some(&self.motion_commands[i - 1]))
                    } else if i > 0 {
                        (None, Some(&self.motion_commands[i - 1]))
                    } else {
                        (None, None)
                    }
                };
            }

            let cmd = &self.motion_commands[i];
            match cmd {
                MotionCmd::Append => {
                    self.mode = EditorMode::Insert;
                    let row_len = self.buffer.line_len(line);
                    if row_len > 0 {
                        state.cursor.x += 1;
                    }
                    executed += 1;
                },
                MotionCmd::Down => {
                    if state.cursor.y < self.buffer.total_lines() {
                        state.cursor.y += 1;
                    }
                    let max_x = self.buffer.line_len(state.cursor.y as usize - 1).max(1);
                    if state.cursor.wanted_x > max_x {
                        state.cursor.x = max_x;
                    } else {
                        state.cursor.x = state.cursor.wanted_x;
                    }
                    executed += 1;
                },
                MotionCmd::Insert => {
                    self.mode = EditorMode::Insert;
                    executed += 1;
                },
                MotionCmd::Left => {
                    if state.cursor.x > 1 {
                        state.cursor.x -= 1;
                        state.cursor.wanted_x -= 1;
                    }
                    executed += 1;
                },
                MotionCmd::LineEnd => {
                    let prev = previous_cmd!();
                    if prev == Some(&MotionCmd::Delete) {
                        let cursor = state.cursor.to_linepos();
                        let line_len = self.buffer.line_len(cursor.line);
                        self.buffer.remove_from_line(cursor.line, cursor.col, line_len - cursor.col);
                        if cursor.col > 0 {
                            state.cursor.x -= 1;
                            state.cursor.wanted_x = state.cursor.x;
                        }
                        executed += 2;
                        continue
                    }
                    // go one over like in vim
                    if self.mode == EditorMode::Visual {
                        state.cursor.x = (self.buffer.line_len(state.cursor.y as usize - 1) + 1).max(1);
                    } else {
                        state.cursor.x = (self.buffer.line_len(state.cursor.y as usize - 1)).max(1);
                    }
                    state.cursor.wanted_x = state.cursor.x;
                    executed += 1;
                },
                MotionCmd::LineStart => {
                    let prev = previous_cmd!();
                    if prev == Some(&MotionCmd::Delete) {
                        let cursor = state.cursor.to_linepos();
                        self.buffer.remove_from_line(cursor.line, 0, cursor.col);
                        executed += 1;
                    }
                    state.cursor.x = 1;
                    state.cursor.wanted_x = 1;
                    executed += 1;
                },
                MotionCmd::Right => {
                    let line_len = self.buffer.line_len(state.cursor.y as usize - 1);
                    if state.cursor.x < line_len {
                        state.cursor.x += 1;
                        state.cursor.wanted_x += 1;
                    } else if self.mode == EditorMode::Visual && state.cursor.x == line_len {
                        // go one over like in vim to delete whole line + newline
                        state.cursor.x += 1;
                        state.cursor.wanted_x += 1;
                    }
                    executed += 1;
                },
                MotionCmd::Up => {
                    if state.cursor.y > 1 {
                        state.cursor.y -= 1;
                    }
                    let max_x = (self.buffer.line_len(state.cursor.y as usize - 1)).max(1);
                    if state.cursor.wanted_x > max_x {
                        state.cursor.x = max_x;
                    } else {
                        state.cursor.x = state.cursor.wanted_x;
                    }
                    executed += 1;
                },
                MotionCmd::Word => {
                    let (prev1, prev2) = two_previous_cmds!();
                    match prev2 {
                        None => {
                            let cursor = state.cursor.to_linepos();
                            let Some(pos) = find_next_word_start(cursor, &self.buffer) else { executed += 1; continue };
                            state.cursor.from_linepos(pos);
                            executed += 1;
                        },
                        Some(MotionCmd::Count(n)) => {
                            let cursor = state.cursor.to_linepos();
                            let Some(mut pos) = count(cursor, &self.buffer, *n, find_next_word_start) else { executed += 2; continue };
                            match prev1 {
                                Some(MotionCmd::Delete) => {
                                    if pos.col > 0 {
                                        pos.col -= 1;
                                    } else {
                                        pos.line -= 1;
                                        pos.col = self.buffer.line_len(pos.line);
                                    }
                                    self.buffer.remove_by_range(cursor, pos);
                                    executed += 3;
                                },
                                None => {
                                    state.cursor.from_linepos(pos);
                                    executed += 2;
                                }
                                _ => {},
                            }
                        }
                        Some(MotionCmd::Inside) => {
                            let cursor = state.cursor.to_linepos();
                            let Some(start) = find_current_word_start(cursor, &self.buffer) else { executed += 1; continue };
                            let Some(end) = find_current_word_end(cursor, &self.buffer) else { executed += 1; continue };
                            if self.mode == EditorMode::Visual {
                                self.visual_range_anchor = start;
                                state.cursor.from_linepos(end);
                                executed += 2;
                                continue
                            }

                            match prev1 {
                                None => executed += 2,
                                Some(MotionCmd::Delete) => {
                                    let line = state.cursor.y - 1;
                                    self.buffer.remove_from_line(line, start.col, end.col - start.col + 1);
                                    state.cursor.x = ((start.col + 1).min(self.buffer.line_len(line))).max(1);
                                    state.cursor.wanted_x = state.cursor.x;
                                    executed += 3;
                                }
                                _ => {},
                            }

                        },
                        Some(MotionCmd::Delete) => {
                            let cursor = state.cursor.to_linepos();
                            match prev1 {
                                None => {
                                    let Some(pos) = find_next_word_start(cursor, &self.buffer) else { executed += 1; continue; };
                                    if pos.line + 1 == state.cursor.y {
                                        self.buffer.remove_from_line(state.cursor.y - 1, state.cursor.x - 1, pos.col - state.cursor.x);
                                    }
                                    executed += 2;
                                },
                                Some(MotionCmd::Count(n)) => {
                                    let Some(mut pos) = count(cursor, &self.buffer, *n, find_next_word_start) else { executed += 2; continue };
                                    if pos.col > 0 {
                                        pos.col -= 1;
                                    } else {
                                        pos.line -= 1;
                                        pos.col = self.buffer.line_len(pos.line);
                                    }
                                    self.buffer.remove_by_range(cursor, pos);
                                    executed += 3;
                                },
                                _ => {},
                            }
                        },
                        _ => {},
                    }
                },
                MotionCmd::WORD => {
                    let (prev1, prev2) = two_previous_cmds!();
                    match prev2 {
                        None => {
                            let cursor = state.cursor.to_linepos();
                            let Some(pos) = find_next_WORD_start(cursor, &self.buffer) else { executed += 1; continue };
                            state.cursor.from_linepos(pos);
                        },
                        Some(MotionCmd::Inside) => {
                            let cursor = state.cursor.to_linepos();
                            let Some(start) = find_current_WORD_start(cursor, &self.buffer) else { executed += 1; continue };
                            let Some(end) = find_current_WORD_end(cursor, &self.buffer) else { executed += 1; continue };
                            if self.mode == EditorMode::Visual {
                                self.visual_range_anchor = start;
                                state.cursor.from_linepos(end);
                                executed += 2;
                                continue
                            }

                            match prev1 {
                                None => executed += 1,
                                Some(MotionCmd::Delete) => {
                                    let line = state.cursor.y - 1;
                                    self.buffer.remove_from_line(line, start.col, end.col - start.col + 1);
                                    state.cursor.x = ((start.col + 1).min(self.buffer.line_len(line))).max(1);
                                    state.cursor.wanted_x = state.cursor.x;
                                    executed += 2;
                                }
                                _ => {},
                            }

                        },
                        Some(MotionCmd::Delete) => {
                            let cursor = state.cursor.to_linepos();
                            let Some(pos) = find_next_WORD_start(cursor, &self.buffer) else { executed += 1; continue };
                            if pos.line + 1 == state.cursor.y {
                                self.buffer.remove_from_line(state.cursor.y - 1, state.cursor.x - 1, pos.col - state.cursor.x);
                            }
                            executed += 1;
                        },
                        _ => {},
                    }
                    executed += 1;
                },
                MotionCmd::BackWord => {
                    let prev = previous_cmd!();
                    let cursor = state.cursor.to_linepos();
                    let Some(pos) = find_previous_word_start(cursor, &self.buffer) else { executed += 1; continue };
                    match prev {
                        None => {
                            state.cursor.from_linepos(pos);
                        },
                        Some(MotionCmd::Delete) => {

                        },
                        _ => {},
                    }
                    executed += 1;
                },
                MotionCmd::Xdel => {
                    let prev = previous_cmd!();
                    let n = if let Some(MotionCmd::Count(n)) = prev { *n } else { 1 };
                    let line_len = self.buffer.line_len(line);
                    let cursor = state.cursor.to_linepos();
                    if line_len > 0 {
                        self.buffer.remove_from_line(line, cursor.col, (n as usize).min(line_len - cursor.col));
                        if (state.cursor.x - 1) as usize >= (line_len - 1) && state.cursor.x > 1 {
                            state.cursor.x -= 1;
                            state.cursor.wanted_x = state.cursor.x;
                        }
                    }
                    executed += 1;
                },
                MotionCmd::Delete => {
                    if self.mode == EditorMode::Visual {
                        let cursor = state.cursor.to_linepos();
                        let min = self.visual_range_anchor.min(cursor);
                        let max = self.visual_range_anchor.max(cursor);
                        self.buffer.remove_by_range(min, max);
                        self.mode = EditorMode::Normal;

                        state.cursor.from_linepos(min);

                        executed += 1;
                        continue
                    }

                    let prev = previous_cmd!();
                    match prev {
                        Some(MotionCmd::Delete) => {
                            self.buffer.remove_line(state.cursor.y as usize - 1);
                            if state.cursor.y as usize > self.buffer.total_lines() && state.cursor.y > 1 {
                                state.cursor.y -= 1;
                            }
                            let line_len = self.buffer.line_len(state.cursor.y - 1);
                            if state.cursor.x as usize > line_len {
                                state.cursor.x = line_len.max(1);
                            }
                            executed += 2;
                        },
                        None => continue,
                        _ => continue,
                    }
                },
                MotionCmd::VisualMode => {
                    self.mode = EditorMode::Visual;
                    let cursor = state.cursor.to_linepos();
                    self.visual_range_anchor = cursor;

                    executed += 1;
                },
                MotionCmd::Inside => continue,
                MotionCmd::Count(_) => continue,
                _ => todo!("{:?}", cmd),
            }
        }

        self.motion_commands.drain(0..executed);
    }
}

