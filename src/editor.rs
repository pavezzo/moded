use std::{fs, io::{self, Read}, path::{Path, PathBuf}};

use crate::{gap_buffer::TextBuffer, vim_commands::NormalCmd, SpecialKey, State};

#[derive(PartialEq, Eq)]
pub enum EditorMode {
    Insert,
    Normal,
}

pub struct Editor {
    pub buffer: TextBuffer,
    pub file_path: PathBuf,
    pub mode: EditorMode,
    pub normal_commands: Vec<NormalCmd>,
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
            }
        }
        todo!();
    }

    pub fn handle_input(&mut self, state: &mut State) {
        if self.mode == EditorMode::Insert {
            let line = state.cursor.y as usize - 1;
            if !state.io.chars.is_empty() {
                self.buffer.insert_into_line(line, state.cursor.x as usize - 1, state.io.chars.as_bytes());
                state.cursor.x += state.io.chars.chars().count() as u32;
            }
            if state.io.pressed_special(SpecialKey::Enter) {
                let line_len = self.buffer.line_len(line) - 1;
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
                if row_len > 1 && state.cursor.x > 1 && state.cursor.x as usize <= row_len {
                    self.buffer.remove_from_line(line, state.cursor.x as usize - 2, 1);
                    state.cursor.x -= 1;
                    state.cursor.wanted_x = state.cursor.x;
                } else if row_len == 1 && state.cursor.x == 1 {
                    self.buffer.remove_line(line);
                    if state.cursor.y > 1 {
                        let len = self.buffer.line_len(line - 1);
                        state.cursor.y -= 1;
                        state.cursor.x = (len as u32 - 1).max(1);
                        state.cursor.wanted_x = state.cursor.x;
                    }
                } else if state.cursor.x == 1 && state.cursor.y > 1 {
                    //self.buffer.remove_from_end_and_merge(line - 1, 1);
                    //self.buffer.remove_line_index_len(line - 1, self.buffer.line_len(line - 1), 1);
                    let next_cursor_pos = self.buffer.line_len(line - 1);
                    self.buffer.remove_from_line(line - 1, self.buffer.line_len(line - 1) - 1, 1);
                    state.cursor.x = next_cursor_pos as u32;
                    state.cursor.wanted_x = state.cursor.x;
                    state.cursor.y -= 1;
                }
            }
        } else if self.mode == EditorMode::Normal {
            for char in state.io.chars.chars() {
                let Some(cmd) = NormalCmd::from_char(char) else { continue; };
                self.normal_commands.push(cmd);
            }
            self.execute_normal_commands(state);
        }
    }

    fn execute_normal_commands(&mut self, state: &mut State) {
        let mut executed = 0;
        let line = state.cursor.y as usize - 1;

        for cmd in &self.normal_commands {
            match cmd {
                NormalCmd::Append => {
                    self.mode = EditorMode::Insert;
                    let row_len = self.buffer.line_len(line);
                    if row_len > 1 {
                        state.cursor.x += 1;
                    }
                    executed += 1;
                },
                NormalCmd::Down => {
                    if state.cursor.y < self.buffer.total_lines() as u32 {
                        state.cursor.y += 1;
                    }
                    let max_x = (self.buffer.line_len(state.cursor.y as usize - 1) as u32 - 1).max(1);
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
                    state.cursor.x = (self.buffer.line_len(state.cursor.y as usize - 1) as u32 - 1).max(1);
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
                    if state.cursor.x < state.max_cols() && state.cursor.x < line_len as u32 - 1 {
                        state.cursor.x += 1;
                        state.cursor.wanted_x += 1;
                    }
                    executed += 1;
                },
                NormalCmd::Up => {
                    if state.cursor.y > 1 {
                        state.cursor.y -= 1;
                    }
                    let max_x = (self.buffer.line_len(state.cursor.y as usize - 1) as u32 - 1).max(1);
                    if state.cursor.wanted_x > max_x {
                        state.cursor.x = max_x;
                    } else {
                        state.cursor.x = state.cursor.wanted_x;
                    }
                    executed += 1;
                },
                NormalCmd::Word => {
                    let line = self.buffer.line(state.cursor.y as usize - 1);
                    let mut pos = None;
                    for (i, char) in line.chars().skip(state.cursor.x as usize - 1).enumerate() {
                        if char == ' ' {
                            pos = Some(i);
                            break;
                        }
                    }
                    //if let Some(pos) = line.chars().skip(state.cursor.x as usize - 1).find(|x| *x == ' ') {
                    //if let Some(pos) = line[(state.cursor.x as usize - 1)..line.len()].find(' ') {
                    if let Some(pos) = pos {
                        state.cursor.x += pos as u32 + 1;
                        state.cursor.wanted_x = state.cursor.x;
                    }
                    executed += 1;
                },
                NormalCmd::Xdel => {
                    let row_len = self.buffer.line_len(line);
                    if row_len > 1 {
                        self.buffer.remove_from_line(line, state.cursor.x as usize - 1, 1);
                        if (state.cursor.x - 1) as usize >= (row_len - 2) && state.cursor.x > 1 {
                            state.cursor.x -= 1;
                            state.cursor.wanted_x = state.cursor.x;
                        }
                    }
                    executed += 1;
                },
            }
        }

        self.normal_commands.drain(0..executed);
    }
}
