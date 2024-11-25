use crate::gap_buffer::{LinePos, LineView, TextBuffer};


pub fn search(needle: &[u8], buf: &TextBuffer) -> Vec<LinePos> {
    let view = buf.full_view();
    let mut positions = Vec::new();
    match view {
        LineView::Contiguous(s) => {
            for (i, window) in s.as_bytes().windows(needle.len()).enumerate() {
                if window == needle {
                    positions.push(buf.byte_to_linepos(i));
                }
            }
        },
        LineView::Parts(s1, s2) => {
            for (i, window) in s1.as_bytes().windows(needle.len()).enumerate() {
                if window == needle {
                    positions.push(buf.byte_to_linepos(i));
                }
            }
            for (i, window) in s2.as_bytes().windows(needle.len()).enumerate() {
                if window == needle {
                    positions.push(buf.byte_to_linepos(i + s1.len()));
                }
            }
        },
    }

    positions
}
