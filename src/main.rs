use ropey::{Rope, RopeSlice};
use std::fs::File;
use std::io::{self, BufReader};
use std::path::Path;

pub struct Addr {
    line: usize,
    column: usize,
}

pub struct Range {
    from: Addr,
    to: Addr,
}

impl Range {
    pub fn extend_left(&mut self, buf: &Buffer) -> ropey::Result<()> {
        let idx = buf.addr_to_idx(&self.from)?;
        let new_from = buf.idx_to_addr(idx - 1)?;
        self.from = new_from;
        Ok(())
    }

    pub fn extend_right(&mut self, buf: &Buffer) -> ropey::Result<()> {
        let idx = buf.addr_to_idx(&self.to)?;
        let new_to = buf.idx_to_addr(idx + 1)?;
        self.to = new_to;
        Ok(())
    }
}

pub struct Buffer {
    text: Rope,
}

impl Buffer {
    pub fn new() -> Self {
        Buffer { text: Rope::new() }
    }

    pub fn from_file(path: &Path) -> io::Result<Self> {
        let file = File::open(path)?;
        let reader = BufReader::new(file);
        Self::from_reader(reader)
    }

    pub fn from_reader<T>(reader: T) -> io::Result<Self>
    where
        T: io::Read,
    {
        match Rope::from_reader(reader) {
            Ok(v) => Ok(Buffer { text: v }),
            Err(e) => Err(e),
        }
    }

    fn addr_to_idx(&self, addr: &Addr) -> ropey::Result<usize> {
        let idx = self.text.try_line_to_char(addr.line)?;
        Ok(idx + addr.column)
    }

    fn idx_to_addr(&self, idx: usize) -> ropey::Result<Addr> {
        let line = self.text.try_char_to_line(idx)?;
        let line_idx = self.text.try_line_to_char(line)?;
        Ok(Addr {
            line: line,
            column: idx - line_idx,
        })
    }

    pub fn get_slice(&self, r: &Range) -> ropey::Result<RopeSlice> {
        let from_idx = self.addr_to_idx(&r.from)?;
        let to_idx = self.addr_to_idx(&r.to)?;
        Ok(self.text.slice(from_idx..to_idx))
    }

    pub fn set_slice(&mut self, r: &Range, s: RopeSlice) -> ropey::Result<Range> {
        let from_idx = self.addr_to_idx(&r.from)?;
        let to_idx = self.addr_to_idx(&r.to)?;
        self.text.remove(from_idx..to_idx);
        self.text.insert(from_idx, &s.to_string());
        let from = self.idx_to_addr(from_idx)?;
        let to = self.idx_to_addr(from_idx + s.len_chars())?;
        Ok(Range { from: from, to: to })
    }
}

fn main() {
    let buf = Buffer::from_file(Path::new("test.txt")).unwrap();
}
