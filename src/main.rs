use ropey::{Rope, RopeSlice};
use std::fs::File;
use std::io::{self, BufReader};
use std::path::Path;
use std::sync::{Arc, Mutex};

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
    pub fn as_index(&self, text: &Rope) -> ropey::Result<usize> {
        match self {
            Addr::Index(idx) => Ok(*idx),
            Addr::Coordinates(line, column) => {
                let idx = text.try_line_to_char(*line)?;
                Ok(idx + column)
            }
            Addr::LineStart(line) => text.try_line_to_char(*line),
            Addr::LineEnd(line) => Ok(text.try_line_to_char(*line)? + text.line(*line).len_chars()),
            Addr::BufferStart => Ok(0),
            Addr::BufferEnd => Ok(text.len_chars() - 1),
        }
    }

    pub fn as_coordinates(&self, text: &Rope) -> ropey::Result<(usize, usize)> {
        match self {
            Addr::Index(idx) => {
                let line = text.try_char_to_line(*idx)?;
                let column = idx - text.try_line_to_char(line)?;
                Ok((line, column))
            }
            Addr::Coordinates(line, column) => Ok((*line, *column)),
            Addr::LineStart(line) => Ok((*line, 0)),
            Addr::LineEnd(line) => Ok((*line, text.line(*line).len_chars() - 1)),
            Addr::BufferStart => Ok((0, 0)),
            Addr::BufferEnd => {
                let line = text.try_char_to_line(text.len_chars() - 1)?;
                let column = text.len_chars() - 1 - text.try_line_to_char(line)?;
                Ok((line, column))
            }
        }
    }

    pub fn move_left(&mut self, text: &Rope, n: usize) -> ropey::Result<()> {
        *self = Addr::Index(self.as_index(text)? + n);
        Ok(())
    }

    pub fn move_right(&mut self, text: &Rope, n: usize) -> ropey::Result<()> {
        *self = Addr::Index(self.as_index(text)? - n);
        Ok(())
    }
}

#[derive(Clone)]
pub struct Dot {
    from: Addr,
    to: Addr,
}

impl Dot {
    pub fn new() -> Dot {
        Dot {
            from: Addr::BufferStart,
            to: Addr::BufferEnd,
        }
    }
    pub fn left_right(&mut self, text: &Rope, left: Addr, right: Addr) -> ropey::Result<()> {
        self.from = Addr::Index(left.as_index(text)?);
        self.to = Addr::Index(right.as_index(text)?);
        Ok(())
    }
    pub fn anchor_left(&mut self, text: &Rope, anchor: Addr, to: Addr) -> ropey::Result<()> {
        self.from = Addr::Index(anchor.as_index(text)?);
        self.to = Addr::Index(anchor.as_index(text)? + to.as_index(text)?);
        Ok(())
    }

    pub fn anchor_right(&mut self, text: &Rope, from: Addr, anchor: Addr) -> ropey::Result<()> {
        self.from = Addr::Index(anchor.as_index(text)? - from.as_index(text)?);
        self.to = Addr::Index(anchor.as_index(text)?);
        Ok(())
    }

    pub fn move_left(&mut self, text: &Rope, n: usize) -> ropey::Result<()> {
        self.from.move_left(text, n)?;
        self.to.move_left(text, n)?;
        Ok(())
    }

    pub fn move_right(&mut self, text: &Rope, n: usize) -> ropey::Result<()> {
        self.from.move_right(text, n)?;
        self.to.move_right(text, n)?;
        Ok(())
    }

    pub fn extend_left(&mut self, text: &Rope, n: usize) -> ropey::Result<()> {
        self.to.move_left(text, n)?;
        Ok(())
    }

    pub fn extend_right(&mut self, text: &Rope, n: usize) -> ropey::Result<()> {
        self.from.move_right(text, n)?;
        Ok(())
    }

    pub fn trim_left(&mut self, text: &Rope, n: usize) -> ropey::Result<()> {
        self.to.move_right(text, n)?;
        if self.to.as_index(text)? < self.from.as_index(text)? {
            (self.from, self.to) = (self.to, self.from);
        }
        Ok(())
    }

    pub fn trim_right(&mut self, text: &Rope, n: usize) -> ropey::Result<()> {
        self.from.move_left(text, n)?;
        if self.from.as_index(text)? > self.to.as_index(text)? {
            (self.from, self.to) = (self.to, self.from);
        }
        Ok(())
    }

    pub fn get(&self, text: &Rope) -> ropey::Result<String> {
        let from = self.from.as_index(text)?;
        let to = self.to.as_index(text)?;

        let slice: String = text.slice(from..to).chars().collect();
        Ok(slice)
    }

    pub fn set(&mut self, text: &mut Rope, s: RopeSlice) -> ropey::Result<()> {
        let from = self.from.as_index(text)?;
        let to = self.to.as_index(text)?;

        text.remove(from..to);
        text.insert(from, &s.to_string());
        Ok(())
    }
}

#[derive(Clone)]
pub struct Buffer {
    text: Rope,
    dot: Vec<Dot>,
}

impl Default for Buffer {
    fn default() -> Self {
        Self::new()
    }
}

impl Buffer {
    pub fn new() -> Self {
        Buffer {
            text: Rope::new(),
            dot: vec![Dot::new()],
        }
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
            Ok(text) => Ok(Buffer {
                text,
                dot: vec![Dot::new()],
            }),
            Err(e) => Err(e),
        }
    }

    pub fn left_right(&mut self, left: Addr, right: Addr) {
        self.dot.iter_mut().for_each(|dot| {
            let _ = dot.left_right(&self.text, left, right);
        });
    }
    pub fn anchor_left(&mut self, anchor: Addr, to: Addr) {
        self.dot.iter_mut().for_each(|dot| {
            let _ = dot.anchor_left(&self.text, anchor, to);
        });
    }

    pub fn anchor_right(&mut self, from: Addr, anchor: Addr) {
        self.dot.iter_mut().for_each(|dot| {
            let _ = dot.anchor_right(&self.text, from, anchor);
        });
    }

    pub fn move_left(&mut self, n: usize) {
        self.dot.iter_mut().for_each(|dot| {
            let _ = dot.move_left(&self.text, n);
        });
    }

    pub fn move_right(&mut self, n: usize) {
        self.dot.iter_mut().for_each(|dot| {
            let _ = dot.move_right(&self.text, n);
        });
    }

    pub fn extend_left(&mut self, n: usize) {
        self.dot.iter_mut().for_each(|dot| {
            let _ = dot.extend_left(&self.text, n);
        });
    }

    pub fn extend_right(&mut self, n: usize) {
        self.dot.iter_mut().for_each(|dot| {
            let _ = dot.extend_right(&self.text, n);
        });
    }

    pub fn trim_left(&mut self, n: usize) {
        self.dot.iter_mut().for_each(|dot| {
            let _ = dot.trim_left(&self.text, n);
        });
    }

    pub fn trim_right(&mut self, n: usize) {
        self.dot.iter_mut().for_each(|dot| {
            let _ = dot.trim_right(&self.text, n);
        });
    }

    pub fn get(&self) -> Vec<ropey::Result<String>> {
        self.dot.iter().map(|dot| dot.get(&self.text)).collect()
    }

    pub fn set(&mut self, s: RopeSlice) {
        self.dot.iter_mut().for_each(|dot| {
            let _ = dot.set(&mut self.text, s);
        });
    }
}

fn main() {
    let buf = Buffer::from_file(Path::new("test.txt")).unwrap();
}
