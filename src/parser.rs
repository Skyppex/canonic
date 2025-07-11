use core::str::{Chars, FromStr};

use alloc::{
    string::{String, ToString},
    vec::Vec,
};

use crate::{
    packed_list::PathSegmentList,
    path::{Path, PathSegment, Root},
};

pub fn parse_path(input: &str) -> Result<Path, &'static str> {
    if input.is_empty() {
        return Ok(Path::new());
    }

    let mut segments = Vec::new();

    for segment in input.replace(r"\", "/").split('/') {
        if !segment.is_empty() {
            segments.push(PathSegment::from_str(segment)?);
        } else {
            return Err("path segments cannot be empty");
        }
    }

    Ok(Path::from(
        segments.into_iter().collect::<PathSegmentList>(),
    ))
}

fn parse(mut cursor: Cursor) -> Result<Path, &'static str> {
    let root = parse_root(cursor)?;

    todo!()
}

fn parse_root(mut cursor: Cursor) -> Result<Option<Root>, &'static str> {
    let Some(next) = cursor.first() else {
        return Ok(None);
    };

    match next {
        '/' => {
            if let Some('/') = cursor.second() {
                todo!()
            } else {
                cursor.eat(); // consume the '/'
                Ok(Some(Root {
                    string: String::from("/"),
                }))
            }
        }
        'a'..'z' | 'A'..='Z' => {
            let mut segment = String::new();
            while let Some(c) = cursor.eat() {
                if c.is_alphanumeric() || c == '_' {
                    segment.push(c);
                } else {
                    break;
                }
            }
            if segment.is_empty() {
                return Err("invalid path segment");
            }
            Ok(Some(segment))
        }
        other => Ok(None),
    }
}

struct Cursor {
    chars: Vec<char>,
}

impl Cursor {
    pub fn new(chars: Vec<char>) -> Self {
        Self { chars }
    }

    pub fn eat(&mut self) -> Option<char> {
        self.chars.pop()
    }

    pub fn first(&self) -> Option<char> {
        self.chars.first().cloned()
    }

    pub fn second(&self) -> Option<char> {
        self.chars.get(1).cloned()
    }
}
