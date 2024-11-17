use std::{fs, io::{self, Read}, path::{Path, PathBuf}};

use crate::{gap_buffer::{LinePos, TextBuffer}, vim_commands::*, SpecialKey, State};

#[derive(PartialEq, Eq)]
pub enum EditorMode {
    Insert,
    Normal,
    Visual,
}

pub struct Editor {
    pub buffer: TextBuffer,
    pub file_path: PathBuf,
    pub mode: EditorMode,
    pub normal_commands: Vec<NormalCmd>,
    pub visual_commands: Vec<VisualCmd>,
    pub visual_range_anchor: LinePos,
    pub visual_range_moving: LinePos,
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
                normal_commands: Vec::new(),
                visual_commands: Vec::new(),
                visual_range_anchor: LinePos { line: 0, col: 0 },
                visual_range_moving: LinePos { line: 0, col: 0 },
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
        } else if self.mode == EditorMode::Normal {
            for char in state.io.chars.chars() {
                let Some(cmd) = NormalCmd::from_char(&self.normal_commands, char) else { continue; };
                self.normal_commands.push(cmd);
            }
            self.execute_normal_commands(state);
        } else if self.mode == EditorMode::Visual {
            for char in state.io.chars.chars() {
                let Some(cmd) = VisualCmd::from_char(&self.visual_commands, char) else { continue; };
                self.visual_commands.push(cmd);
            }
            self.execute_visual_commands(state);
            if state.io.pressed_special(SpecialKey::Escape) {
                self.visual_commands.clear();
                self.mode = EditorMode::Normal;
            }
        }
    }

    fn execute_normal_commands(&mut self, state: &mut State) {
        let mut executed = 0;
        let line = state.cursor.y as usize - 1;

        //for cmd in &self.normal_commands {
        for i in 0..self.normal_commands.len() {
            let cmd = &self.normal_commands[i];
            match cmd {
                NormalCmd::Append => {
                    self.mode = EditorMode::Insert;
                    let row_len = self.buffer.line_len(line);
                    if row_len > 0 {
                        state.cursor.x += 1;
                    }
                    executed += 1;
                },
                NormalCmd::Down => {
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
                NormalCmd::Insert => {
                    self.mode = EditorMode::Insert;
                    executed += 1;
                },
                NormalCmd::Left => {
                    if state.cursor.x > 1 {
                        state.cursor.x -= 1;
                        state.cursor.wanted_x -= 1;
                    }
                    executed += 1;
                },
                NormalCmd::LineEnd => {
                    state.cursor.x = (self.buffer.line_len(state.cursor.y as usize - 1)).max(1);
                    state.cursor.wanted_x = state.cursor.x;
                    executed += 1;
                },
                NormalCmd::LineStart => {
                    state.cursor.x = 1;
                    state.cursor.wanted_x = 1;
                    executed += 1;
                },
                NormalCmd::Right => {
                    let line_len = self.buffer.line_len(state.cursor.y as usize - 1);
                    if state.cursor.x < state.max_cols() && state.cursor.x < line_len {
                        state.cursor.x += 1;
                        state.cursor.wanted_x += 1;
                    }
                    executed += 1;
                },
                NormalCmd::Up => {
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
                NormalCmd::Word => {
                    let previous_cmd = if i > 0 {
                        self.normal_commands.get(i - 1)
                    } else {
                        None
                    };
                    match previous_cmd {
                        None => {
                            let pos = find_next_word_start(state, &self.buffer);
                            let Some(pos) = pos else { executed += 1; continue; };
                            state.cursor.x = pos.col + 1;
                            state.cursor.y = pos.line + 1;
                            state.cursor.wanted_x = state.cursor.x;
                        },
                        Some(NormalCmd::Inside) => {
                            let Some(start) = find_current_word_start(&state, &self.buffer) else { executed += 1; continue };
                            let Some(end) = find_current_word_end(&state, &self.buffer) else { executed += 1; continue };
                            let previous_cmd = if i > 1 {
                                self.normal_commands.get(i - 2)
                            } else {
                                None
                            };
                            match previous_cmd {
                                None => executed += 1,
                                Some(NormalCmd::Delete) => {
                                    let line = state.cursor.y - 1;
                                    self.buffer.remove_from_line(line, start.col, end.col - start.col + 1);
                                    state.cursor.x = ((start.col + 1).min(self.buffer.line_len(line))).max(1);
                                    state.cursor.wanted_x = state.cursor.x;
                                    executed += 2;
                                }
                                _ => {},
                            }

                        },
                        Some(NormalCmd::Delete) => {
                            let pos = find_next_word_start(state, &self.buffer);
                            let Some(pos) = pos else { executed += 1; continue; };
                            if pos.line + 1 == state.cursor.y {
                                self.buffer.remove_from_line(state.cursor.y - 1, state.cursor.x - 1, pos.col - state.cursor.x);
                            }
                            executed += 1;
                        },
                        _ => {},
                    }
                    executed += 1;
                },
                NormalCmd::WORD => {
                    let previous_cmd = if i > 0 {
                        self.normal_commands.get(i - 1)
                    } else {
                        None
                    };
                    match previous_cmd {
                        None => {
                            let pos = find_next_WORD_start(state, &self.buffer);
                            let Some(pos) = pos else { executed += 1; continue; };
                            state.cursor.x = pos.col + 1;
                            state.cursor.y = pos.line + 1;
                            state.cursor.wanted_x = state.cursor.x;
                        },
                        Some(NormalCmd::Inside) => {
                            let Some(start) = find_current_WORD_start(&state, &self.buffer) else { executed += 1; continue };
                            let Some(end) = find_current_WORD_end(&state, &self.buffer) else { executed += 1; continue };
                            let previous_cmd = if i > 1 {
                                self.normal_commands.get(i - 2)
                            } else {
                                None
                            };
                            match previous_cmd {
                                None => executed += 1,
                                Some(NormalCmd::Delete) => {
                                    let line = state.cursor.y - 1;
                                    self.buffer.remove_from_line(line, start.col, end.col - start.col + 1);
                                    state.cursor.x = ((start.col + 1).min(self.buffer.line_len(line))).max(1);
                                    state.cursor.wanted_x = state.cursor.x;
                                    executed += 2;
                                }
                                _ => {},
                            }

                        },
                        Some(NormalCmd::Delete) => {
                            let pos = find_next_WORD_start(state, &self.buffer);
                            let Some(pos) = pos else { executed += 1; continue; };
                            if pos.line + 1 == state.cursor.y {
                                self.buffer.remove_from_line(state.cursor.y - 1, state.cursor.x - 1, pos.col - state.cursor.x);
                            }
                            executed += 1;
                        },
                        _ => {},
                    }
                    executed += 1;
                },
                NormalCmd::BackWord => {
                    let previous_cmd = if i > 0 {
                        self.normal_commands.get(i - 1)
                    } else {
                        None
                    };
                    let pos = find_previous_word_start(state, &self.buffer);
                    let Some(pos) = pos else { executed += 1; continue; };
                    match previous_cmd {
                        None => {
                            state.cursor.y = pos.line + 1;
                            state.cursor.x = pos.col + 1;
                            state.cursor.wanted_x = state.cursor.x;
                        },
                        Some(NormalCmd::Delete) => {

                        },
                        _ => {},
                    }
                    executed += 1;
                },
                NormalCmd::Xdel => {
                    let row_len = self.buffer.line_len(line);
                    if row_len > 0 {
                        self.buffer.remove_from_line(line, state.cursor.x as usize - 1, 1);
                        if (state.cursor.x - 1) as usize >= (row_len - 1) && state.cursor.x > 1 {
                            state.cursor.x -= 1;
                            state.cursor.wanted_x = state.cursor.x;
                        }
                    }
                    executed += 1;
                },
                NormalCmd::Delete => {
                    if i == 0 { continue }
                    let previous_cmd = self.normal_commands.get(i-1);
                    match previous_cmd {
                        Some(NormalCmd::Delete) => {
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
                NormalCmd::Visual => {
                    self.mode = EditorMode::Visual;
                    let cursor = state.cursor.to_linepos();
                    self.visual_range_anchor = cursor;
                    self.visual_range_moving = cursor;

                    executed += 1;
                },
                NormalCmd::Inside => continue,
                _ => todo!("{:?}", cmd),
            }
        }

        self.normal_commands.drain(0..executed);
    }

    fn execute_visual_commands(&mut self, state: &mut State) {
        let mut executed = 0;
        let line = state.cursor.y as usize - 1;

        for i in 0..self.visual_commands.len() {
            let cmd = &self.visual_commands[i];
            match cmd {
                VisualCmd::Down => {
                    if state.cursor.y < self.buffer.total_lines() {
                        state.cursor.y += 1;
                    }
                    let max_x = self.buffer.line_len(state.cursor.y as usize - 1).max(1);
                    if state.cursor.wanted_x > max_x {
                        state.cursor.x = max_x;
                    } else {
                        state.cursor.x = state.cursor.wanted_x;
                    }
                    let cursor = state.cursor.to_linepos();
                    self.visual_range_moving = cursor;
                    executed += 1;
                },
                VisualCmd::Left => {
                    if state.cursor.x > 1 {
                        state.cursor.x -= 1;
                        state.cursor.wanted_x -= 1;
                    }
                    let cursor = state.cursor.to_linepos();
                    self.visual_range_moving = cursor;
                    executed += 1;
                },
                VisualCmd::LineEnd => {
                    // go one over line, as in vim
                    state.cursor.x = (self.buffer.line_len(state.cursor.y as usize - 1) + 1).max(1);
                    state.cursor.wanted_x = state.cursor.x;
                    let cursor = state.cursor.to_linepos();
                    self.visual_range_moving = cursor;
                    executed += 1;
                },
                VisualCmd::LineStart => {
                    state.cursor.x = 1;
                    state.cursor.wanted_x = 1;
                    let cursor = state.cursor.to_linepos();
                    self.visual_range_moving = cursor;
                    executed += 1;
                },
                VisualCmd::Right => {
                    let line_len = self.buffer.line_len(state.cursor.y as usize - 1);
                    // allow to go one over the line, as in vim
                    if state.cursor.x <= line_len {
                        state.cursor.x += 1;
                        state.cursor.wanted_x += 1;
                    }
                    let cursor = state.cursor.to_linepos();
                    self.visual_range_moving = cursor;
                    executed += 1;
                },
                VisualCmd::Up => {
                    if state.cursor.y > 1 {
                        state.cursor.y -= 1;
                    }
                    let max_x = (self.buffer.line_len(state.cursor.y as usize - 1)).max(1);
                    if state.cursor.wanted_x > max_x {
                        state.cursor.x = max_x;
                    } else {
                        state.cursor.x = state.cursor.wanted_x;
                    }
                    let cursor = state.cursor.to_linepos();
                    self.visual_range_moving = cursor;
                    executed += 1;
                },
                VisualCmd::Word => {
                    let previous_cmd = if i > 0 {
                        self.visual_commands.get(i - 1)
                    } else {
                        None
                    };
                    match previous_cmd {
                        None => {
                            let pos = find_next_word_start(state, &self.buffer);
                            let Some(pos) = pos else { executed += 1; continue; };
                            state.cursor.x = pos.col + 1;
                            state.cursor.y = pos.line + 1;
                            state.cursor.wanted_x = state.cursor.x;

                            let cursor = state.cursor.to_linepos();
                            self.visual_range_moving = cursor;
                        },
                        Some(VisualCmd::Inside) => {
                            let Some(start) = find_current_word_start(&state, &self.buffer) else { executed += 1; continue };
                            self.visual_range_anchor = start;
                            let Some(end) = find_current_word_end(&state, &self.buffer) else { executed += 1; continue };
                            self.visual_range_moving = end;

                            state.cursor.x = end.col + 1;
                            state.cursor.y = end.line + 1;
                            state.cursor.wanted_x = state.cursor.x;

                            executed += 1;
                        },
                        _ => {},
                    }
                    executed += 1;
                },
                VisualCmd::WORD => {
                    let previous_cmd = if i > 0 {
                        self.visual_commands.get(i - 1)
                    } else {
                        None
                    };
                    match previous_cmd {
                        None => {
                            let pos = find_next_WORD_start(state, &self.buffer);
                            let Some(pos) = pos else { executed += 1; continue; };
                            state.cursor.x = pos.col + 1;
                            state.cursor.y = pos.line + 1;
                            state.cursor.wanted_x = state.cursor.x;

                            let cursor = state.cursor.to_linepos();
                            self.visual_range_moving = cursor;
                        },
                        Some(VisualCmd::Inside) => {
                            let Some(start) = find_current_WORD_start(&state, &self.buffer) else { executed += 1; continue };
                            self.visual_range_anchor = start;
                            let Some(end) = find_current_WORD_end(&state, &self.buffer) else { executed += 1; continue };
                            self.visual_range_anchor = end;

                            executed += 1;
                        },
                        _ => {},
                    }
                    executed += 1;
                },
                VisualCmd::BackWord => {
                    let pos = find_previous_word_start(state, &self.buffer);
                    let Some(pos) = pos else { executed += 1; continue; };
                    state.cursor.y = pos.line + 1;
                    state.cursor.x = pos.col + 1;
                    state.cursor.wanted_x = state.cursor.x;

                    let cursor = state.cursor.to_linepos();
                    self.visual_range_moving = cursor;
                    executed += 1;
                },
                VisualCmd::Delete => {
                    let min = self.visual_range_anchor.min(self.visual_range_moving);
                    let max = self.visual_range_anchor.max(self.visual_range_moving);
                    self.buffer.remove_by_range(min, max);
                    self.mode = EditorMode::Normal;

                    state.cursor.x = min.col + 1;
                    state.cursor.wanted_x = state.cursor.x;
                    state.cursor.y = min.line + 1;

                    executed += 1;
                },
                _ => {},
            }
        }

        self.visual_commands.drain(0..executed);
    }
}
