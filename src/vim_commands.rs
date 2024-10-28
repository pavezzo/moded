use crate::{gap_buffer::TextBuffer, State};



#[repr(u8)]
#[derive(PartialEq, Eq)]
pub enum NormalCmd {
    Append,
    Down,
    Insert, 
    Left,
    LineEnd,
    LineStart,
    Right,
    Up,
    Word,
    Xdel,
}


impl NormalCmd {
    pub fn from_char(ch: char) -> Option<Self> {
        match ch {
            '$' => Some(NormalCmd::LineEnd),
            '0' => Some(NormalCmd::LineStart),
            'a' => Some(NormalCmd::Append),
            'h' => Some(NormalCmd::Left),
            'i' => Some(NormalCmd::Insert),
            'j' => Some(NormalCmd::Down),
            'k' => Some(NormalCmd::Up),
            'l' => Some(NormalCmd::Right),
            'w' => Some(NormalCmd::Word),
            'x' => Some(NormalCmd::Xdel),
            _ => None,
        }
    }
}

// TODO: maybe pass commands in stack since there shouldn't be that many expect for macros
//pub fn execute_normal_commands(state: &mut State, buffer: &mut TextBuffer, commands: Vec<NormalCmd>) -> Vec<NormalCmd> {
//    for cmd in commands {
//        match cmd {
//            NormalCmd::Append => {
//
//            },
//            NormalCmd::Down => todo!(),
//            NormalCmd::Insert => todo!(),
//            NormalCmd::Left => todo!(),
//            NormalCmd::LineEnd => todo!(),
//            NormalCmd::Right => todo!(),
//            NormalCmd::Up => todo!(),
//            NormalCmd::Word => todo!(),
//        }
//    }
//
//    commands
//}
