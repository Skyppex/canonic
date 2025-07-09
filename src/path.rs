use core::str::FromStr;

use alloc::{string::String, vec::Vec};

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
                    path.remove(prev);
                    path.remove(index);

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
}
