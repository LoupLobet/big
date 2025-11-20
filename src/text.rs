use ropey::{Rope, RopeSlice};
use std::cmp::Ordering;
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

impl<'a> Addr {
    pub fn as_index(&self, text: &'a Arc<Mutex<Rope>>) -> ropey::Result<usize> {
        match self {
            Addr::Index(idx) => Ok(*idx),
            Addr::Coordinates(line, column) => {
                let text = text.lock().unwrap();
                let idx = text.try_line_to_char(*line)?;
                Ok(idx + column)
            }
            Addr::LineStart(line) => text.lock().unwrap().try_line_to_char(*line),
            Addr::LineEnd(line) => {
                let text = text.lock().unwrap();
                Ok(text.try_line_to_char(*line)? + text.line(*line).len_chars())
            }
            Addr::BufferStart => Ok(0),
            Addr::BufferEnd => Ok(text.lock().unwrap().len_chars() - 1),
        }
    }

    pub fn as_coordinates(&self, text: &'a Arc<Mutex<Rope>>) -> ropey::Result<(usize, usize)> {
        match self {
            Addr::Index(idx) => {
                let text = text.lock().unwrap();
                let line = text.try_char_to_line(*idx)?;
                let column = idx - text.try_line_to_char(line)?;
                Ok((line, column))
            }
            Addr::Coordinates(line, column) => Ok((*line, *column)),
            Addr::LineStart(line) => Ok((*line, 0)),
            Addr::LineEnd(line) => Ok((*line, text.lock().unwrap().line(*line).len_chars() - 1)),
            Addr::BufferStart => Ok((0, 0)),
            Addr::BufferEnd => {
                let text = text.lock().unwrap();
                let line = text.try_char_to_line(text.len_chars() - 1)?;
                let column = text.len_chars() - 1 - text.try_line_to_char(line)?;
                Ok((line, column))
            }
        }
    }

    pub fn move_left(&mut self, text: &'a Arc<Mutex<Rope>>, n: usize) -> ropey::Result<()> {
        match self.as_index(text)?.cmp(&n) {
            Ordering::Less => Err(ropey::Error::CharIndexOutOfBounds(
                0,
                text.lock().unwrap().len_chars(),
            )),
            _ => {
                *self = Addr::Index(self.as_index(text)? - n);
                Ok(())
            }
        }
    }

    //
    //
    //
    //
    // NEED TO CALL MUTEX LOCK HIGHER IN THE CALL HIERARCHY (probably at dot level)
    //
    //
    //
    //

    pub fn move_right(&mut self, text: &'a Arc<Mutex<Rope>>, n: usize) -> ropey::Result<()> {
        let text = text.lock().unwrap();
        match (self.as_index(text)? + n).cmp(&text.len_chars()) {
            Ordering::Greater => Err(ropey::Error::CharIndexOutOfBounds(
                text.len_chars(),
                text.len_chars(),
            )),
            _ => {
                *self = Addr::Index(self.as_index(text)? + n);
                Ok(())
            }
        }
    }
}

#[derive(Clone)]
pub struct Dot<'a> {
    text: &'a Arc<Mutex<Rope>>,
    from: Addr,
    to: Addr,
}

impl<'a> Dot<'a> {
    pub fn new(buf: &'a Buffer) -> Dot<'a> {
        Dot {
            text: &buf.text,
            from: Addr::BufferStart,
            to: Addr::BufferEnd,
        }
    }
    pub fn left_right(&mut self, left: Addr, right: Addr) -> ropey::Result<()> {
        self.from = Addr::Index(left.as_index(self.text)?);
        self.to = Addr::Index(right.as_index(self.text)?);
        Ok(())
    }
    pub fn anchor_left(&mut self, anchor: Addr, to: Addr) -> ropey::Result<()> {
        self.from = Addr::Index(anchor.as_index(self.text)?);
        self.to = Addr::Index(anchor.as_index(self.text)? + to.as_index(self.text)?);
        Ok(())
    }

    pub fn anchor_right(&mut self, from: Addr, anchor: Addr) -> ropey::Result<()> {
        self.from = Addr::Index(anchor.as_index(self.text)? - from.as_index(self.text)?);
        self.to = Addr::Index(anchor.as_index(self.text)?);
        Ok(())
    }

    pub fn move_left(&mut self, n: usize) -> ropey::Result<()> {
        self.from.move_left(self.text, n)?;
        self.to.move_left(self.text, n)?;
        Ok(())
    }

    pub fn move_right(&mut self, n: usize) -> ropey::Result<()> {
        self.from.move_right(self.text, n)?;
        self.to.move_right(self.text, n)?;
        Ok(())
    }

    pub fn extend_left(&mut self, n: usize) -> ropey::Result<()> {
        self.to.move_left(self.text, n)?;
        Ok(())
    }

    pub fn extend_right(&mut self, n: usize) -> ropey::Result<()> {
        self.from.move_right(self.text, n)?;
        Ok(())
    }

    pub fn trim_left(&mut self, n: usize) -> ropey::Result<()> {
        self.to.move_right(self.text, n)?;
        if self.to.as_index(self.text)? < self.from.as_index(self.text)? {
            (self.from, self.to) = (self.to, self.from);
        }
        Ok(())
    }

    pub fn trim_right(&mut self, n: usize) -> ropey::Result<()> {
        self.from.move_left(self.text, n)?;
        if self.from.as_index(self.text)? > self.to.as_index(self.text)? {
            (self.from, self.to) = (self.to, self.from);
        }
        Ok(())
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

    pub fn from_file(path: &Path) -> io::Result<Self> {
        let file = File::open(path)?;
        let reader = BufReader::new(file);
        Self::from_reader(reader)
    }

    pub fn get(&self, dot: &Dot) -> ropey::Result<String> {
        let from = dot.from.as_index(&self.text)?;
        let to = dot.to.as_index(&self.text)?;

        let slice: String = dot.text.lock().unwrap().slice(from..to).chars().collect();
        Ok(slice)
    }

    pub fn set(&mut self, dot: &mut Dot, s: RopeSlice) -> ropey::Result<()> {
        let from = dot.from.as_index(&self.text)?;
        let to = dot.to.as_index(&self.text)?;

        let mut text = dot.text.lock().unwrap();
        text.remove(from..to);
        text.insert(from, &s.to_string());
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_buffer_from_file() {
        let buf = Buffer::from_file(Path::new("tests/test.txt")).unwrap();
        let mut dot = Dot::new(&buf);
        dot.left_right(Addr::BufferStart, Addr::BufferEnd).unwrap();
        assert_eq!(
            buf.get(&dot).unwrap(),
            String::from("Hello there !\nHow are you ?\nI test a text editor.")
        );
    }

    #[test]
    fn test_dot_anchor_left() {
        let buf = Buffer::from_file(Path::new("tests/test.txt")).unwrap();
        let mut dot = Dot::new(&buf);
        dot.anchor_left(Addr::BufferStart, Addr::Index(5)).unwrap();
        assert_eq!(buf.get(&dot).unwrap(), "Hello");
    }

    #[test]
    fn test_dot_anchor_right() {
        let buf = Buffer::from_file(Path::new("tests/test.txt")).unwrap();
        let mut dot = Dot::new(&buf);
        dot.anchor_right(Addr::Index(5), Addr::Index(5)).unwrap();
        assert_eq!(buf.get(&dot).unwrap(), "Hello");
    }

    #[test]
    fn test_dot_left_right() {
        let buf = Buffer::from_file(Path::new("tests/test.txt")).unwrap();
        let mut dot = Dot::new(&buf);
        dot.left_right(Addr::Index(5), Addr::Index(10)).unwrap();
        assert_eq!(buf.get(&dot).unwrap(), " ther");
    }

    #[test]
    fn test_dot_move_left() {
        let buf = Buffer::from_file(Path::new("tests/test.txt")).unwrap();
        let mut dot = Dot::new(&buf);

        // out of buffer case
        dot.anchor_left(Addr::BufferStart, Addr::Index(2)).unwrap();
        let ret = dot.move_left(1);
        println!("{:?}", ret);
        assert!(ret.is_err());
        // regular case
        dot.anchor_right(Addr::Index(7), Addr::BufferEnd).unwrap();
        dot.move_left(1).unwrap();
        assert_eq!(buf.get(&dot).unwrap(), " editor");
    }

    #[test]
    fn test_dot_move_right() {
        let buf = Buffer::from_file(Path::new("tests/test.txt")).unwrap();
        let mut dot = Dot::new(&buf);

        // out of buffer case
        dot.anchor_right(Addr::Index(1), Addr::BufferEnd).unwrap();
        let ret = dot.move_right(1);
        assert!(ret.is_err());
        // regular case
        dot.anchor_left(Addr::BufferStart, Addr::Index(5)).unwrap();
        dot.move_right(2).unwrap();
        assert_eq!(buf.get(&dot).unwrap(), "llo t");
    }

    // #[test]
    // fn test_dot_extend_left() {
    //     let buf = Buffer::from_file(Path::new("tests/test.txt")).unwrap();
    //     let mut dot = Dot::new(&buf);
    //     dot.anchor_right(Addr::Index(7), Addr::BufferEnd).unwrap();
    //     dot.extend_left(2).unwrap();
    //     assert_eq!(buf.get(&dot).unwrap(), "t editor.");
    // }

    // #[test]
    // fn test_dot_extend_right() {
    //     let buf = Buffer::from_file(Path::new("tests/test.txt")).unwrap();
    //     let mut dot = Dot::new(&buf);
    //     dot.anchor_right(Addr::Index(7), Addr::BufferEnd).unwrap();
    //     dot.move_right(22).unwrap();
    //     assert_eq!(buf.get(&dot).unwrap(), "e you ?");
    // }
}
