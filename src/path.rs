use core::str::FromStr;

use alloc::{
    string::{String, ToString},
    vec::Vec,
};

use crate::{
    builder::StringPathBuilder,
    packed_list::{Node, PathSegmentList},
    parser,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Path {
    pub(crate) prefix: Option<Prefix>,
    pub(crate) drive: Option<Drive>,
    pub(crate) root: Option<Root>,
    pub(crate) segments: PathSegmentList,
}

impl Path {
    pub fn new() -> Self {
        Path {
            segments: PathSegmentList::new(),
            prefix: None,
            drive: None,
            root: None,
        }
    }

    pub fn builder(self) -> StringPathBuilder {
        StringPathBuilder::new(self)
    }

    pub fn has_root(&self) -> bool {
        self.root.is_some()
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
        self.has_root()
    }

    pub fn is_relative(&self) -> bool {
        !self.has_root()
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
            prefix: None,
            drive: None,
            root: None,
        })
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

    pub fn parent(mut self) -> Option<Path> {
        self.segments.remove_last()?;
        Some(self)
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
                // resolve home
                // remove ~
                // join home with self
                let home = dirs::home_dir().ok_or_else(|| "couldn't resolve home")?;
                let path = Path::from_str(
                    home.to_str()
                        .expect("home must be valid on its own operating system"),
                )?;

                let head = self.segments.head.expect("head exists as a ~");
                self.segments.remove(head);
                self = path.join(self);
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

impl FromStr for Path {
    type Err = &'static str;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        parser::parse_path(s)
    }
}

impl From<PathSegmentList> for Path {
    fn from(segments: PathSegmentList) -> Self {
        Path {
            segments,
            prefix: None,
            drive: None,
            root: None,
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
        }
    }
}

impl Default for Path {
    fn default() -> Self {
        Self::new()
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
        assert!(path.root.is_none());
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
            .with_resolver(true)
            .build();

        assert!(resolved.is_ok());
        let resolved = resolved.unwrap();
        assert_eq!(resolved, expected);
    }
}
