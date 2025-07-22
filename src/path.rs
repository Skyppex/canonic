use core::str::FromStr;
#[cfg(feature = "std")]
use std::ffi::{OsStr, OsString};

use alloc::{
    string::{String, ToString},
    vec::Vec,
};

use crate::{
    builder::{Base, StringPathBuilder},
    packed_list::{Node, PathSegmentList},
    parser,
    zip_greedy::zip_greedy,
};

#[derive(Debug, Clone, Eq)]
pub struct Path {
    pub(crate) prefix: Option<Prefix>,
    pub(crate) drive: Option<Drive>,
    pub(crate) root: Option<Root>,
    pub(crate) segments: PathSegmentList,
    pub(crate) is_dir: bool,
}

impl PartialEq for Path {
    fn eq(&self, other: &Self) -> bool {
        self.prefix == other.prefix
            && self.drive == other.drive
            && self.root == other.root
            && self.segments == other.segments
            && self.is_dir() == other.is_dir()
    }
}

impl Path {
    pub fn new() -> Self {
        Path {
            segments: PathSegmentList::new(),
            prefix: None,
            drive: None,
            root: None,
            is_dir: false,
        }
    }

    #[allow(private_interfaces)]
    pub fn builder(self) -> StringPathBuilder<Base> {
        StringPathBuilder::new(self)
    }

    pub fn has_root(&self) -> bool {
        self.root.is_some()
    }

    pub fn join(&self, path: impl AsRef<Path>) -> Result<Self, &'static str> {
        let mut path = path.as_ref().clone();

        if path.is_absolute() {
            if self.drive.is_some()
                && self.root.is_none()
                && self.prefix.is_none()
                && self.segments.len() == 0
            {
                let mut path = path.clone();
                path.drive = self.drive.clone();
                return Ok(path.clone());
            }

            return Ok(path.clone());
        }

        let mut result = self.clone();

        match (&self.drive, &path.drive) {
            (
                Some(Drive {
                    letter: self_letter,
                }),
                Some(Drive {
                    letter: path_letter,
                }),
            ) => {
                if *self_letter != *path_letter {
                    return Err("cannot join two paths from different drives");
                }
            }
            (None, Some(path_drive)) => result.drive = Some(path_drive.clone()),
            _ => {}
        }

        if self.is_file() && path.segments.head().is_some_and(|h| h.value.0 == ".") {
            result = result.parent().expect("file must have a parent");
            path.segments.remove(0);
        }

        result.is_dir = if path.segments.len() > 0 || path.is_root() {
            path.is_dir
        } else {
            self.is_dir
        };

        for segment in path.segments.into_iter() {
            result.segments.push(segment);
        }

        Ok(result)
    }

    pub fn with_basename(&self, basename: impl AsRef<str>) -> Result<Self, &'static str> {
        let basename = basename.as_ref();
        let mut result = self.clone().resolve()?;

        if result.segments.remove_last().is_some() {
            if result.segments.len() > 0 {
                result.is_dir = true;
            }
        }

        let path = Path::from_str(basename)?.resolve()?;
        result.join(path)
    }

    pub fn root(&self) -> Option<Self> {
        if !self.has_root() {
            None
        } else {
            let mut clone = self.clone();
            clone.segments = PathSegmentList::new();
            Some(clone)
        }
    }

    pub fn dirname(&self) -> Option<&str> {
        let mut components = self.components();
        components.pop()?;
        components.pop()
    }

    #[cfg(feature = "std")]
    pub fn exists(&self) -> bool {
        self.to_std_path().exists()
    }

    pub fn is_absolute(&self) -> bool {
        self.has_root()
    }

    pub fn is_relative(&self) -> bool {
        !self.has_root()
    }

    pub fn is_file(&self) -> bool {
        !self.is_root() && !self.is_dir
    }

    pub fn is_dir(&self) -> bool {
        self.is_root() || self.is_dir
    }

    #[cfg(feature = "std")]
    pub fn is_canonic_file(&self) -> bool {
        self.to_std_path().is_file()
    }

    #[cfg(feature = "std")]
    pub fn is_canonic_dir(&self) -> bool {
        self.to_std_path().is_dir()
    }

    #[cfg(feature = "std")]
    pub fn is_canonic_symlink(&self) -> bool {
        self.to_std_path().is_symlink()
    }

    pub fn is_root(&self) -> bool {
        self.has_root() && self.segments.len() == 0
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

    #[cfg(feature = "std")]
    pub fn with_cwd_base(&self) -> Result<Self, &'static str> {
        let cwd = std::env::current_dir().map_err(|_| "failed to get cwd")?;
        Path::try_from(cwd)?.join(self)
    }

    pub fn extension(&self) -> Option<&str> {
        let basename = self.basename()?;

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

    pub fn parent(&self) -> Option<Path> {
        let mut parent = self.clone();
        parent.segments.remove_last()?;
        parent.is_dir = true;

        if !parent.has_root() && parent.segments.len() == 0 {
            parent.segments.push(".".to_string());
        }

        Some(parent)
    }

    pub fn resolve(mut self) -> Result<Self, &'static str> {
        let Some(head_index) = self.segments.head else {
            return Ok(self);
        };

        if let Some(Node {
            value: PathSegment(s),
            ..
        }) = self.segments.head()
        {
            if s == "~" {
                #[cfg(feature = "std")]
                {
                    let home = dirs::home_dir().ok_or_else(|| "couldn't resolve home")?;
                    let path = Path::from_str(
                        home.to_str()
                            .expect("home must be valid on its own operating system"),
                    )?;

                    let head = self.segments.head.expect("head exists as a ~");
                    self.segments.remove(head);
                    self = path.join(self)?;
                }
            }
        }

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
        Ok(self)
    }

    pub fn resolve_at(&self, base: impl AsRef<Path>) -> Result<Self, &'static str> {
        self.join(base.as_ref())?.resolve()
    }

    #[cfg(feature = "std")]
    pub fn resolve_at_cwd(&self) -> Result<Self, &'static str> {
        self.with_cwd_base()?.resolve()
    }

    #[cfg(feature = "std")]
    pub fn traverse_symlinks(self) -> Result<Self, &'static str> {
        let path = Into::<std::path::PathBuf>::into(self)
            .canonicalize()
            .map_err(|_| "couldn't canonicalize path")?;

        TryFrom::<std::path::PathBuf>::try_from(path).map_err(|_| "hello")
    }

    pub fn is_windows_compatible(&self) -> bool {
        self.segments.iter().all(|s| s.is_windows_compatible())
    }

    pub fn is_unix_compatible(&self) -> bool {
        self.prefix.is_none()
            && self.root.as_ref().is_none_or(|r| r == &Root::Normal)
            && self.segments.iter().all(|s| s.is_unix_compatible())
    }

    #[cfg(feature = "std")]
    pub fn to_std_path(&self) -> std::path::PathBuf {
        self.into()
    }

    pub fn to_string(self) -> String {
        self.builder().build_string()
    }

    #[cfg(feature = "std")]
    pub fn to_os_string(self) -> OsString {
        self.builder().build_os_string()
    }

    pub fn diff(&self, path: impl AsRef<Path>) -> Option<Path> {
        let path = path.as_ref();

        if self.prefix != path.prefix {
            return None;
        }

        if self.drive != path.drive {
            return None;
        }

        if self.root != path.root {
            return None;
        }

        let mut zipped = zip_greedy(self.segments.iter(), path.segments.iter()).peekable();

        let Some((l, r)) = zipped.peek() else {
            return Some(Path {
                prefix: None,
                drive: self.drive.clone(),
                root: None,
                segments: PathSegmentList::new(),
                is_dir: false,
            });
        };

        if l != r && self.root.is_none() {
            return None;
        }

        if let (Some(PathSegment(l)), Some(PathSegment(r))) = (l, r) {
            if l == ".." || r == ".." {
                return None;
            }
        }

        loop {
            let Some((l, r)) = zipped.peek() else {
                break;
            };

            if l != r {
                break;
            }

            zipped.next();
        }

        let (li, ri): (Vec<_>, Vec<_>) = zipped.unzip();

        let mut segments = PathSegmentList::new();
        let mut is_dir = self.is_dir();

        if (path.is_file() && self == &path.parent().expect("file must have a parent"))
            || (self.is_dir() && self == path)
        {
            segments.push(".".to_string());
            is_dir = true;
        }

        let mut count = ri.iter().flatten().count();

        if path.is_file() && count != 0 {
            count -= 1;
        }

        for _ in 0..count {
            segments.push("..".to_string());
        }

        for s in li.iter().flatten() {
            segments.push((*s).clone());
        }

        Some(Path {
            prefix: None,
            drive: self.drive.clone(),
            root: None,
            segments,
            is_dir,
        })
    }
}

impl FromStr for Path {
    type Err = &'static str;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        parser::parse_path(s)
    }
}

#[cfg(feature = "std")]
impl TryFrom<&OsStr> for Path {
    type Error = &'static str;

    fn try_from(value: &OsStr) -> Result<Self, Self::Error> {
        let s = value.to_str().ok_or("Path must be valid UTF-8")?;
        Path::from_str(s)
    }
}

#[cfg(feature = "std")]
impl TryFrom<OsString> for Path {
    type Error = &'static str;

    fn try_from(value: OsString) -> Result<Self, Self::Error> {
        Path::try_from(value.as_os_str())
    }
}

impl From<PathSegmentList> for Path {
    fn from(segments: PathSegmentList) -> Self {
        Path {
            segments,
            prefix: None,
            drive: None,
            root: None,
            is_dir: false,
        }
    }
}

impl<P: Into<PathSegment>> From<P> for Path {
    fn from(segment: P) -> Self {
        Path {
            segments: PathSegmentList::from(segment),
            prefix: None,
            drive: None,
            root: None,
            is_dir: false,
        }
    }
}

impl Default for Path {
    fn default() -> Self {
        Self::new()
    }
}

impl AsRef<Path> for Path {
    fn as_ref(&self) -> &Path {
        self
    }
}

#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub(crate) struct PathSegment(pub(crate) String);

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

impl From<String> for PathSegment {
    fn from(segment: String) -> Self {
        PathSegment(segment)
    }
}

impl FromStr for PathSegment {
    type Err = &'static str;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if s.contains('/') || s.contains('\\') {
            return Err("path segment cannot contain path separators");
        }

        Ok(Self(s.into()))
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Prefix {
    ExtendedPath,
    Device,
}

#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub(crate) struct Drive {
    pub letter: char,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Root {
    Normal,
    Unc,
}

#[cfg(test)]
mod test {
    use core::str::FromStr;

    use rstest::rstest;

    use super::*;

    #[rstest]
    #[case("a/b/c", "d/e", "a/b/c/d/e")]
    #[case("a/b/c", "", "a/b/c")]
    #[case("", "d/e", "d/e")]
    #[case("/a", "/b/c", "/b/c")]
    #[case("c:/a", "b/c", "c:/a/b/c")]
    #[case("c:/a", "c:b/c", "c:/a/b/c")]
    #[case("c:a", "c:/b/c", "c:/b/c")]
    #[case("a", "c:/b/c", "c:/b/c")]
    #[case("c:a", "c:b/c", "c:a/b/c")]
    #[case("/a", "c:b/c", "c:/a/b/c")]
    #[case("/a", "c:/b/c", "c:/b/c")]
    #[case("a", "c:b/c", "c:a/b/c")]
    #[case("c:", "a", "c:a")]
    #[case("c:", "/a", "c:/a")]
    #[case("c:", "c:/a", "c:/a")]
    #[case("a", "c:", "c:a")]
    #[case("/a", "c:", "c:/a")]
    #[case("c:/a", "c:", "c:/a")]
    #[case("a/b/c", "./d/e", "a/b/d/e")]
    #[case("a/b/c/", "../d/e", "a/b/c/../d/e")]
    #[case("", "", "")]
    fn join(#[case] left: &str, #[case] right: &str, #[case] expected: &str) {
        // arrange
        let path1 = Path::from_str(left).unwrap();
        let path2 = Path::from_str(right).unwrap();

        // act
        let joined_path = path1.join(path2).unwrap();

        // assert
        let expected = Path::from_str(expected).unwrap();
        assert_eq!(joined_path, expected);
    }

    #[rstest]
    #[case("a/b/c", "d/e", "a/b/d/e")]
    #[case("a/b/c", "", "a/b/")]
    #[case("", "d/e", "d/e")]
    #[case("/a", "/b/c", "/b/c")]
    #[case("c:/a", "b/c", "c:/b/c")]
    #[case("c:/a", "c:b/c", "c:/b/c")]
    #[case("c:a", "c:/b/c", "c:/b/c")]
    #[case("a", "c:/b/c", "c:/b/c")]
    #[case("c:a", "c:b/c", "c:b/c")]
    #[case("/a", "c:b/c", "c:/b/c")]
    #[case("/a", "c:/b/c", "c:/b/c")]
    #[case("a", "c:b/c", "c:b/c")]
    #[case("c:", "a", "c:a")]
    #[case("c:", "/a", "c:/a")]
    #[case("c:", "c:/a", "c:/a")]
    #[case("a", "c:", "c:")]
    #[case("/a", "c:", "c:/")]
    #[case("c:/a", "c:", "c:/")]
    #[case("a/b/c", "./d/e", "a/b/d/e")]
    #[case("a/b/c/", "../d/e", "a/b/../d/e")]
    #[case("", "", "")]
    fn with_basename(#[case] left: &str, #[case] right: &str, #[case] expected: &str) {
        // arrange
        let path = Path::from_str(left).unwrap();

        // act
        let joined_path = path.with_basename(right).unwrap();

        // assert
        let expected = Path::from_str(expected).unwrap();
        assert_eq!(joined_path, expected);
    }

    #[rstest]
    #[case("c:/a", "d:b/c")]
    #[case("d:/a", "c:")]
    #[case("d:", "c:")]
    fn join_should_fail(#[case] left: &str, #[case] right: &str) {
        // arrange
        let path1 = Path::from_str(left).unwrap();
        let path2 = Path::from_str(right).unwrap();

        // act
        let joined_path = path1.join(path2);

        // assert
        assert!(joined_path.is_err());
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
    #[case("a/b", Some("a/"))]
    #[case("a/b/", Some("a/"))]
    #[case("/a/b", Some("/a/"))]
    #[case("/a/b/", Some("/a/"))]
    #[case("a", Some("./"))]
    fn parent(#[case] path: &str, #[case] expected: Option<&str>) {
        // arrange
        let path = Path::from_str(path).unwrap();

        // act
        let dirname = path.parent();

        // assert
        let expected = expected.map(|e| Path::from_str(e).unwrap());
        assert_eq!(dirname, expected);
    }

    #[rstest]
    #[case("", true)]
    #[case("/", false)]
    #[case("/.", false)]
    #[case("/./", false)]
    #[case("/..", false)]
    #[case("/../", false)]
    #[case("file", true)]
    #[case("file/", false)]
    #[case(".file", true)]
    #[case(".file/", false)]
    #[case("dir/file", true)]
    #[case("dir/file/", false)]
    #[case("dir/", false)]
    #[case("//", false)]
    #[case("a/b.c", true)]
    #[case("a/b.c/", false)]
    #[case("a.b/c", true)]
    #[case("a.b/c/", false)]
    #[case("a/", false)]
    #[case("a", true)]
    #[case("a.b/", false)]
    #[case("C:", true)]
    #[case("C:/", false)]
    #[case("C:/Users", true)]
    #[case("C:/Users/", false)]
    #[case("C:/file.txt", true)]
    #[case("C:/file.txt/", false)]
    #[case(r"\\", false)]
    #[case(r"\\.", false)]
    #[case(r"\\.\", false)]
    #[case(r"\\?\UNC\", false)]
    #[case(r"\\?\C:\", false)]
    #[case(r"\\Server\Share", true)]
    #[case(r"\\Server\Share\", false)]
    #[case(r"\\Server\Share\foo", true)]
    #[case(r"\\Server\Share\foo\", false)]
    #[case(r"\\?\C:\foo", true)]
    #[case(r"\\?\C:\foo\", false)]
    #[case(r"\\.\COM1", true)]
    #[case(r"\\.\COM1\", false)]
    #[case("~/file.txt", true)]
    #[case("~/file.txt/", false)]
    #[case("~/dir/", false)]
    #[case("~/dir", true)]
    #[case("//?/UNC/server/share/file", true)]
    #[case("//?/UNC/server/share/file/", false)]
    fn is_file(#[case] path: &str, #[case] expected: bool) {
        // arrange
        let path = Path::from_str(path).unwrap();

        // act
        let result = path.is_file();

        // assert
        assert_eq!(result, expected);
    }

    #[rstest]
    #[case("", false)]
    #[case("/", true)]
    #[case("/.", true)]
    #[case("/./", true)]
    #[case("/..", true)]
    #[case("/../", true)]
    #[case("file", false)]
    #[case("file/", true)]
    #[case(".file", false)]
    #[case(".file/", true)]
    #[case("dir/file", false)]
    #[case("dir/file/", true)]
    #[case("dir/", true)]
    #[case("//", true)]
    #[case("a/b.c", false)]
    #[case("a/b.c/", true)]
    #[case("a.b/c", false)]
    #[case("a.b/c/", true)]
    #[case("a/", true)]
    #[case("a", false)]
    #[case("a.b/", true)]
    #[case("C:", false)]
    #[case("C:/", true)]
    #[case("C:/Users", false)]
    #[case("C:/Users/", true)]
    #[case("C:/file.txt", false)]
    #[case("C:/file.txt/", true)]
    #[case(r"\\", true)]
    #[case(r"\\.", true)]
    #[case(r"\\.\", true)]
    #[case(r"\\?\UNC\", true)]
    #[case(r"\\?\C:\", true)]
    #[case(r"\\Server\Share", false)]
    #[case(r"\\Server\Share\", true)]
    #[case(r"\\Server\Share\foo", false)]
    #[case(r"\\Server\Share\foo\", true)]
    #[case(r"\\?\C:\foo", false)]
    #[case(r"\\?\C:\foo\", true)]
    #[case(r"\\.\COM1", false)]
    #[case(r"\\.\COM1\", true)]
    #[case("~/file.txt", false)]
    #[case("~/file.txt/", true)]
    #[case("~/dir/", true)]
    #[case("~/dir", false)]
    #[case("//?/UNC/server/share/file", false)]
    #[case("//?/UNC/server/share/file/", true)]
    fn is_dir(#[case] path: &str, #[case] expected: bool) {
        // arrange
        let path = Path::from_str(path).unwrap();

        // act
        let result = path.is_dir();

        // assert
        assert_eq!(result, expected);
    }

    #[rstest]
    #[case("a/b/c/", "a/d/e/", Some("../../b/c/"))]
    #[case("a/b/", "a/d/e/", Some("../../b/"))]
    #[case("a/b/c/", "a/d/", Some("../b/c/"))]
    #[case("c:/a/b/c/", "c:/a/d/", Some("c:../b/c/"))]
    #[case("c:a/b/c/", "c:a/d/", Some("c:../b/c/"))]
    #[case(r"\\.\a/b/c/", r"\\.\a/d/", Some("../b/c/"))]
    #[case(r"\\?\c:\a\b\c\", r"\\?\c:\a\d\", Some("c:../b/c/"))]
    #[case("c:/a/b/c", "c:a/d", None)]
    #[case("c:a/b/c", "c:/a/d", None)]
    #[case("a/b/c", "a/b/c", Some(""))]
    #[case("a/b/", "a/b/c", Some("."))]
    #[case("a/b/c", "a/b/", Some("c"))]
    #[case("a/b/", "a/b/c/", Some(".."))]
    #[case("/a/b/c/", "/a/b/", Some("c/"))]
    #[case("/a/b/", "/a/b/c/", Some(".."))]
    #[case("/a/b/c/", "/a/d/e/", Some("../../b/c/"))]
    #[case("/a/b/c/", "a/b/c/", None)]
    #[case("a/b/c/", "/a/b/c/", None)]
    #[case("c:/a/b/c/", "c:/a/b/", Some("c:c/"))]
    #[case("c:/a/b/", "c:/a/b/c/", Some("c:../"))]
    #[case("c:/a/b/c", "d:/a/b", None)]
    #[case("c:/a/b", "c:a/b", None)]
    #[case("c:a/b/c/", "c:a/b/", Some("c:c/"))]
    #[case("c:a/b/", "c:a/b/c/", Some("c:.."))]
    #[case("~/a/b/", "~/a/c/", Some("../b/"))]
    #[case("~/a/b", "/a/b", None)]
    #[case("~/a/b", "a/b", None)]
    #[case("../a/b", "../a/c", None)]
    #[case("./a/b", "./a/c", Some("b"))]
    #[case("/a/b/c/", "/a/b/c/d/e/", Some("../../"))]
    #[case("/a/b/c/d/e", "/a/b/c/", Some("d/e"))]
    #[case(r"\\?\c:\a\b\c\", r"\\?\c:\a\d\", Some("c:../b/c/"))]
    #[case(r"\\.\a\b\c\", r"\\.\a\d\", Some("../b/c/"))]
    #[case(r"\\?\UNC\server\share\a\", r"\\?\UNC\server\share\b\", Some("../a/"))]
    #[case(r"\\.\C:\dir\file.txt", r"\\.\C:\dir\other.txt", Some("C:file.txt"))]
    #[case(r"\\.\C:\dir\file.txt", r"\\.\D:\dir\file.txt", None)]
    #[case(r"\\?\C:\foo\bar\", r"\\?\C:\foo\bar\baz\", Some("C:.."))]
    #[case(r"\\?\C:\foo\bar\baz", r"\\?\C:\foo\bar", Some("C:baz"))]
    #[case(r"\\?\C:/foo/bar", r"\\?\C:/foo/baz", Some("C:bar"))]
    #[case(r"\\?\C:/foo/bar", r"\\?\D:/foo/bar", None)]
    #[case(r"c:/Users/Alice", r"c:/Users/Bob", Some("c:Alice"))]
    #[case(r"c:/a/b/c/", r"c:/a/b/c/Projects/", Some("c:../"))]
    #[case(r"//Server/Share/a", r"//Server/Share/b", Some("a"))]
    #[case(r"//Server/Share/a/b", r"//Server/Share/c/d", Some("../a/b"))]
    #[case(r"\\?\UNC\srv\f\", r"\\?\UNC\srv\f\file.txt", Some("."))]
    #[case(r"\\?\UNC\srv\s\f\", r"\\?\UNC\srv2\s\f\", Some("../../../srv/s/f/"))]
    #[case(r"~/projects/foo", r"~/projects/bar", Some("foo"))]
    #[case(r"~/projects/foo", r"/home/alice/projects/foo", None)]
    #[case(r"/tmp/foo/", r"/tmp/bar/", Some("../foo/"))]
    #[case(r"/tmp/foo", r"/var/tmp/foo", Some("../../tmp/foo"))]
    #[case("../foo/bar", "./foo/bar", None)]
    #[case("a/b/c", "..", None)]
    #[case("../a/b", "../../a/b", None)]
    #[case(r"\\.\COM1", r"\\.\COM2", Some("COM1"))]
    #[case(r"\\.\COM1", r"\\.\COM1", Some(""))]
    #[case(r"\\.\Drive0", r"\\.\Drive1", Some("Drive0"))]
    #[case(r"C:/", r"C:/Users", Some("C:."))]
    #[case(r"C:/", r"C:/Users/", Some("C:../"))]
    #[case(r"C:/Users", r"C:/", Some("C:Users"))]
    #[case(r"C:/Users/", r"C:/", Some("C:Users/"))]
    #[case(r"C:foo/bar", r"C:foo/baz", Some("C:bar"))]
    #[case(r"C:foo/bar", r"D:foo/bar", None)]
    #[case(r"/", r"/usr/bin/", Some("../../"))]
    #[case(r"/usr/bin/", r"/", Some("usr/bin/"))]
    #[case(r"./foo/bar", r"foo/bar", None)]
    #[case(r"C:/Users/Alice", r"C:/Users/Alice", Some("C:"))]
    #[case(r"C:/Users/Alice/", r"C:/Users/Alice/", Some("C:./"))]
    #[case(r"C:/Users/Alice/Documents", r"C:/Users/Alice/Documents", Some("C:"))]
    #[case(r"\\?\D:\a", r"\\?\D:\a", Some("D:"))]
    #[case(r"//?/UNC/server/share/a/", r"//?/UNC/server/share/a/b/", Some(".."))]
    fn diff(#[case] left: &str, #[case] right: &str, #[case] expected: Option<&str>) {
        // arrange
        let left = Path::from_str(left).unwrap();
        let right = Path::from_str(right).unwrap();

        // act
        let diff = left.diff(&right);

        // assert
        let expected = expected.map(|e| Path::from_str(e).unwrap());
        assert_eq!(diff, expected);
    }

    #[rstest]
    #[case("a", true)]
    #[case(".a", true)]
    #[case("a.", false)]
    #[case("a b", true)]
    #[case("a ", false)]
    #[case("a:", true)] // becomes a drive
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

    // C:\Users\Alice\Documents\file.txt        drive - rooted path
    // D:\Projects\code.py                      drive - rooted path
    #[rstest]
    #[case(r"C:\Users\Alice\Documents\file.txt", 'C', 4)]
    #[case(r"D:\Projects\code.py", 'D', 2)]
    fn win_drive_rooted(#[case] path: &str, #[case] drive: char, #[case] len: usize) {
        // act
        let path = Path::from_str(path);

        // assert
        assert!(path.is_ok());
        let path = path.unwrap();
        assert!(path.prefix.is_none());
        assert!(path.drive.is_some());
        assert_eq!(path.drive.unwrap().letter, drive);
        assert!(path.root.is_some());
        assert_eq!(path.root.unwrap(), Root::Normal);
        assert_eq!(path.segments.len(), len);
    }

    // \Windows\System32\cmd.exe                rooted path
    #[rstest]
    #[case(r"\Windows\System32\cmd.exe", 3)]
    fn win_rooted(#[case] path: &str, #[case] len: usize) {
        // act
        let path = Path::from_str(path);

        // assert
        assert!(path.is_ok());
        let path = path.unwrap();
        assert!(path.prefix.is_none());
        assert!(path.drive.is_none());
        assert!(path.root.is_some());
        assert_eq!(path.root.unwrap(), Root::Normal);
        assert_eq!(path.segments.len(), len);
    }

    // ..\Documents\file.txt                    relative path
    // .\file.txt                               relative path
    // file.txt                                 relative path
    #[rstest]
    #[case(r"..\Documents\file.txt", 3)]
    #[case(r".\file.txt", 2)]
    #[case(r"file.txt", 1)]
    fn win_relative(#[case] path: &str, #[case] len: usize) {
        // act
        let path = Path::from_str(path);

        // assert
        assert!(path.is_ok());
        let path = path.unwrap();
        assert!(path.prefix.is_none());
        assert!(path.drive.is_none());
        assert!(path.root.is_none());
        assert_eq!(path.segments.len(), len);
    }

    // \\Server\Share\folder\file.txt           unc rooted path
    #[rstest]
    #[case(r"\\Server\Share\folder\file.txt", 4)]
    fn unc(#[case] path: &str, #[case] len: usize) {
        // act
        let path = Path::from_str(path);

        // assert
        assert!(path.is_ok());
        let path = path.unwrap();
        assert!(path.prefix.is_none());
        assert!(path.drive.is_none());
        assert!(path.root.is_some());
        assert_eq!(path.root.unwrap(), Root::Unc);
        assert_eq!(path.segments.len(), len);
    }

    // \\?\C:\Very\Long\Path\file.txt           extended-length - drive - rooted path
    #[rstest]
    #[case(r"\\?\C:\Very\Long\Path\file.txt", 'C', 4)]
    fn win_extended_length_drive_rooted(
        #[case] path: &str,
        #[case] drive: char,
        #[case] len: usize,
    ) {
        // act
        let path = Path::from_str(path);

        // assert
        assert!(path.is_ok());
        let path = path.unwrap();
        assert!(path.prefix.is_some());
        assert_eq!(path.prefix.unwrap(), Prefix::ExtendedPath);
        assert!(path.drive.is_some());
        assert_eq!(path.drive.unwrap().letter, drive);
        assert!(path.root.is_some());
        assert_eq!(path.root.unwrap(), Root::Normal);
        assert_eq!(path.segments.len(), len);
    }

    // \\.\C:\path\to\file.txt           device - drive - rooted path
    #[rstest]
    #[case(r"\\.\C:\path\to\file.txt", 'C', 3)]
    fn win_device_drive_rooted(#[case] path: &str, #[case] drive: char, #[case] len: usize) {
        // act
        let path = Path::from_str(path);

        // assert
        assert!(path.is_ok());
        let path = path.unwrap();
        assert!(path.prefix.is_some());
        assert_eq!(path.prefix.unwrap(), Prefix::Device);
        assert!(path.drive.is_some());
        assert_eq!(path.drive.unwrap().letter, drive);
        assert!(path.root.is_some());
        assert_eq!(path.root.unwrap(), Root::Normal);
        assert_eq!(path.segments.len(), len);
    }

    // \\?\UNC\server\store\very\long\path\file.txt           extended-length - drive - rooted path
    #[rstest]
    #[case(r"\\?\UNC\server\store\very\long\path\file.txt", 6)]
    fn win_extended_length_unc(#[case] path: &str, #[case] len: usize) {
        // act
        let path = Path::from_str(path);

        // assert
        assert!(path.is_ok());
        let path = path.unwrap();
        assert!(path.prefix.is_some());
        assert_eq!(path.prefix.unwrap(), Prefix::ExtendedPath);
        assert!(path.drive.is_none());
        assert!(path.root.is_some());
        assert_eq!(path.root.unwrap(), Root::Unc);
        assert_eq!(path.segments.len(), len);
    }

    // C:folder\file.txt                        drive - relative path
    #[rstest]
    #[case(r"C:folder\file.txt", 'C', 2)]
    fn win_drive_relative(#[case] path: &str, #[case] drive: char, #[case] len: usize) {
        // act
        let path = Path::from_str(path);

        // assert
        assert!(path.is_ok());
        let path = path.unwrap();
        assert!(path.prefix.is_none());
        assert!(path.drive.is_some());
        assert_eq!(path.drive.unwrap().letter, drive);
        assert!(path.root.is_none());
        assert_eq!(path.segments.len(), len);
    }

    // \\.\pipe\my-pipe                         device prefix - path to device
    // \\.\COM1                                 device prefix - path to device
    // \\.\PhysicalDrive0                       device prefix - path to device
    #[rstest]
    #[case(r"\\.\pipe\my-pipe", 2)]
    #[case(r"\\.\COM1", 1)]
    #[case(r"\\.\PhysicalDrive0", 1)]
    fn win_device(#[case] path: &str, #[case] len: usize) {
        // act
        let path = Path::from_str(path);

        // assert
        assert!(path.is_ok());
        let path = path.unwrap();
        assert!(path.prefix.is_some());
        assert_eq!(path.prefix.unwrap(), Prefix::Device);
        assert!(path.drive.is_none());
        assert!(path.root.is_some_and(|r| r == Root::Normal));
        assert_eq!(path.segments.len(), len);
    }

    // /home/alice/file.txt                     rooted path
    // /etc/hosts                               rooted path
    // /tmp/file.txt                            rooted path
    #[rstest]
    #[case(r"/home/alice/file.txt", 3)]
    #[case(r"/etc/hosts", 2)]
    #[case(r"/tmp/file.txt", 2)]
    fn rooted(#[case] path: &str, #[case] len: usize) {
        // act
        let path = Path::from_str(path);

        // assert
        assert!(path.is_ok());
        let path = path.unwrap();
        assert!(path.prefix.is_none());
        assert!(path.drive.is_none());
        assert!(path.root.is_some());
        assert_eq!(path.root.unwrap(), Root::Normal);
        assert_eq!(path.segments.len(), len);
    }

    // ./file.txt                               relative path
    // ../file.txt                              relative path
    // file.txt                                 relative path
    // ../../etc/passwd                         relative path
    #[rstest]
    #[case(r"./file.txt", 2)]
    #[case(r"../file.txt", 2)]
    #[case(r"file.txt", 1)]
    #[case(r"../../etc/passwd", 4)]
    fn relative(#[case] path: &str, #[case] len: usize) {
        // act
        let path = Path::from_str(path);

        // assert
        assert!(path.is_ok());
        let path = path.unwrap();
        assert!(path.prefix.is_none());
        assert!(path.drive.is_none());
        assert!(path.root.is_none());
        assert_eq!(path.segments.len(), len);
    }

    // ~/file.txt                               user-relative path
    #[rstest]
    #[case(r"~/file.txt", 2)]
    fn user_relative(#[case] path: &str, #[case] len: usize) {
        // act
        let path = Path::from_str(path);

        // assert
        assert!(path.is_ok());
        let path = path.unwrap();
        assert!(path.prefix.is_none());
        assert!(path.drive.is_none());
        assert!(path.root.is_none());
        assert_eq!(path.segments.len(), len);
    }

    #[cfg(feature = "std")]
    #[rstest]
    #[case(r"~/a/b/./c/../d/e.txt", r"~/a/b/d/e.txt")]
    #[case(r"\\.\path/to/../over\here", r"//./path/over/here")]
    #[case(r"//?/c:/path/./do spaces work?", r"//?/c:/path/do spaces work?")]
    #[case(
        r"\\?\UNC\server\store\..\files\file.txt",
        r"//?/UNC/server/files/file.txt"
    )]
    fn combine_them_all(#[case] path: &str, #[case] expected: &str) {
        let home = dirs::home_dir().unwrap();
        let home = home.to_str().unwrap();
        let expected = expected.replace("~", home);
        let path = Path::from_str(path);
        assert!(path.is_ok());
        let path = path.unwrap();

        let resolved = path
            .builder()
            .with_separator('/')
            .with_resolver()
            .build_string();

        assert!(resolved.is_ok());
        let resolved = resolved.unwrap();
        assert_eq!(resolved, expected);
    }

    #[cfg(feature = "std")]
    #[rstest]
    fn resolve_at_cwd() {
        let path = Path::from_str(".local/").unwrap();

        let resolved = path.resolve_at_cwd().unwrap();

        assert_eq!(
            resolved,
            Path::from_str("/home/brage/dev/code/canonic/.local/").unwrap()
        );
    }
}
