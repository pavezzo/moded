use crate::{gap_buffer::{LinePos, TextBuffer}, State};



#[repr(u8)]
#[derive(PartialEq, Eq, Debug)]
pub enum NormalCmd {
    Append,
    Around,
    Inside,
    Delete,
    Down,
    Insert, 
    Left,
    LineEnd,
    LineStart,
    Right,
    Up,
    Word,
    BackWord,
    WORD,
    Xdel,
    Visual,
    VisualLine,
}


impl NormalCmd {
    pub fn from_char(previous: &[NormalCmd], ch: char) -> Option<Self> {
        match ch {
            '$' => Some(NormalCmd::LineEnd),
            '0' => Some(NormalCmd::LineStart),
            'a' => {
                match previous.last() {
                    Some(NormalCmd::Delete) => Some(NormalCmd::Around),
                    None => Some(NormalCmd::Append),
                    _ => None,
                }
                
            },
            'b' => Some(NormalCmd::BackWord),
            'd' => Some(NormalCmd::Delete),
            'h' => Some(NormalCmd::Left),
            'i' => {
                match previous.last() {
                    Some(NormalCmd::Delete) => Some(NormalCmd::Inside),
                    None => Some(NormalCmd::Insert),
                    _ => None,
                }
            },
            'j' => Some(NormalCmd::Down),
            'k' => Some(NormalCmd::Up),
            'l' => Some(NormalCmd::Right),
            'v' => Some(NormalCmd::Visual),
            'V' => Some(NormalCmd::VisualLine),
            'w' => Some(NormalCmd::Word),
            'W' => Some(NormalCmd::WORD),
            'x' => Some(NormalCmd::Xdel),
            _ => None,
        }
    }
}

#[repr(u8)]
#[derive(PartialEq, Eq, Debug)]
pub enum VisualCmd {
    Around,
    BackWord,
    Delete,
    Down,
    Inside,
    Left,
    LineEnd,
    LineStart,
    NormalMode,
    Right,
    Up,
    WORD,
    Word,
}

impl VisualCmd {
    pub fn from_char(_previous: &[VisualCmd], ch: char) -> Option<VisualCmd> {
        match ch {
            '$' => Some(VisualCmd::LineEnd),
            '0' => Some(VisualCmd::LineStart),
            'a' => Some(VisualCmd::Around),
            'b' => Some(VisualCmd::BackWord),
            'd' => Some(VisualCmd::Delete),
            'h' => Some(VisualCmd::Left),
            'i' => Some(VisualCmd::Inside),
            'j' => Some(VisualCmd::Down),
            'k' => Some(VisualCmd::Up),
            'l' => Some(VisualCmd::Right),
            'v' => Some(VisualCmd::NormalMode),
            'w' => Some(VisualCmd::Word),
            'W' => Some(VisualCmd::WORD),
            'x' => Some(VisualCmd::Delete),
            _ => None,
        }
    }
}


pub fn find_next_word_start(state: &State, buf: &TextBuffer) -> Option<LinePos> {
    let mut pos = LinePos { line: state.cursor.y - 1, col: state.cursor.x - 1 };
    let mut iter = buf.utf8_iter(pos);

    if let Some(c) = iter.next() {
        let mut line_add = 0;
        let mut col = pos.col as isize;
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
            return Some(LinePos { line: state.cursor.y - 1 + line_add, col: col as usize })
        }
    }

    None
}


#[allow(non_snake_case)]
pub fn find_next_WORD_start(state: &State, buf: &TextBuffer) -> Option<LinePos> {
    let pos = LinePos { line: state.cursor.y - 1, col: state.cursor.x - 1 };
    let mut iter = buf.utf8_iter(pos);

    if let Some(c) = iter.next() {
        let mut line_add = 0;
        let mut col = pos.col as isize;
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
            return Some(LinePos { line: state.cursor.y - 1 + line_add, col: col as usize })
        }
    }

    None
}


pub fn find_current_word_start(state: &State, buf: &TextBuffer) -> Option<LinePos> {
    let pos = LinePos { line: state.cursor.y - 1, col: state.cursor.x - 1 };
    let iter = buf.utf8_rev_iter(pos);

    let mut col = pos.col;
    let mut looking_for_letter = false;
    let mut looking_for_whitespace = false;
    let mut looking_for_special = false;
    for char in iter {
        if char == '\r' || char == '\n' { 
            break; 
        }

        if col == pos.col {
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

    return Some(LinePos { line: pos.line, col })
}


pub fn find_current_word_end(state: &State, buf: &TextBuffer) -> Option<LinePos> {
    let pos = LinePos { line: state.cursor.y - 1, col: state.cursor.x - 1 };
    let iter = buf.utf8_iter(pos);

    let mut col = pos.col;
    let mut looking_for_letter = false;
    let mut looking_for_whitespace = false;
    let mut looking_for_special = false;
    for char in iter {
        if char == '\r' || char == '\n' { 
            break; 
        }

        if col == pos.col {
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

    return Some(LinePos { line: pos.line, col })
}


#[allow(non_snake_case)]
pub fn find_current_WORD_start(state: &State, buf: &TextBuffer) -> Option<LinePos> {
    let pos = LinePos { line: state.cursor.y - 1, col: state.cursor.x - 1 };
    let iter = buf.utf8_rev_iter(pos);

    let mut col = pos.col;
    let mut looking_for_whitespace = true;
    for char in iter {
        if char == '\r' || char == '\n' { 
            break; 
        }
        
        if col == pos.col && char.is_whitespace() {
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

    return Some(LinePos { line: pos.line, col })
}


#[allow(non_snake_case)]
pub fn find_current_WORD_end(state: &State, buf: &TextBuffer) -> Option<LinePos> {
    let pos = LinePos { line: state.cursor.y - 1, col: state.cursor.x - 1 };
    let iter = buf.utf8_iter(pos);

    let mut col = pos.col;
    let mut looking_for_whitespace = true;
    for char in iter {
        if char == '\r' || char == '\n' { 
            break; 
        }

        if col == pos.col && char.is_whitespace() {
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

    return Some(LinePos { line: pos.line, col })
}


pub fn find_previous_word_start(state: &State, buf: &TextBuffer) -> Option<LinePos> {
    let mut line = state.cursor.y - 1;
    let mut col = state.cursor.x - 1;

    if col > 0 {
        col -= 1;
    } else {
        if line == 0 {
            return None
        }
        let line_len = buf.line_len(state.cursor.y - 2);
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


pub fn find_next_word_end(state: &State, buf: &TextBuffer) -> Option<LinePos> {
    let mut line = state.cursor.y - 1;
    let mut col = state.cursor.x - 1;

    let line_len = buf.line_len(line);
    if col < line_len - 1 {
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
