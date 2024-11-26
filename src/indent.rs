use crate::gap_buffer::{LinePos, TextBuffer};

pub fn indent_wanted(line: usize, buf: &TextBuffer) -> Option<usize> {
    if line == 0 { return None }
    let iter = buf.bytes_iter(LinePos{ line: line - 1, col: 0 });

    let mut indent = 0;
    for byte in iter {
        if byte != b' ' { break }
        indent += 1;
    }

    Some(indent)
}
