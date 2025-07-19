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
    pub fn as_index(&self, text: Arc<Mutex<Rope>>) -> ropey::Result<usize> {
        let text = text.lock().unwrap();
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

    pub fn as_coordinates(&self, text: Arc<Mutex<Rope>>) -> ropey::Result<(usize, usize)> {
        let text = text.lock().unwrap();
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

    pub fn move_left(&mut self, n: usize, text: Arc<Mutex<Rope>>) -> ropey::Result<()> {
        *self = Addr::Index(self.as_index(text)? + n);
        Ok(())
    }

    pub fn move_right(&mut self, n: usize, text: Arc<Mutex<Rope>>) -> ropey::Result<()> {
        *self = Addr::Index(self.as_index(text)? - n);
        Ok(())
    }
}

#[derive(Clone)]
pub struct Address {
    text: Arc<Mutex<Rope>>,
    addr: Addr,
}

impl Address {
    pub fn as_index(&self) -> ropey::Result<usize> {
        self.addr.as_index(self.text.clone())
    }

    pub fn as_coordinates(&self) -> ropey::Result<(usize, usize)> {
        self.addr.as_coordinates(self.text.clone())
    }

    pub fn move_left(&mut self, n: usize) -> ropey::Result<()> {
        self.addr = Addr::Index(self.as_index()? + n);
        Ok(())
    }

    pub fn move_right(&mut self, n: usize) -> ropey::Result<()> {
        self.addr = Addr::Index(self.as_index()? - n);
        Ok(())
    }
}

#[derive(Clone)]
pub struct Dot {
    text: Arc<Mutex<Rope>>,
    from: Addr,
    to: Addr,
}

impl Dot {
    pub fn left_right(mut self, left: Addr, right: Addr) -> ropey::Result<Self> {
        self.from = Addr::Index(left.as_index(self.text.clone())?);
        self.to = Addr::Index(right.as_index(self.text.clone())?);
        Ok(self)
    }
    pub fn anchor_left(mut self, anchor: Addr, to: Addr) -> ropey::Result<Self> {
        self.from = Addr::Index(anchor.as_index(self.text.clone())?);
        self.to =
            Addr::Index(anchor.as_index(self.text.clone())? + to.as_index(self.text.clone())?);
        Ok(self)
    }

    pub fn anchor_right(mut self, from: Addr, anchor: Addr) -> ropey::Result<Self> {
        self.from =
            Addr::Index(anchor.as_index(self.text.clone())? - from.as_index(self.text.clone())?);
        self.to = Addr::Index(anchor.as_index(self.text.clone())?);
        Ok(self)
    }

    pub fn move_left(mut self, n: usize) -> ropey::Result<Self> {
        self.from.move_left(n, self.text.clone())?;
        self.to.move_left(n, self.text.clone())?;
        Ok(self)
    }

    pub fn move_right(mut self, n: usize) -> ropey::Result<Self> {
        self.from.move_right(n, self.text.clone())?;
        self.to.move_right(n, self.text.clone())?;
        Ok(self)
    }

    pub fn extend_left(mut self, n: usize) -> ropey::Result<Self> {
        self.to.move_left(n, self.text.clone())?;
        Ok(self)
    }

    pub fn extend_right(mut self, n: usize) -> ropey::Result<Self> {
        self.from.move_right(n, self.text.clone())?;
        Ok(self)
    }

    pub fn trim_left(mut self, n: usize) -> ropey::Result<Self> {
        self.to.move_right(n, self.text.clone())?;
        if self.to.as_index(self.text.clone())? < self.from.as_index(self.text.clone())? {
            (self.from, self.to) = (self.to, self.from);
        }
        Ok(self)
    }

    pub fn trim_right(mut self, n: usize) -> ropey::Result<Self> {
        self.from.move_left(n, self.text.clone())?;
        if self.from.as_index(self.text.clone())? > self.to.as_index(self.text.clone())? {
            (self.from, self.to) = (self.to, self.from);
        }
        Ok(self)
    }

    pub fn get(&self) -> ropey::Result<String> {
        let from = self.from.as_index(self.text.clone())?;
        let to = self.to.as_index(self.text.clone())?;

        let text = self.text.lock().unwrap();
        let slice: String = text.slice(from..to).chars().collect();
        Ok(slice)
    }

    pub fn set(self, s: RopeSlice) -> ropey::Result<Self> {
        let from = self.from.as_index(self.text.clone())?;
        let to = self.to.as_index(self.text.clone())?;

        let text = Arc::clone(&self.text);
        let mut text = text.lock().unwrap();
        text.remove(from..to);
        text.insert(from, &s.to_string());
        Ok(self)
    }
}

#[derive(Clone)]
pub struct Buffer {
    text: Arc<Mutex<Rope>>,
}

impl Default for Buffer {
    fn default() -> Self {
        Self::new()
    }
}

impl Buffer {
    pub fn new() -> Self {
        Buffer {
            text: Arc::new(Mutex::new(Rope::new())),
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
                text: Arc::new(Mutex::new(text)),
            }),
            Err(e) => Err(e),
        }
    }

    pub fn new_dot(&self) -> Dot {
        Dot {
            text: Arc::clone(&self.text),
            from: Addr::BufferStart,
            to: Addr::BufferStart,
        }
    }
}

fn main() {
    let buf = Buffer::from_file(Path::new("test.txt")).unwrap();
    let mut dot = buf
        .new_dot()
        .anchor_left(Addr::LineStart(1), Addr::Index(3))
        .unwrap();
    println!("line 2: {}", dot.get().unwrap());
    dot = dot.move_left(1).unwrap();
    dot = dot.extend_left(1).unwrap();
    println!("line 2 forward by 1: {}", dot.get().unwrap());

    buf.new_dot()
        .left_right(Addr::LineStart(2), Addr::LineStart(2))
        .unwrap()
        .set(RopeSlice::from("CACA"))
        .unwrap();

    println!(
        "{}",
        buf.new_dot()
            .left_right(Addr::BufferStart, Addr::BufferEnd)
            .unwrap()
            .get()
            .unwrap()
    );
}
