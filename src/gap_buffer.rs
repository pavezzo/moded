use std::fmt::Debug;


#[derive(PartialEq, Eq, Debug, Clone, Copy)]
pub enum LineSeparator {
    LF = 1,
    CRLF = 2,
}

impl LineSeparator {
    pub fn as_str(&self) -> &'static str {
        match self {
            LineSeparator::LF => "\n",
            LineSeparator::CRLF => "\r\n",
        }
    }
}


// zero indexed
#[derive(Debug, Clone, Copy)]
pub struct LinePos {
    pub line: usize,
    pub col: usize,
}

impl LinePos {
    pub fn new(line: usize, col: usize) -> Self {
        Self { line, col }
    }
}

impl PartialEq for LinePos {
    fn eq(&self, other: &Self) -> bool {
        self.line == other.line && self.col == other.col
    }
}

impl Eq for LinePos {}

impl PartialOrd for LinePos {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        if self.line < other.line { return Some(std::cmp::Ordering::Less) }
        if self.line > other.line { return Some(std::cmp::Ordering::Greater) }
        if self.col < other.col { return Some(std::cmp::Ordering::Less) }
        if self.col > other.col { return Some(std::cmp::Ordering::Greater) }

        Some(std::cmp::Ordering::Equal)
    }
}

impl Ord for LinePos {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        unsafe { self.partial_cmp(other).unwrap_unchecked() }
    }
}


pub enum LineView<'a> {
    Contiguous(&'a str),
    Parts(&'a str, &'a str),
}


pub struct TextBuffer {
    pub chars: GapBuffer<u8>,
    pub lines: GapBuffer<usize>,
    pub line_sep: LineSeparator,
}

// everything is 0-indexed
impl TextBuffer {
    pub fn from_data(chars: Vec<u8>) -> Self {
        let mut lines = Vec::new();
        let st = unsafe {std::str::from_utf8_unchecked(&chars)};
        // assuming newlines for now
        let mut start = 0;
        let line_sep = if st.contains("\r\n") {
            LineSeparator::CRLF
        } else {
            LineSeparator::LF
        };

        for line in st.lines() {
            lines.push(start);
            start += line.len() + line_sep as usize;
        }
        let lines = GapBuffer::new(lines);
        println!("Using {:?} line separator", line_sep);

        Self { chars: GapBuffer::new(chars), lines, line_sep }
    }

    // TODO: maybe make this work with references
    pub fn line(&self, line: usize) -> String {
        let start = self.lines.get_one(line);
        let bytes = if line < self.total_lines() - 1 {
            let end = self.lines.get_one(line + 1);
            self.chars.get_by_range(start..end)
        } else {
            self.chars.get_to_end(start)
        };
        let mut st = String::from_utf8(bytes).unwrap();
        if st.ends_with(self.line_sep.as_str()) {
            if self.line_sep == LineSeparator::LF {
                st.pop();
            } else {
                st.pop();
                st.pop();
            }
        }

        st
    }

    pub fn raw_line(&self, line: usize) -> String {
        let start = self.lines.get_one(line);
        let bytes = if line < self.total_lines() - 1 {
            let end = self.lines.get_one(line + 1);
            self.chars.get_by_range(start..end)
        } else {
            self.chars.get_to_end(start)
        };
        let st = String::from_utf8(bytes).unwrap();

        st
    }

    // line length as seen in screen
    pub fn line_len(&self, line: usize) -> usize {
        let mut screen_len = 0;
        let iter = self.utf8_iter(LinePos{ line, col: 0 });
        for ch in iter {
            if ch == '\n' {
                if self.line_sep == LineSeparator::CRLF {
                    screen_len -= 1;
                }
                break;
            }
            screen_len += 1;
        }

        screen_len
    }

    // as bytes in buffer
    pub fn raw_line_len(&self, line: usize) -> usize {
        if line + 1 < self.total_lines() {
            return self.lines.get_one(line + 1) - self.lines.get_one(line)
        }

        let line_start = self.lines.get_one(line);
        self.chars.get_to_end(line_start).len()
    }

    pub fn total_lines(&self) -> usize {
        self.lines.len()
    }

    pub fn insert_into_line(&mut self, line: usize, index: usize, data: &[u8]) {
        let start = self.lines.get_one(line);
        let actual_bytes = self.screen_index_to_bytes_index(line, index);

        self.chars.insert(start + actual_bytes, data);
        self.lines.increment_range_by((line + 1)..self.lines.len(), data.len());
    }

    pub fn insert_empty_line(&mut self, row: usize) {
        if row < self.total_lines() {
            let index = self.lines.get_one(row);
            self.chars.insert(index, self.line_sep.as_str().as_bytes());
            self.lines.insert(row, &[index]);
            self.lines.increment_range_by((row+1)..self.lines.len(), self.line_sep as usize);
            return;
        }

        if row == self.total_lines() && !self.raw_line(row - 1).ends_with(self.line_sep.as_str()) {
            self.insert_into_line(row - 1, self.raw_line_len(row - 1), self.line_sep.as_str().as_bytes());
        }

        let index = self.lines.get_one(row - 1) + self.raw_line_len(row - 1);
        self.chars.insert(index, self.line_sep.as_str().as_bytes());
        let before = self.lines.get_one(row - 1) + self.raw_line_len(row - 1) - self.line_sep as usize;
        self.lines.insert(row, &[before]);
    }

    pub fn remove_from_line(&mut self, line: usize, index: usize, len: usize) {
        let start = self.lines.get_one(line);

        let actual_index = self.screen_index_to_bytes_index(line, index);
        let mut actual_len = 0;

        let iter = self.utf8_iter(LinePos{ line, col: 0 });
        for (i, char) in iter.enumerate() {
            if i >= index + len { break; }
            if i >= index { actual_len += char.len_utf8(); }
        }

        self.chars.remove(start + actual_index, actual_len);
        self.lines.decrement_range_by((line + 1)..self.lines.len(), actual_len);
    }

    pub fn remove_by_range(&mut self, start: LinePos, end: LinePos) {
        if start.line == end.line {
            let line_len = self.line_len(start.line);
            self.remove_from_line(start.line, start.col, (end.col - start.col + 1).min(line_len));
            if end.col == line_len && self.total_lines() > 1 {
                self.remove_line_sep(start.line);
            }
            return
        }

        let line_len = self.line_len(start.line);
        self.remove_from_line(start.line, start.col, line_len - start.col);

        self.remove_from_line(end.line, 0, end.col + 1);

        for _ in (start.line + 1)..end.line {
            self.remove_line(start.line + 1);
        }

        self.remove_line_sep(start.line);
    }

    pub fn remove_line(&mut self, line: usize) {
        let start = self.lines.get_one(line);
        let len = self.raw_line_len(line);
        self.chars.remove(start, len);
        if line < self.total_lines() - 1 {
            self.lines.decrement_range_by((line + 1)..self.lines.len(), len);
        }
        if self.total_lines() > 1 {
            self.lines.remove(line, 1);
        }
    }

    pub fn remove_line_sep(&mut self, line: usize) {
        let start = self.lines.get_one(line);
        let len = self.raw_line_len(line);
        self.chars.remove(start + len - self.line_sep as usize, self.line_sep as usize);
        self.lines.decrement_range_by((line + 1)..self.lines.len(), self.line_sep as usize);
        self.lines.remove(line + 1, 1);
    }

    pub fn split_line_at_index(&mut self, line: usize, index: usize) {
        let start = self.lines.get_one(line);

        let actual_index = self.screen_index_to_bytes_index(line, index);

        self.chars.insert(start + actual_index, self.line_sep.as_str().as_bytes());
        self.lines.insert(line + 1, &[start + actual_index]);
        self.lines.increment_range_by((line + 1)..self.lines.len(), self.line_sep as usize);
    }

    pub fn utf8_iter(&self, pos: LinePos) -> Utf8Iter {
        let start = self.lines.get_one(pos.line);
        let gap_iter = self.chars.into_iterator(start);
        let mut utf8_iter = Utf8Iter { inner: gap_iter };

        let mut i = 0;
        while i < pos.col {
            utf8_iter.next();
            i += 1;
        }

        utf8_iter
    }

    pub fn utf8_rev_iter(&self, pos: LinePos) -> Utf8RevIter {
        let line_len = self.line_len(pos.line) + self.line_sep as usize;
        let start = if pos.line < self.total_lines() - 1 {
            self.lines.get_one(pos.line + 1) - 1
        } else {
            self.chars.len() - 1
        };
        let gap_rev_iter = self.chars.into_rev_iterator(start);
        let mut utf8_rev_iter = Utf8RevIter { inner: gap_rev_iter };

        let mut i = 0;
        while i < line_len - pos.col - 1 {
            let c = utf8_rev_iter.next();
            i += 1;
        }

        utf8_rev_iter
    }

    // zero indexed
    fn screen_index_to_bytes_index(&self, line: usize, index: usize) -> usize {
        let iter = self.utf8_iter(LinePos{ line, col: 0 });
        let mut actual_index = 0;
        for (i, ch) in iter.enumerate() {
            if i >= index { break; }
            actual_index += ch.len_utf8();
        }

        actual_index
    }
}

pub struct Utf8Iter<'a> {
    inner: GapBufferIter<'a>, 
}

impl<'a> Iterator for Utf8Iter<'a> {
    type Item = char;

    fn next(&mut self) -> Option<Self::Item> {
        let first_byte = self.inner.next()?;
        if first_byte < 0b1000_0000 {
            return Some(first_byte as char)
        }

        const LEN_MASK: u8 = 0b1111_0000;
        let len = match first_byte & LEN_MASK {
            0b1100_0000 => 2,
            0b1110_0000 => 3,
            0b1111_0000 => 4,
            _ => return None,
        };

        const VALUE_MASKS: [u8; 3] = [0b0001_1111, 0b0000_1111, 0b0000_0111];
        let mut res = ((first_byte & VALUE_MASKS[len - 2]) as u32) << 6;

        const FOLLOW_MASK: u8 = 0b0011_1111;

        let next = unsafe { self.inner.next().unwrap_unchecked() };
        res |= (next & FOLLOW_MASK) as u32;

        if len > 2 {
            res <<= 6;
            let next = unsafe { self.inner.next().unwrap_unchecked() };
            res |= (next & FOLLOW_MASK) as u32;

            if len > 3 {
                res <<= 6;
                let next = unsafe { self.inner.next().unwrap_unchecked() };
                res |= (next & FOLLOW_MASK) as u32;
            }
        }

        unsafe { Some(std::char::from_u32_unchecked(res)) }
    }
}

pub struct Utf8RevIter<'a> {
    inner: GapBufferRevIter<'a>, 
}

impl<'a> Iterator for Utf8RevIter<'a> {
    type Item = char;

    fn next(&mut self) -> Option<Self::Item> {
        let last_byte = self.inner.next()?;
        if last_byte < 0b1000_0000 {
            return Some(last_byte as char)
        }

        const FOLLOW_MASK: u8 = 0b0011_1111;
        const VALUE_MASKS: [u8; 3] = [0b0001_1111, 0b0000_1111, 0b0000_0111];

        let mut len = 2;
        let mut res = (last_byte & FOLLOW_MASK) as u32;

        let next = unsafe { self.inner.next().unwrap_unchecked() };
        if next & 0b1100_0000 == 0b1100_0000 {
            res |= ((next & VALUE_MASKS[len - 2]) as u32) << 6;
            return unsafe { Some(std::char::from_u32_unchecked(res)) }
        }
        res |= ((next & FOLLOW_MASK) as u32) << 6;
        len += 1;

        let next = unsafe { self.inner.next().unwrap_unchecked() };
        if next & 0b1110_0000 == 0b1110_0000 {
            res |= ((next & VALUE_MASKS[len - 2]) as u32) << 12;
            return unsafe { Some(std::char::from_u32_unchecked(res)) }
        }
        res |= ((next & FOLLOW_MASK) as u32) << 12;
        len += 1;

        let next = unsafe { self.inner.next().unwrap_unchecked() };
        res |= ((next & VALUE_MASKS[len - 2]) as u32) << 18;
        unsafe { Some(std::char::from_u32_unchecked(res)) }
    }
}


pub struct GapBuffer<T: Copy + Debug + std::ops::Add + std::ops::AddAssign + std::ops::SubAssign> {
    data: Vec<T>,
    gap_start: usize,
    gap_end: usize,
}

impl<T: Copy + Debug + std::ops::Add + std::ops::AddAssign + std::ops::SubAssign> GapBuffer<T> {
    pub fn new(mut data: Vec<T>) -> Self {
        let gap_start = data.len();
        let gap_end = data.capacity();
        unsafe {
            data.set_len(gap_end);
        }
        
        Self { data, gap_start, gap_end }
    }

    pub fn insert(&mut self, index: usize, data: &[T]) {
        //assert!(index <= self.data.len() - (self.gap_end - self.gap_start));

        let mut gap_size = self.gap_end - self.gap_start;

        if gap_size < data.len() {
            let old_len = self.data.len();
            self.data.reserve(data.len());
            let added_size = self.data.capacity() - old_len;
            unsafe {
                self.data.set_len(self.data.capacity());
            }
            if self.gap_end > old_len {
                self.move_bytes(self.gap_end, self.gap_end + added_size, self.gap_end - old_len);
            } else {
                self.move_bytes(self.gap_end, self.gap_end + added_size, old_len - self.gap_end);
            }
            gap_size += added_size;
            self.gap_end += added_size;
        }

        if self.gap_start == index {
            self.data[self.gap_start..(self.gap_start + data.len())].copy_from_slice(data);
            self.gap_start += data.len();
            return;
        }

        if index > self.gap_start {
            self.move_bytes(self.gap_end, self.gap_start, index - self.gap_start);
        } else {
            self.move_bytes(index, index + gap_size, self.gap_start - index);
        }

        //self.move_bytes(index, index + gap_size, self.data.len() - index - gap_size);
        self.gap_start = index;
        self.gap_end = index + gap_size;

        self.data[self.gap_start..(self.gap_start + data.len())].copy_from_slice(data);
        self.gap_start += data.len();
        assert!(self.gap_end <= self.data.len(), "gap_end: {}, data.len(): {}, raw bytes: {:?}", self.gap_end, self.data.len(), self.get_by_range(0..(self.data.len() - (self.gap_end - self.gap_start))));
    }

    pub fn remove(&mut self, from: usize, len: usize) {
        let gap_size = self.gap_end - self.gap_start;
        if gap_size == 0 {
            self.gap_start = from;
            self.gap_end = from + len;
            return;
        }

        let index = if from < self.gap_start {
            from
        } else {
            from + gap_size
        };

        if index == self.gap_end {
            self.gap_end += len;
            return
        }

        if index + len == self.gap_start {
            self.gap_start = index;
            return;
        }

        if index < self.gap_start && index + len > self.gap_start {
            self.gap_end += len - (self.gap_start - index);
            self.gap_start = index;
            return;
        }

        if index < self.gap_start {
            self.move_bytes(index + len, index, self.gap_start - index - len);
            //self.gap_start = index + len;
            self.gap_start = index + (self.gap_start - index - len);
            //self.gap_end = self.gap_start + gap_size;
        } else {
            let gap = index - self.gap_end;
            self.move_bytes(self.gap_end, self.gap_start, gap);
            self.gap_start += gap;
            self.gap_end = index + len;
        }

        assert!(self.gap_end <= self.data.len(), "gap_end: {}, data.len(): {}", self.gap_end, self.data.len());
    }

    pub fn get_one(&self, pos: usize) -> T {
        //assert!(pos < self.data.len() - (self.gap_end - self.gap_start), "pos: {}, data.len() - gap_size: {}, data.len(): {}", pos, self.data.len() - (self.gap_end - self.gap_start), self.data.len());
        if pos < self.gap_start {
            return self.data[pos]
        }

        self.data[pos + (self.gap_end - self.gap_start)]
    }

    pub fn get_by_range(&self, range: std::ops::Range<usize>) -> Vec<T> {
        assert!(range.start <= range.end, "range.start: {}, range.end: {}", range.start, range.end);

        if range.end < self.gap_start {
            return self.data[range].to_vec();
        } else if range.start >= self.gap_start {
            return self.data[(range.start + (self.gap_end - self.gap_start))..(range.end + (self.gap_end - self.gap_start))].to_vec()
        }

        let gap_size = self.gap_end - self.gap_start;
        let mut first_part = self.data[range.start..self.gap_start].to_owned(); 
        first_part.extend_from_slice(&self.data[self.gap_end..(range.end + gap_size)]);

        first_part
    }

    pub fn get_to_end(&self, start: usize) -> Vec<T> {
        let gap_size = self.gap_end - self.gap_start;

        if start >= self.gap_start {
            return self.data[(start + gap_size)..self.data.len()].to_vec()
        }

        let mut first_part = self.data[start..self.gap_start].to_owned(); 
        first_part.extend_from_slice(&self.data[self.gap_end..self.data.len()]);

        first_part
    }

    pub fn push_back(&mut self, data: &[T]) {
        let mut gap_size = self.gap_end - self.gap_start;

        if gap_size < data.len() {
            let old_len = self.data.len();
            self.data.reserve(data.len());
            let added_size = self.data.capacity() - old_len;
            unsafe {
                self.data.set_len(self.data.capacity());
            }
            if self.gap_end > old_len {
                self.move_bytes(self.gap_end, self.gap_end + added_size, self.gap_end - old_len);
            } else {
                self.move_bytes(self.gap_end, self.gap_end + added_size, old_len - self.gap_end);
            }
            gap_size += added_size;
            self.gap_end += added_size;
        }

        let new_index = self.data.len() - gap_size;

        if self.gap_start == new_index {
            self.data[self.gap_start..(self.gap_start + data.len())].copy_from_slice(data);
            self.gap_start += data.len();
            return;
        }

        if new_index >= self.gap_start {
            self.gap_start += data.len();
            self.data[self.gap_start..(self.gap_start + data.len())].copy_from_slice(data);
            return;
        }

        self.move_bytes(new_index, new_index + gap_size, self.gap_start - new_index);

        self.gap_start = new_index;
        self.gap_end = new_index + gap_size;

        self.data[self.gap_start..(self.gap_start + data.len())].copy_from_slice(data);
        self.gap_start += data.len();
    }

    pub fn len(&self) -> usize {
        self.data.len() - (self.gap_end - self.gap_start)
    }

    fn move_bytes(&mut self, from: usize, to: usize, len: usize) {
        //println!("moving: from: {from}, to: {to}, len: {len}, data.len: {}, gap_start: {}, gap_end: {}, data: {:?}", self.data.len(), self.gap_start, self.gap_end, self.data);
        self.data.copy_within(from..(from+len), to);
    }

}

impl GapBuffer<usize> {
    pub fn increment_range_by(&mut self, range: std::ops::Range<usize>, by: usize) {
        if range.end < self.gap_start {
            for val in &mut self.data[range] {
                *val += by;
            }
            return;
        } else if range.start >= self.gap_start {
            for val in &mut self.data[(range.start + (self.gap_end - self.gap_start))..(range.end + (self.gap_end - self.gap_start))] {
                *val += by;
            }
            return;
        }

        let gap_size = self.gap_end - self.gap_start;
        for val in &mut self.data[range.start..self.gap_start] {
            *val += by;
        }
        for val in &mut self.data[self.gap_end..(range.end + gap_size)] {
            *val += by;
        }
    }

    pub fn decrement_range_by(&mut self, range: std::ops::Range<usize>, by: usize) {
        if range.end < self.gap_start {
            for val in &mut self.data[range] {
                *val -= by;
            }
            return;
        } else if range.start >= self.gap_start {
            for val in &mut self.data[(range.start + (self.gap_end - self.gap_start))..(range.end + (self.gap_end - self.gap_start))] {
                *val -= by;
            }
            return;
        }

        let gap_size = self.gap_end - self.gap_start;
        for val in &mut self.data[range.start..self.gap_start] {
            *val -= by;
        }
        for val in &mut self.data[self.gap_end..(range.end + gap_size)] {
            *val -= by;
        }
    }
}

impl GapBuffer<u8> {
    fn into_iterator(&self, index: usize) -> GapBufferIter {
        GapBufferIter { index, inner: self }
    }

    fn into_rev_iterator(&self, index: usize) -> GapBufferRevIter {
        GapBufferRevIter { index: index + 1, inner: self }
    }
}

pub struct GapBufferIter<'a> {
    index: usize,
    inner: &'a GapBuffer<u8> 
}

impl<'a> Iterator for GapBufferIter<'a> {
    type Item = u8;

    fn next(&mut self) -> Option<Self::Item> {
        let gap_size = self.inner.gap_end - self.inner.gap_start;
        if self.index >= self.inner.data.len() - gap_size { return None }
        self.index += 1;
        if self.index <= self.inner.gap_start {
            return Some(self.inner.data[self.index - 1])
        }

        Some(self.inner.data[self.index - 1 + gap_size])
    }
}

pub struct GapBufferRevIter<'a> {
    index: usize,
    inner: &'a GapBuffer<u8> 
}

impl<'a> Iterator for GapBufferRevIter<'a> {
    type Item = u8;

    fn next(&mut self) -> Option<Self::Item> {
        let gap_size = self.inner.gap_end - self.inner.gap_start;
        if self.index == 0 { return None; }
        if self.index - 1 < self.inner.gap_start {
            let char = self.inner.data[self.index - 1];
            self.index -= 1;
            return Some(char)
        }

        let char = self.inner.data[self.index - 1 + gap_size];
        self.index -= 1;
        Some(char)
    }
}


#[cfg(test)]
mod tests {
    use std::str::FromStr;

    use super::*;

    fn no_gap(buf: &GapBuffer<u8>) -> String {
        let mut a = String::from_str(std::str::from_utf8(&buf.data[0..buf.gap_start]).unwrap()).unwrap();
        if buf.gap_end <= buf.data.len() {
            a.push_str(std::str::from_utf8(&buf.data[buf.gap_end..buf.data.len()]).unwrap());
        } else {
            panic!("gap_end out of buffer bounds!");
        }

        a
    }
    
    #[test]
    fn test_insert() {
        let data = "test data for myself".as_bytes();
        let mut buf = GapBuffer::new(data.to_vec());

        buf.insert(4, "best".as_bytes());

        assert_eq!(&buf.data[0..8], "testbest".as_bytes());
        assert_eq!(&buf.data[buf.gap_end..buf.data.len()], " data for myself".as_bytes());

        println!("gap_start: {}, gap_end: {}", buf.gap_start, buf.gap_end);
        unsafe {println!("raw: {}", std::str::from_utf8_unchecked(&buf.data))};
        println!("nogap: {}\n", no_gap(&buf));

        //"testbest data for myself"
        //"testbest data for me and myself"
        buf.insert(18, "me and ".as_bytes());

        println!("gap_start: {}, gap_end: {}", buf.gap_start, buf.gap_end);
        unsafe {println!("raw: {}", std::str::from_utf8_unchecked(&buf.data))};
        println!("nogap: {}\n", no_gap(&buf));

        assert_eq!(&buf.data[0..buf.gap_start], "testbest data for me and ".as_bytes());

        //"testbest data for me and myself"
        // "this is testbest data for me and myself"

        buf.insert(0, "this is ".as_bytes());

        println!("gap_start: {}, gap_end: {}", buf.gap_start, buf.gap_end);
        unsafe {println!("raw: {}", std::str::from_utf8_unchecked(&buf.data))};
        println!("nogap: {}\n", no_gap(&buf));
        assert_eq!("this is testbest data for me and myself", &no_gap(&buf));

        // "this is testbest data for me and myself"
        // "this is testbest data for me and myself and it just works"

        buf.insert(39, " and it just works".as_bytes());

        println!("gap_start: {}, gap_end: {}", buf.gap_start, buf.gap_end);
        unsafe {println!("raw: {}", std::str::from_utf8_unchecked(&buf.data))};
        println!("nogap: {}\n", no_gap(&buf));
        assert_eq!("this is testbest data for me and myself and it just works", &no_gap(&buf));


        // "this is testbest data for me and myself and it just works"
        // "this is and will be testbest data for me and myself and it just works"

        buf.insert(8, "and will be ".as_bytes());

        println!("gap_start: {}, gap_end: {}", buf.gap_start, buf.gap_end);
        unsafe {println!("raw: {}", std::str::from_utf8_unchecked(&buf.data))};
        println!("nogap: {}\n", no_gap(&buf));
        assert_eq!("this is and will be testbest data for me and myself and it just works", &no_gap(&buf));
    }

    #[test]
    fn test_remove() {
        let data = "test data for myself".as_bytes();
        let mut buf = GapBuffer::new(data.to_vec());

        // "test data for myself"
        // "test for myself"
        buf.remove(5, 5);

        println!("gap_start: {}, gap_end: {}, data len: {}", buf.gap_start, buf.gap_end, buf.data.len());
        unsafe {println!("raw: {}", std::str::from_utf8_unchecked(&buf.data))};
        println!("nogap: {}\n", no_gap(&buf));
        assert_eq!("test for myself", &no_gap(&buf));


        // "test for myself"
        // "test for elf"
        buf.remove(9, 3);

        println!("gap_start: {}, gap_end: {}, data len: {}", buf.gap_start, buf.gap_end, buf.data.len());
        unsafe {println!("raw: {}", std::str::from_utf8_unchecked(&buf.data))};
        println!("nogap: {}\n", no_gap(&buf));
        assert_eq!("test for elf", &no_gap(&buf));


        // "test for elf"
        // "test for e"
        buf.remove(10, 2);

        println!("gap_start: {}, gap_end: {}, data len: {}", buf.gap_start, buf.gap_end, buf.data.len());
        unsafe {println!("raw: {}", std::str::from_utf8_unchecked(&buf.data))};
        println!("nogap: {}\n", no_gap(&buf));
        assert_eq!("test for e", &no_gap(&buf));


        // "test for e"
        // "test for er"
        buf.insert(10, "r".as_bytes());

        println!("gap_start: {}, gap_end: {}, data len: {}", buf.gap_start, buf.gap_end, buf.data.len());
        unsafe {println!("raw: {}", std::str::from_utf8_unchecked(&buf.data))};
        println!("nogap: {}\n", no_gap(&buf));
        assert_eq!("test for er", &no_gap(&buf));


        // "test for er"
        // "tester"
        buf.remove(4, 5);

        println!("gap_start: {}, gap_end: {}, data len: {}", buf.gap_start, buf.gap_end, buf.data.len());
        unsafe {println!("raw: {}", std::str::from_utf8_unchecked(&buf.data))};
        println!("nogap: {}\n", no_gap(&buf));
        assert_eq!("tester", &no_gap(&buf));

        // "tester"
        // "tster"
        buf.remove(1, 1);

        println!("gap_start: {}, gap_end: {}, data len: {}", buf.gap_start, buf.gap_end, buf.data.len());
        unsafe {println!("raw: {}", std::str::from_utf8_unchecked(&buf.data))};
        println!("nogap: {}\n", no_gap(&buf));
        assert_eq!("tster", &no_gap(&buf));

        // "tster"
        // "tst"
        buf.remove(3, 2);

        println!("gap_start: {}, gap_end: {}, data len: {}", buf.gap_start, buf.gap_end, buf.data.len());
        unsafe {println!("raw: {}", std::str::from_utf8_unchecked(&buf.data))};
        println!("nogap: {}\n", no_gap(&buf));
        assert_eq!("tst", &no_gap(&buf));
    }



    #[test]
    fn test_char_iter() {
        let str = "tes😃t😂iä\nja toinen 🤝 kolmas\nneljäs: ภๅ";
        let buf = TextBuffer::from_data(str.as_bytes().to_vec());

        let mut st = String::new();
        for char in buf.utf8_iter(LinePos { line: 0, col: 0 }) {
            st.push(char);
        }

        assert!(st == str);
    }

    #[test]
    fn test_rev_char_iter() {
        //let str = "tes😃t😂iä\nja toinen 🤝 kolmas\nneljäs: ภๅ";
        let str = "testiä";
        let buf = TextBuffer::from_data(str.as_bytes().to_vec());

        let mut st = String::new();
        for char in buf.utf8_rev_iter(LinePos { line: 0, col: 5 }) {
            if st.len() == 0 {
                st.push(char);
            } else {
                st.insert(0, char);
            }

        }

        assert!(str == st, "{} != {}", str, st);
    }
}
