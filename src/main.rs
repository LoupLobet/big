use ropey::{Rope, RopeSlice};
use std::cmp::Ordering;
use std::fs::File;
use std::io::{self, BufReader};
use std::path::Path;

#[derive(Clone, Copy)]
pub enum Addr {
    Index(usize),
    Coordinates(usize, usize),
    LineStart(usize),
    LineEnd(usize),
    BufferStart,
    BufferEnd,
}

impl Addr {
    pub fn as_index(&self, buf: &Buffer) -> ropey::Result<usize> {
        match self {
            Addr::Index(idx) => Ok(*idx),
            Addr::Coordinates(line, column) => {
                let idx = buf.text.try_line_to_char(*line)?;
                Ok(idx + *column)
            }
            Addr::LineStart(line) => buf.text.try_line_to_char(*line),
            Addr::LineEnd(line) => {
                Ok(buf.text.try_line_to_char(*line)? + buf.text.line(*line).len_chars())
            }
            Addr::BufferStart => Ok(0),
            Addr::BufferEnd => Ok(buf.text.len_chars() - 1),
        }
    }

    pub fn as_coordinates(&self, buf: &Buffer) -> ropey::Result<(usize, usize)> {
        match self {
            Addr::Index(idx) => {
                let line = buf.text.try_char_to_line(*idx)?;
                let column = *idx - buf.text.try_line_to_char(line)?;
                Ok((line, column))
            }
            Addr::Coordinates(line, column) => Ok((*line, *column)),
            Addr::LineStart(line) => Ok((*line, 0)),
            Addr::LineEnd(line) => Ok((*line, buf.text.line(*line).len_chars() - 1)),
            Addr::BufferStart => Ok((0, 0)),
            Addr::BufferEnd => {
                let line = buf.text.try_char_to_line(buf.text.len_chars() - 1)?;
                let column = buf.text.len_chars() - 1 - buf.text.try_line_to_char(line)?;
                Ok((line, column))
            }
        }
    }

    pub fn next_char(&self, buf: &Buffer) -> ropey::Result<Addr> {
        Ok(Addr::Index(
            Addr::Index(self.as_index(buf)? + 1).as_index(buf)?,
        ))
    }

    pub fn prev_char(&self, buf: &Buffer) -> ropey::Result<Addr> {
        Ok(Addr::Index(
            Addr::Index(self.as_index(buf)? - 1).as_index(buf)?,
        ))
    }
}

pub struct Range {
    from: Addr,
    to: Addr,
}

impl Range {
    pub fn move_left(&mut self, buf: &Buffer) -> ropey::Result<()> {
        self.from = self.from.next_char(buf)?;
        self.to = self.to.next_char(buf)?;
        Ok(())
    }

    pub fn move_right(&mut self, buf: &Buffer) -> ropey::Result<()> {
        self.from = self.from.prev_char(buf)?;
        self.to = self.to.prev_char(buf)?;
        Ok(())
    }

    pub fn extend_left(&mut self, buf: &Buffer) -> ropey::Result<()> {
        self.to = self.to.next_char(buf)?;
        Ok(())
    }

    pub fn extend_right(&mut self, buf: &Buffer) -> ropey::Result<()> {
        self.from = self.from.prev_char(buf)?;
        Ok(())
    }

    pub fn trim_left(&mut self, buf: &Buffer) -> ropey::Result<()> {
        self.to = self.to.prev_char(buf)?;
        if self.to.as_index(buf)? < self.from.as_index(buf)? {
            (self.from, self.to) = (self.to, self.from);
        }
        Ok(())
    }

    pub fn trim_right(&mut self, buf: &Buffer) -> ropey::Result<()> {
        self.to = self.from.next_char(buf)?;
        if self.from.as_index(buf)? > self.to.as_index(buf)? {
            (self.from, self.to) = (self.to, self.from);
        }
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

    pub fn get_slice(&self, r: &Range) -> ropey::Result<RopeSlice> {
        let from = r.from.as_index(self)?;
        let to = r.to.as_index(self)?;
        Ok(self.text.slice(from..to))
    }

    pub fn set_slice(&mut self, r: &Range, s: RopeSlice) -> ropey::Result<Range> {
        let from = r.from.as_index(self)?;
        let to = r.to.as_index(self)?;
        self.text.remove(from..to);
        self.text.insert(from, &s.to_string());
        Ok(Range {
            from: Addr::Index(from),
            to: Addr::Index(from + s.len_chars()),
        })
    }
}

fn main() {
    let buf = Buffer::from_file(Path::new("test.txt")).unwrap();
    let mut line_2_range = Range {
        from: Addr::LineStart(1),
        to: Addr::LineEnd(1),
    };
    println!("line 2: {}", buf.get_slice(&line_2_range).unwrap());
    line_2_range.move_left(&buf).unwrap();
    line_2_range.extend_left(&buf).unwrap();
    line_2_range.extend_left(&buf).unwrap();
    println!(
        "line 2 forward by 1: {}",
        buf.get_slice(&line_2_range).unwrap()
    );
}
