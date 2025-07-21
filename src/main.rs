use big::text::{Addr, Buffer, Dot};
use std::path::Path;

fn main() {
    let buf = Buffer::from_file(Path::new("test.txt")).unwrap();
    let mut dot = Dot::new(&buf);
    dot.anchor_left(Addr::LineStart(1), Addr::Index(5)).unwrap();
    println!("{}", buf.get(&dot).unwrap());
}
