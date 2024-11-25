use crate::{editor::EditorMode, gap_buffer::{LinePos, TextBuffer}};


#[derive(PartialEq, Clone, Copy)]
pub enum Action {
    Delete,
    Goto,
    GOTO,
}

#[derive(Clone, Copy)]
pub enum Object {
    BackWord,
    BackWORD,
    Word,
    WordEnd,
    WORD,
    WORDEnd,
    Append,
    Insert,
    NormalMode,
    VisualMode,
    VisualLineMode,
    CommandBarMode,
    SearchMode,
    VisualSelection,
    Up,
    Down,
    Left,
    Right,
    Line,
    LineStart,
    LineEnd,
    CharUnderCursor,
    NextSearchResult,
    PreviousSearchResult,
}

#[derive(PartialEq, Clone, Copy)]
pub enum Modifier {
    Around,
    Inside,
    FindForwards,
    FindBackwards,
    TillForwards,
    TillBackwards,
    Count(u32),
}

pub struct Motion {
    pub action: Option<Action>,
    pub object: Option<Object>,
    pub modifier: Option<Modifier>,
}

impl Motion {
    pub fn new() -> Self {
        Self { action: None, object: None, modifier: None }
    }

    pub fn clear(&mut self) {
        self.action = None;
        self.object = None;
        self.modifier = None;
    }

    pub fn parse(&mut self, char: char, current_mode: EditorMode) {
        match char {
            '$' => self.object = Some(Object::LineEnd),
            '1' ..= '9' => {
                if let Some(Modifier::Count(n)) = self.modifier {
                    self.modifier = Some(Modifier::Count(n * 10 + (char as u32 - '0' as u32)));
                } else {
                    self.modifier = Some(Modifier::Count(char as u32 - '0' as u32));
                }
            },
            '0' => {
                if let Some(Modifier::Count(n)) = self.modifier {
                    self.modifier = Some(Modifier::Count(n * 10));
                } else {
                    self.object = Some(Object::LineStart);
                }
            },
            'a' => {
                if current_mode == EditorMode::Visual {
                    self.modifier = Some(Modifier::Around);
                } else if self.action == Some(Action::Delete) {
                    self.modifier = Some(Modifier::Around);
                } else {
                    self.object = Some(Object::Append);
                }
            },
            'b' => self.object = Some(Object::BackWord),
            'd' => {
                if self.action == Some(Action::Delete) {
                    self.object = Some(Object::Line);
                } else {
                    self.action = Some(Action::Delete);
                    if current_mode == EditorMode::Visual || current_mode == EditorMode::VisualLine {
                        self.object = Some(Object::VisualSelection);
                    }
                }
            },
            'e' => self.object = Some(Object::WordEnd),
            'g' => {
                if self.action == Some(Action::Goto) {
                    self.object = Some(Object::Line);
                } else {
                    self.action = Some(Action::Goto);
                }
            },
            'G' => {
                self.object = Some(Object::Line);
                self.action = Some(Action::GOTO);
            },
            'h' => self.object = Some(Object::Left),
            'i' => {
                if current_mode == EditorMode::Visual {
                    self.modifier = Some(Modifier::Inside);
                } else if self.action == Some(Action::Delete) {
                    self.modifier = Some(Modifier::Inside);
                } else {
                    self.object = Some(Object::Insert);
                }
            },
            'j' => self.object = Some(Object::Down),
            'k' => self.object = Some(Object::Up),
            'l' => self.object = Some(Object::Right),
            'n' => self.object = Some(Object::NextSearchResult),
            'N' => self.object = Some(Object::PreviousSearchResult),
            'v' => {
                if current_mode == EditorMode::Visual {
                    self.object = Some(Object::NormalMode)
                }
                self.object = Some(Object::VisualMode);
            },
            'V' => {
                if current_mode == EditorMode::VisualLine {
                    self.object = Some(Object::NormalMode);
                }
                self.object = Some(Object::VisualLineMode);
            },
            'w' => self.object = Some(Object::Word),
            'W' => self.object = Some(Object::WORD),
            'x' => {
                if current_mode == EditorMode::Visual {
                    self.action = Some(Action::Delete)
                }
                self.action = Some(Action::Delete);
                self.object = Some(Object::CharUnderCursor);
            },
            ':' => self.object = Some(Object::CommandBarMode),
            '/' => self.object = Some(Object::SearchMode),
            _ => {},
        }
    }
}


type BufferCmd = fn(LinePos, &TextBuffer) -> Option<LinePos>;
pub fn count(cursor: LinePos, buf: &TextBuffer, count: u32, f: BufferCmd) -> Option<LinePos> {
    let mut last_pos = None;
    let mut cursor = cursor;
    for _ in 0..count {
        let Some(pos) = f(cursor, buf) else { return last_pos };
        cursor = pos;
        last_pos = Some(pos);
    }

    last_pos
}

pub fn find_next_word_start(cursor: LinePos, buf: &TextBuffer) -> Option<LinePos> {
    let mut iter = buf.utf8_iter(cursor);

    if let Some(c) = iter.next() {
        let mut line_add = 0;
        let mut col = cursor.col as isize;
        let mut found = false;

        if c.is_alphanumeric() || c == '_' {
            let mut found_whitespace = false;
            for char in iter {
                if char == '\n' { 
                    line_add += 1;
                    col = -1;
                    found_whitespace = true;
                    continue;
                }
                if !found_whitespace && char.is_whitespace() { found_whitespace = true; }

                col += 1;

                if !char.is_alphanumeric() && char != ' ' && char != '_' {
                    found = true;
                    break;
                }
                if found_whitespace && (char.is_alphanumeric() || char == '_') {
                    found = true;
                    break;
                }
            }
        } else {
            let mut found_whitespace = false;
            if c == '\n' { line_add += 1; found_whitespace = true }
            else if c.is_whitespace() { found_whitespace = true }
            for char in iter {
                if char == '\n' { 
                    line_add += 1;
                    col = -1;
                    found_whitespace = true;
                    continue;
                }
                if !found_whitespace && char.is_whitespace() { found_whitespace = true; }
                
                col += 1;

                if found_whitespace && !char.is_whitespace() || char.is_alphanumeric() {
                    found = true;
                    break;
                }
            }
        }

        if found {
            return Some(LinePos { line: cursor.line + line_add, col: col as usize })
        }
    }

    None
}


#[allow(non_snake_case)]
pub fn find_next_WORD_start(cursor: LinePos, buf: &TextBuffer) -> Option<LinePos> {
    let mut iter = buf.utf8_iter(cursor);

    if let Some(c) = iter.next() {
        let mut line_add = 0;
        let mut col = cursor.col as isize;
        let mut found = false;

        let mut found_whitespace = false;
        if c == '\n' { line_add += 1; found_whitespace = true; };
        for char in iter {
            if char == '\n' { 
                line_add += 1;
                col = -1;
                found_whitespace = true;
                continue;
            }
            if !found_whitespace && char.is_whitespace() { found_whitespace = true; }

            col += 1;

            if found_whitespace && !char.is_whitespace() {
                found = true;
                break;
            }
        }

        if found {
            return Some(LinePos { line: cursor.line + line_add, col: col as usize })
        }
    }

    None
}


pub fn find_current_word_start(cursor: LinePos, buf: &TextBuffer) -> Option<LinePos> {
    let iter = buf.utf8_rev_iter(cursor);

    let mut col = cursor.col;
    let mut looking_for_letter = false;
    let mut looking_for_whitespace = false;
    let mut looking_for_special = false;
    for char in iter {
        if char == '\r' || char == '\n' { 
            break; 
        }

        if col == cursor.col {
            if is_letter(char) {
                looking_for_whitespace = true;
                looking_for_special = true;
            } else if char.is_whitespace() {
                looking_for_letter = true;
                looking_for_special = true;
            } else {
                looking_for_letter = true;
                looking_for_whitespace = true;
            }
        } 

        if looking_for_letter && is_letter(char) {
            col += 1;
            break;
        }
        if looking_for_whitespace && char.is_whitespace() {
            col += 1;
            break;
        }
        if looking_for_special && is_special(char)  {
            col += 1;
            break;
        }

        if col == 0 { break; }
        col -= 1;
    }

    return Some(LinePos { line: cursor.line, col })
}


pub fn find_current_word_end(cursor: LinePos, buf: &TextBuffer) -> Option<LinePos> {
    let iter = buf.utf8_iter(cursor);

    let mut col = cursor.col;
    let mut looking_for_letter = false;
    let mut looking_for_whitespace = false;
    let mut looking_for_special = false;
    for char in iter {
        if char == '\r' || char == '\n' { 
            break; 
        }

        if col == cursor.col {
            if is_letter(char) {
                looking_for_whitespace = true;
                looking_for_special = true;
            } else if char.is_whitespace() {
                looking_for_letter = true;
                looking_for_special = true;
            } else {
                looking_for_letter = true;
                looking_for_whitespace = true;
            }
        } 

        if looking_for_letter && is_letter(char) {
            break;
        }
        if looking_for_whitespace && char.is_whitespace() {
            break;
        }
        if looking_for_special && is_special(char)  {
            break;
        }

        col += 1;
    }

    if col == 0 {
        return None
    }

    col -= 1;

    return Some(LinePos { line: cursor.line, col })
}


#[allow(non_snake_case)]
pub fn find_current_WORD_start(cursor: LinePos, buf: &TextBuffer) -> Option<LinePos> {
    let iter = buf.utf8_rev_iter(cursor);

    let mut col = cursor.col;
    let mut looking_for_whitespace = true;
    for char in iter {
        if char == '\r' || char == '\n' { 
            break; 
        }
        
        if col == cursor.col && char.is_whitespace() {
            looking_for_whitespace = false;
        }

        if looking_for_whitespace && char.is_whitespace() {
            col += 1;
            break;
        }
        if !looking_for_whitespace && !char.is_whitespace() {
            col += 1;
            break;
        }

        if col == 0 { break; }
        col -= 1;
    }

    return Some(LinePos { line: cursor.line, col })
}


#[allow(non_snake_case)]
pub fn find_current_WORD_end(cursor: LinePos, buf: &TextBuffer) -> Option<LinePos> {
    let iter = buf.utf8_iter(cursor);

    let mut col = cursor.col;
    let mut looking_for_whitespace = true;
    for char in iter {
        if char == '\r' || char == '\n' { 
            break; 
        }

        if col == cursor.col && char.is_whitespace() {
            looking_for_whitespace = false;
        }

        if looking_for_whitespace && char.is_whitespace() {
            break;
        }
        if !looking_for_whitespace && !char.is_whitespace() {
            break;
        }

        col += 1;
    }

    if col == 0 {
        return None
    }

    col -= 1;

    return Some(LinePos { line: cursor.line, col })
}


pub fn find_previous_word_start(cursor: LinePos, buf: &TextBuffer) -> Option<LinePos> {
    let mut line = cursor.line;
    let mut col = cursor.col;

    if col > 0 {
        col -= 1;
    } else {
        if line == 0 {
            return None
        }
        let line_len = buf.line_len(cursor.line - 1);
        line -= 1;
        col = line_len;
    }

    let mut iter = buf.utf8_rev_iter(LinePos { line, col });

    let mut looking_for_letter = false;
    let mut looking_for_special = false;
    let mut found = false;

    let Some(char) = iter.next() else { return None };
    if char.is_whitespace() {
        looking_for_letter = true;
        looking_for_special = true;
    } else if is_letter(char) {
        found = true;
        looking_for_letter = true;
    } else {
        found = true;
        looking_for_special = true;
    }

    for char in iter {
        if char == '\n' {
            if found { break; }
            line -= 1;
            col = buf.line_len(line);
            continue
        }
        if char == '\r' { continue }

        if !found && looking_for_letter && is_letter(char) {
            found = true;
            looking_for_special = false;
        }
        if !found && looking_for_special && is_special(char) {
            found = true;
            looking_for_letter = false;
        }

        if found && looking_for_letter && !is_letter(char) {
            break;
        }
        if found && looking_for_special && !is_special(char) {
            break;
        }

        col -= 1;
    }

    Some(LinePos{ line, col })
}


pub fn find_next_word_end(cursor: LinePos, buf: &TextBuffer) -> Option<LinePos> {
    let mut line = cursor.line;
    let mut col = cursor.col;

    let line_len = buf.line_len(line);
    if col + 1 < line_len {
        col += 1;
    } else {
        if line == buf.total_lines() - 1 {
            return None
        }
        line += 1;
        col = 0;
    }

    let mut iter = buf.utf8_iter(LinePos { line, col });

    let mut looking_for_letter = false;
    let mut looking_for_special = false;
    let mut found = false;

    let Some(char) = iter.next() else { return None };
    if char.is_whitespace() {
        looking_for_letter = true;
        looking_for_special = true;
    } else if is_letter(char) {
        found = true;
        looking_for_letter = true;
    } else {
        found = true;
        looking_for_special = true;
    }

    if char == '\n' {
        line += 1;
        col = 0;
    }

    for char in iter {
        if char == '\n' {
            if found { break; }
            line += 1;
            col = 0;
            continue
        }
        if char == '\r' { continue }

        if !found && looking_for_letter && is_letter(char) {
            found = true;
            looking_for_special = false;
        }
        if !found && looking_for_special && is_special(char) {
            found = true;
            looking_for_letter = false;
        }

        if found && looking_for_letter && !is_letter(char) {
            break;
        }
        if found && looking_for_special && !is_special(char) {
            break;
        }

        col += 1;
    }

    Some(LinePos{ line, col })
}


fn is_letter(char: char) -> bool {
    char.is_alphanumeric() || char == '_'
}

fn is_special(char: char) -> bool {
    !(char.is_whitespace() || char.is_alphanumeric() || (char == '_'))
}
