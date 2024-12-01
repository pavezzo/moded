use crate::{editor::Editor, State};

type BarFn = fn (&State, &Editor, &str) -> bool;

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


fn write(_: &State, _: &Editor, _: &str) -> bool {
    true
}

fn edit(_: &State, _: &Editor, _: &str) -> bool {
    true
}

fn quit(_: &State, _: &Editor, _: &str) -> bool {
    true
}
