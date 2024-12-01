use std::{io::Write, path::Path, sync::atomic::Ordering};

use crate::{editor::{next_buffer_id, Editor}, gap_buffer::{LineView, TextBuffer}, State, SHOULD_QUIT};

pub enum CommandBarAction {
    None,
    Quit,
    NewBuffer(TextBuffer),
    SwitchToBuffer(usize),
}

type Result = std::result::Result<CommandBarAction, ()>;
type BarFn = fn (&mut State, &Editor, &str) -> Result;

macro_rules! lookup_table {
    ($($name:expr => $func:expr),* $(,)?) => {
        const NAMES: &[&str] = &[
            $($name),*
        ];

        const FUNCTIONS: &[BarFn] = &[
            $($func),*
        ];
    };
}


// keep this sorted
lookup_table! {
    "e" => edit,
    "edit" => edit,
    "q" => quit,
    "quit" => quit,
    "w" => write,
    "write" => write,
}


pub fn match_cmd(input: &str) -> Option<BarFn> {
    let n = NAMES.binary_search(&input);
    let n = match n {
        Ok(n) => return Some(FUNCTIONS[n]),
        Err(n) => n,
    };

    if NAMES[n].starts_with(input) {
        return Some(FUNCTIONS[n]);
    }

    None
}



fn write(_: &mut State, editor: &Editor, args: &str) -> Result {
    let Some(buffer) = editor.buffers.get(editor.current_buffer) else { return Err(()) };
    let view = buffer.full_view();
    let Some(file_path) = &buffer.file_path else { return Err(()) };
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

    Ok(CommandBarAction::None)
}

fn edit(_: &mut State, editor: &Editor, args: &str) -> Result {
    for (i, buffer) in editor.buffers.iter().enumerate() {
        let Some(path) = &buffer.file_path else { continue };
        if let Some(path) = path.as_os_str().to_str() {
            if path == args {
                return Ok(CommandBarAction::SwitchToBuffer(i))
            }
        }
    }

    if args.len() > 0 {
        let buffer = TextBuffer::from_path(next_buffer_id(), Path::new(args));
        return Ok(CommandBarAction::NewBuffer(buffer))
    }

    Ok(CommandBarAction::None)
}

fn quit(_: &mut State, _: &Editor, _: &str) -> Result {
    SHOULD_QUIT.store(true, Ordering::Relaxed);
    Ok(CommandBarAction::None)
}
