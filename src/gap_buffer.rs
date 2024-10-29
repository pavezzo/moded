use std::fmt::Debug;



fn screen_chars_size(bytes: &[u8]) -> usize {
    let st = std::str::from_utf8(&bytes).unwrap();
    st.len()
}

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



pub struct TextBuffer {
    pub chars: GapBuffer<u8>,
    pub lines: GapBuffer<usize>,
    pub line_sep: LineSeparator,
}

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
        let start = self.lines.get_by_pos(line);
        let bytes = if line < self.total_lines() - 1 {
            let end = self.lines.get_by_pos(line + 1);
            self.chars.get_by_range(start..end)
        } else {
            self.chars.get_to_end(start)
        };
        let mut st = String::from_utf8(bytes).unwrap();
        if self.line_sep == LineSeparator::LF {
            st.pop();
        } else {
            st.pop();
            st.pop();
        }

        st
    }

    pub fn raw_line(&self, line: usize) -> String {
        let start = self.lines.get_by_pos(line);
        let bytes = if line < self.total_lines() - 1 {
            let end = self.lines.get_by_pos(line + 1);
            self.chars.get_by_range(start..end)
        } else {
            self.chars.get_to_end(start)
        };
        let mut st = String::from_utf8(bytes).unwrap();

        st
    }

    // line lenght as seen by the user
    pub fn line_len(&self, line: usize) -> usize {
        let line = self.raw_line(line);
        return line.chars().count() - self.line_sep as usize;
        //if line + 1 < self.lines_len() {
        //    //return self.lines.get_by_pos(line + 1) - self.lines.get_by_pos(line)
        //}
        //
        //let line_start = self.lines.get_by_pos(line);
        //self.chars.get_to_end(line_start).len()
    }

    // as bytes in buffer
    pub fn raw_line_len(&self, line: usize) -> usize {
        if line + 1 < self.total_lines() {
            return self.lines.get_by_pos(line + 1) - self.lines.get_by_pos(line)
        }

        let line_start = self.lines.get_by_pos(line);
        self.chars.get_to_end(line_start).len()
    }

    pub fn total_lines(&self) -> usize {
        self.lines.len()
    }

    pub fn insert_into_line(&mut self, line: usize, index: usize, data: &[u8]) {
        let start = self.lines.get_by_pos(line);

        let st = self.line(line);
        let mut i = 0;
        let mut actual_bytes = 0;
        for char in st.chars() {
            if i >= index { break; }
            actual_bytes += char.len_utf8();
            i += 1;
        }

        self.chars.insert(start + actual_bytes, data);
        self.lines.increment_range_by((line + 1)..self.lines.len(), data.len());
    }

    pub fn insert_empty_line(&mut self, row: usize) {
        if row < self.total_lines() {
            let index = self.lines.get_by_pos(row);
            self.chars.insert(index, self.line_sep.as_str().as_bytes());
            self.lines.insert(row, &[index]);
            self.lines.increment_range_by((row+1)..self.lines.len(), self.line_sep as usize);
            return;
        }

        let index = self.lines.get_by_pos(row - 1) + self.raw_line_len(row - 1);
        self.chars.insert(index, self.line_sep.as_str().as_bytes());
        let before = self.lines.get_by_pos(row - 1) + self.raw_line_len(row - 1) - self.line_sep as usize;
        self.lines.insert(row, &[before]);
    }

    pub fn remove_from_line(&mut self, line: usize, index: usize, len: usize) {
        let start = self.lines.get_by_pos(line);
        let st = self.raw_line(line);

        let mut actual_index = 0;
        let mut actual_len = 0;
        //let skip = (index as isize - 1).max(0) as usize;
        for (i, char) in st.chars().enumerate() {
            if i >= index + len { break; }
            if i < index { actual_index += char.len_utf8(); }
            if i >= index { actual_len += char.len_utf8(); }
        }

        if (actual_len + actual_index + self.line_sep as usize) == st.len() {
            actual_len += self.line_sep as usize - 1;
        }


        let b = self.chars.get_by_range((start + actual_index)..(start + actual_index + actual_len));
        self.chars.remove(start + actual_index, actual_len);
        self.lines.decrement_range_by((line + 1)..self.lines.len(), actual_len);

        if b == self.line_sep.as_str().as_bytes() {
            self.lines.remove(line + 1, 1);
        }
    }

    pub fn remove_line(&mut self, line: usize) {
        let start = self.lines.get_by_pos(line);
        let len = self.raw_line_len(line);
        self.chars.remove(start, len);
        self.lines.decrement_range_by((line + 1)..self.lines.len(), len);
        self.lines.remove(line, 1);
    }

    pub fn remove_line_sep(&mut self, line: usize) {
        let start = self.lines.get_by_pos(line);
        let len = self.raw_line_len(line);
        //let next_line_len = self.line_len(line + 1);
        self.chars.remove(start + len - self.line_sep as usize, self.line_sep as usize);
        self.lines.decrement_range_by((line + 1)..self.lines.len(), self.line_sep as usize);
        self.lines.remove(line + 1, 1);
    }

    pub fn split_line_at_index(&mut self, line: usize, index: usize) {
        let start = self.lines.get_by_pos(line);

        let st = self.line(line);
        let mut actual_index = 0;
        for (i, char) in st.chars().enumerate() {
            if i >= index { break; }
            actual_index += char.len_utf8();
        }

        self.chars.insert(start + actual_index, self.line_sep.as_str().as_bytes());
        self.lines.insert(line + 1, &[start + actual_index]);
        self.lines.increment_range_by((line + 1)..self.lines.len(), self.line_sep as usize);
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
        //self.gap_start = index;
        //self.gap_end = self.gap_start + gap_size;

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

    pub fn get_by_pos(&self, pos: usize) -> T {
        assert!(pos < self.data.len() - (self.gap_end - self.gap_start), "pos: {}, data.len() - gap_size: {}", pos, self.data.len() - (self.gap_end - self.gap_start));
        if pos < self.gap_start {
            return self.data[pos]
        }

        self.data[pos + (self.gap_end - self.gap_start)]
    }

    //pub fn get_by_range(&self, range: std::ops::Range<usize>) -> &[T] {
    pub fn get_by_range(&self, range: std::ops::Range<usize>) -> Vec<T> {
        assert!(range.start <= range.end, "range.start: {}, range.end: {}", range.start, range.end);

        if range.end < self.gap_start {
            return self.data[range].to_vec();
        } else if range.start >= self.gap_start {
            return self.data[(range.start + (self.gap_end - self.gap_start))..(range.end + (self.gap_end - self.gap_start))].to_vec()
        }

        let gap_size = self.gap_end - self.gap_start;
        let mut first_part = self.data[range.start..self.gap_start].to_owned(); 
        //first_part.copy_from_slice(&self.data[self.gap_end..(range.end + gap_size)]);
        first_part.extend_from_slice(&self.data[self.gap_end..(range.end + gap_size)]);

        first_part
    }

    pub fn get_to_end(&self, start: usize) -> Vec<T> {
        let gap_size = self.gap_end - self.gap_start;
        let end = if self.gap_end == self.data.len() {
            self.gap_start - 1
        } else {
            self.data.len()
        };
        if start >= self.gap_start {
            return self.data[(start + gap_size)..self.data.len()].to_vec()
        }

        let mut first_part = self.data[start..self.gap_start].to_owned(); 
        first_part.extend_from_slice(&self.data[self.gap_end..self.data.len()]);

        first_part
    }

    pub fn increment_range_by(&mut self, range: std::ops::Range<usize>, by: T) {
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

    pub fn decrement_range_by(&mut self, range: std::ops::Range<usize>, by: T) {
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
            //self.insert(new_index, data)
            self.data[self.gap_start..(self.gap_start + data.len())].copy_from_slice(data);
            //self.move_bytes(self.gap_end, self.gap_start, new_index - self.gap_start);
            return;
        } else {

            self.move_bytes(new_index, new_index + gap_size, self.gap_start - new_index);
        }


        //if new_index > self.gap_start {
        //    self.move_bytes(self.gap_end, self.gap_start, new_index - self.gap_start);
        //} else {
        //    self.move_bytes(new_index, new_index + gap_size, self.gap_start - new_index);
        //}

        //self.move_bytes(index, index + gap_size, self.data.len() - index - gap_size);
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
}
