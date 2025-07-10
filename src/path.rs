use core::str::FromStr;

use alloc::{
    string::{String, ToString},
    vec::Vec,
};

use crate::{
    builder::StringPathBuilder,
    packed_list::{Node, PathSegmentList},
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Path {
    segments: PathSegmentList,
    has_root: bool,
}

#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub(crate) struct PathSegment(pub(crate) String);

impl Path {
    pub fn new() -> Self {
        Path {
            segments: PathSegmentList::new(),
            has_root: false,
        }
    }

    pub fn builder(self) -> StringPathBuilder {
        StringPathBuilder::new(self)
    }

    pub fn has_root(&self) -> bool {
        self.has_root
    }

    pub fn join(mut self, path: impl Into<Path>) -> Self {
        for segment in path.into().segments.into_iter() {
            self.segments.push(segment);
        }

        self
    }

    pub fn root(&self) -> Option<Self> {
        todo!()
    }

    pub fn dirname(&self) -> Option<&str> {
        let mut components = self.components();
        components.pop()?;
        components.pop()
    }

    pub fn exists(&self) -> bool {
        todo!()
    }

    pub fn is_absolute(&self) -> bool {
        self.has_root
    }

    pub fn is_relative(&self) -> bool {
        !self.has_root
    }

    pub fn basename(&self) -> Option<&str> {
        self.components().pop()
    }

    pub fn stem(&self) -> Option<&str> {
        let basename = self.basename()?;

        if basename.is_empty() {
            return None;
        }

        let last = basename.rfind('.').unwrap_or(0);

        if last == 0 || last == basename.len() - 1 {
            return Some(basename);
        }

        Some(&basename[..last])
    }

    pub fn relative_to(&self, base: &Path) -> Option<Self> {
        if self.is_absolute() && base.is_absolute() && self.root() != base.root() {
            return None;
        }

        let mut segments = self.segments.clone();
        let mut base_segments = base.segments.clone();

        // Remove common prefix
        while segments.head.is_some() && base_segments.head.is_some() {
            let segment = &segments[segments.head.unwrap()].value;
            let base_segment = &base_segments[base_segments.head.unwrap()].value;

            if segment != base_segment {
                break;
            }

            segments.remove(segments.head.unwrap());
            base_segments.remove(base_segments.head.unwrap());
        }

        let len = base_segments.len();

        if len == base.segments.len() {
            return None;
        }

        for _ in 0..len {
            segments.push_start("..".to_string());
        }

        Some(Path {
            segments,
            has_root: false,
        })
    }

    pub fn extension(&self) -> Option<&str> {
        let mut basename = self.basename()?;

        if basename.is_empty() {
            return None;
        }

        let last = basename.rfind('.').unwrap_or(0);

        if last == 0 || last == basename.len() - 1 {
            return None;
        }

        Some(&basename[last + 1..])
    }

    pub fn components(&self) -> Vec<&str> {
        self.segments.iter().map(|p| p.0.as_str()).collect()
    }

    pub fn parent(mut self) -> Option<Path> {
        self.segments.remove_last()?;
        Some(self)
    }

    pub fn resolve(mut self) -> Self {
        let Some(head_index) = self.segments.head else {
            return self;
        };

        fn traverse(mut path: PathSegmentList, index: usize) -> PathSegmentList {
            let node = &path[index];
            let value = &node.value;
            let prev = node.prev;
            let next = node.next;

            if value.0 == "." {
                path.remove(index);
            } else if value.0 == ".." {
                let Some(prev) = prev else {
                    let Some(next) = next else {
                        return path;
                    };

                    return traverse(path, next);
                };

                let prev_node = &mut path[prev];

                let Some(prev_prev) = prev_node.prev else {
                    if prev_node.value.0 != ".." {
                        path.remove(prev);
                        path.remove(index);
                    }

                    let Some(next) = next else {
                        return path;
                    };

                    return traverse(path, next);
                };

                let prev_prev_node = &mut path[prev_prev];
                prev_prev_node.next = next;

                if let Some(next) = next {
                    let next_node = &mut path[next];
                    next_node.prev = Some(prev_prev);
                } else {
                    path.tail = Some(prev_prev);
                }

                path.free(prev);
                path.free(index);
            }

            let Some(next) = next else {
                return path;
            };

            traverse(path, next)
        }

        self.segments = traverse(self.segments, head_index);
        self
    }

    pub fn traverse_symlinks(self) -> Self {
        todo!()
    }

    pub fn is_windows_compatible(&self) -> bool {
        self.segments.iter().all(|s| s.is_windows_compatible())
    }

    pub fn is_unix_compatible(&self) -> bool {
        self.segments.iter().all(|s| s.is_unix_compatible())
    }
}

impl PathSegment {
    pub fn is_windows_compatible(&self) -> bool {
        const RESERVED_NAMES: [&str; 22] = [
            "CON", "PRN", "AUX", "NUL", "COM1", "COM2", "COM3", "COM4", "COM5", "COM6", "COM7",
            "COM8", "COM9", "LPT1", "LPT2", "LPT3", "LPT4", "LPT5", "LPT6", "LPT7", "LPT8", "LPT9",
        ];

        let segment = self.0.as_str();

        if segment.is_empty() {
            return false;
        }

        assert!(!segment.contains('/'));
        assert!(!segment.contains('\\'));

        for c in segment.chars() {
            let c_u32 = c as u32;
            if c_u32 < 0x20 || matches!(c, '<' | '>' | ':' | '"' | '|' | '?' | '*') {
                return false;
            }
        }

        if let Some(last) = segment.chars().rev().next() {
            if last == '.' || last == ' ' {
                return false;
            }
        }

        let name_end = segment.find('.').unwrap_or(segment.len());

        let mut is_reserved = false;
        for &reserved in RESERVED_NAMES.iter() {
            if segment.len() >= reserved.len() {
                let mut matches = true;

                for (i, rc) in reserved.chars().enumerate() {
                    let sc = segment.as_bytes()[i] as char;
                    if !rc.eq_ignore_ascii_case(&sc) {
                        matches = false;
                        break;
                    }
                }

                if matches && name_end == reserved.len() {
                    is_reserved = true;
                    break;
                }
            }
        }

        !is_reserved
    }

    pub fn is_unix_compatible(&self) -> bool {
        let segment = &self.0;

        if segment.is_empty() {
            return false;
        }

        assert!(!segment.contains('/'));

        for c in segment.chars() {
            if c == '\0' {
                return false;
            }
        }

        true
    }
}

impl FromStr for Path {
    type Err = &'static str;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if s.is_empty() {
            return Ok(Self::new());
        }

        let mut path = Self::new();

        for segment in s.replace(r"\", "/").split('/') {
            if !segment.is_empty() {
                path.segments.push(PathSegment::from_str(segment)?);
            } else {
                return Err("path segments cannot be empty");
            }
        }

        Ok(path)
    }
}

impl From<PathSegmentList> for Path {
    fn from(segments: PathSegmentList) -> Self {
        Path {
            segments,
            has_root: false,
        }
    }
}

impl<P: Into<PathSegment>> From<P> for Path {
    fn from(segment: P) -> Self {
        Path {
            segments: PathSegmentList::from(segment),
            has_root: false,
        }
    }
}

impl Default for Path {
    fn default() -> Self {
        Self::new()
    }
}

impl From<String> for PathSegment {
    fn from(segment: String) -> Self {
        PathSegment(segment)
    }
}

impl FromStr for PathSegment {
    type Err = &'static str;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(Self(s.into()))
    }
}

#[cfg(test)]
mod test {
    use core::str::FromStr;

    use rstest::rstest;

    use super::*;

    #[rstest]
    fn join() {
        // arrange
        let path1 = Path::from_str("a/b/c").unwrap();
        let path2 = Path::from_str("d/e").unwrap();

        // act
        let joined_path = path1.join(path2);

        // assert
        assert_eq!(joined_path.segments.len(), 5);
        let expected = Path::from_str("a/b/c/d/e").unwrap();
        assert_eq!(joined_path, expected);
    }

    #[rstest]
    fn join2() {
        // arrange
        let path1 = Path::from_str("a/b/c").unwrap();
        let path2 = Path::from_str("").unwrap();

        // act
        let joined_path = path1.join(path2);

        // assert
        assert_eq!(joined_path.segments.len(), 3);
        let expected = Path::from_str("a/b/c").unwrap();
        assert_eq!(joined_path, expected);
    }

    #[rstest]
    fn join3() {
        // arrange
        let path1 = Path::from_str("").unwrap();
        let path2 = Path::from_str("d/e").unwrap();

        // act
        let joined_path = path1.join(path2);

        // assert
        assert_eq!(joined_path.segments.len(), 2);
        let expected = Path::from_str("d/e").unwrap();
        assert_eq!(joined_path, expected);
    }

    #[rstest]
    fn dirname() {
        // arrange
        let path = Path::from_str("a/b/c").unwrap();

        // act
        let dirname = path.dirname();

        // assert
        assert_eq!(dirname, Some("b"));
    }

    #[rstest]
    #[case("a/b/c", Some("c"))]
    #[case("a/b/.c", Some(".c"))]
    #[case("a/b/.c", Some(".c"))]
    #[case("a/b/c.", Some("c."))]
    #[case("a/b/.c.", Some(".c."))]
    #[case("a/b/c.d", Some("c.d"))]
    #[case("a/b/.c.d", Some(".c.d"))]
    #[case("a/b/c.d.", Some("c.d."))]
    #[case("a/b/c.d.e", Some("c.d.e"))]
    fn basename(#[case] str: &str, #[case] expected: Option<&str>) {
        // arrange
        let path = Path::from_str(str).unwrap();

        // act
        let basename = path.basename();

        // assert
        assert_eq!(basename, expected);
    }

    #[rstest]
    #[case("a/b/c", Some("c"))]
    #[case("a/b/.c", Some(".c"))]
    #[case("a/b/c.", Some("c."))]
    #[case("a/b/.c.", Some(".c."))]
    #[case("a/b/c.d", Some("c"))]
    #[case("a/b/.c.d", Some(".c"))]
    #[case("a/b/c.d.", Some("c.d."))]
    #[case("a/b/c.d.e", Some("c.d"))]
    fn stem(#[case] str: &str, #[case] expected: Option<&str>) {
        // arrange
        let path = Path::from_str(str).unwrap();

        // act
        let stem = path.stem();

        // assert
        assert_eq!(stem, expected);
    }

    #[rstest]
    #[case("a/b/c", None)]
    #[case("a/b/.c", None)]
    #[case("a/b/c.", None)]
    #[case("a/b/.c.", None)]
    #[case("a/b/c.d", Some("d"))]
    #[case("a/b/.c.d", Some("d"))]
    #[case("a/b/c.d.", None)]
    #[case("a/b/c.d.e", Some("e"))]
    fn extension(#[case] str: &str, #[case] expected: Option<&str>) {
        // arrange
        let path = Path::from_str(str).unwrap();

        // act
        let extension = path.extension();

        // assert
        assert_eq!(extension, expected);
    }

    #[rstest]
    fn parent() {
        // arrange
        let path = Path::from_str("a/b/c").unwrap();

        // act
        let dirname = path.parent();

        // assert
        let expected_parent = Path::from_str("a/b").unwrap();
        assert_eq!(dirname, Some(expected_parent));
    }

    #[rstest]
    #[case("a/b/c", "b/b/c", None)]
    #[case("a/b/c", "a/b", Some("c"))]
    #[case("a/b/c", "a/b/d", Some("../c"))]
    #[case("a/b/c", "a/d/e", Some("../../b/c"))]
    #[case("a/b/.c", "a/b", Some(".c"))]
    #[case("a/b/.c", "a/b/d", Some("../.c"))]
    #[case("a/b/c.d", "a/b/d", Some("../c.d"))]
    #[case("a/b/c.d", "a/b.d", Some("../b/c.d"))]
    fn relative_to(#[case] path: &str, #[case] base: &str, #[case] expected: Option<&str>) {
        // arrange
        let path = Path::from_str(path).unwrap();
        let base = Path::from_str(base).unwrap();

        // act
        let relative = path.relative_to(&base);

        // assert
        let expected = expected.map(|e| Path::from_str(e).unwrap());
        assert_eq!(relative, expected);

        let relative_str = relative.map(|p| p.builder().with_resolver(true).build());
        let expected_str = expected.map(|p| p.builder().with_resolver(true).build());
        assert_eq!(relative_str, expected_str);
    }

    #[rstest]
    #[case("a", true)]
    #[case(".a", true)]
    #[case("a.", false)]
    #[case("a b", true)]
    #[case("a ", false)]
    #[case("a:", false)]
    #[case(":a", false)]
    #[case("a>", false)]
    #[case("a<", false)]
    #[case("a\"", false)]
    #[case("a|", false)]
    #[case("a?", false)]
    #[case("a*", false)]
    #[case("a\0", false)]
    fn is_windows_compatible(#[case] path: &str, #[case] expected: bool) {
        // arrange
        let path = Path::from_str(path).unwrap();

        // act
        let compatible = path.is_windows_compatible();

        // assert
        assert_eq!(compatible, expected);
    }

    #[rstest]
    #[case("a", true)]
    #[case(".a", true)]
    #[case("a.", true)]
    #[case("a b", true)]
    #[case("a ", true)]
    #[case("a:", true)]
    #[case(":a", true)]
    #[case("a>", true)]
    #[case("a<", true)]
    #[case("a\"", true)]
    #[case("a|", true)]
    #[case("a?", true)]
    #[case("a*", true)]
    #[case("a\0", false)]
    fn is_unix_compatible(#[case] path: &str, #[case] expected: bool) {
        // arrange
        let path = Path::from_str(path).unwrap();

        // act
        let compatible = path.is_unix_compatible();

        // assert
        assert_eq!(compatible, expected);
    }

    #[rstest]
    #[case("a//")]
    #[case(r"a\\")]
    fn path_from_str(#[case] path: &str) {
        // act
        let path = Path::from_str(path);

        // assert
        assert!(path.is_err());
        assert_eq!(path.unwrap_err(), "path segments cannot be empty");
    }
}
