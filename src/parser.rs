use alloc::{string::String, vec::Vec};

use crate::{
    packed_list::PathSegmentList,
    path::{Drive, Path, Prefix, Root},
};

pub fn parse_path(input: &str) -> Result<Path, &'static str> {
    parse(Cursor::new(input.chars().collect()))
}

fn parse(mut cursor: Cursor) -> Result<Path, &'static str> {
    let prefix = parse_prefix(&mut cursor);
    let drive = parse_drive(&mut cursor);
    let root = parse_root(&mut cursor, &prefix)?;

    let segments = parse_segments(&mut cursor)?;

    Ok(Path {
        prefix,
        drive,
        root,
        segments,
    })
}

fn parse_prefix(cursor: &mut Cursor) -> Option<Prefix> {
    let mut clone = cursor.clone();
    let first = clone.eat();
    let second = clone.eat();
    let third = clone.eat();
    let fourth = clone.eat();

    match (first, second, third, fourth) {
        (Some('/'), Some('/'), Some('.'), Some('/')) => {
            cursor.eat();
            cursor.eat();
            cursor.eat();
            cursor.eat();
            Some(Prefix::Device)
        }
        (Some('/'), Some('/'), Some('?'), Some('/')) => {
            cursor.eat();
            cursor.eat();
            cursor.eat();
            cursor.eat();
            Some(Prefix::ExtendedPath)
        }
        _ => None,
    }
}

fn parse_drive(cursor: &mut Cursor) -> Option<Drive> {
    match (cursor.first(), cursor.second()) {
        (Some(letter), Some(':')) if letter.is_alphabetic() => {
            cursor.eat();
            cursor.eat();
            Some(Drive { letter: letter })
        }
        _ => None,
    }
}

fn parse_root(cursor: &mut Cursor, prefix: &Option<Prefix>) -> Result<Option<Root>, &'static str> {
    if let Some(Prefix::ExtendedPath) = prefix {
        let mut clone = cursor.clone();
        let first = clone.eat();
        let second = clone.eat();
        let third = clone.eat();

        if let (Some('U'), Some('N'), Some('C')) = (first, second, third) {
            cursor.eat(); // consume U
            cursor.eat(); // consume N
            cursor.eat(); // consume C
            let slash = cursor.eat(); // consume '/' which must come here
            let Some('/') = slash else {
                return Err(
                    r"extended-length UNC paths must have a slash after the \\?\UNC prefix",
                );
            };

            return Ok(Some(Root::Unc));
        }
    }

    if let Some('/') = cursor.first() {
        if let Some('/') = cursor.second() {
            cursor.eat(); // consume the first '/'
            cursor.eat(); // consume the second '/'
            return Ok(Some(Root::Unc));
        } else {
            cursor.eat(); // consume the '/'
            return Ok(Some(Root::Normal));
        }
    }

    Ok(None)
}

fn parse_segments(cursor: &mut Cursor) -> Result<PathSegmentList, &'static str> {
    let mut segments = Vec::new();

    while cursor.first().is_some() {
        let mut segment = String::new();

        while let Some(next) = cursor.eat() {
            if next == '\\' || next == '/' {
                if segment.is_empty() {
                    return Err("path segments cannot be empty");
                }

                break;
            }

            segment.push(next);
        }

        segments.push(segment);
    }

    Ok(segments.into_iter().collect())
}

#[derive(Debug, Clone)]
struct Cursor {
    chars: Vec<char>,
}

impl Cursor {
    pub fn new(chars: Vec<char>) -> Self {
        Self {
            chars: chars
                .into_iter()
                .map(|c| if c == '\\' { '/' } else { c })
                .collect(),
        }
    }

    pub fn eat(&mut self) -> Option<char> {
        if self.chars.len() > 0 {
            Some(self.chars.remove(0))
        } else {
            None
        }
    }

    pub fn first(&self) -> Option<char> {
        self.chars.first().cloned()
    }

    pub fn second(&self) -> Option<char> {
        self.chars.get(1).cloned()
    }
}
