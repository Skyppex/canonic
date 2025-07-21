use core::marker::PhantomData;
#[cfg(feature = "std")]
use std::ffi::OsString;

use alloc::string::String;

use crate::path::{Drive, Path, Prefix, Root};

pub struct StringPathBuilder<T> {
    path: Path,
    separator: char,
    _phantom_data: PhantomData<T>,
}

pub enum Base {}
pub enum WithResolver {}
pub enum WithSymlinkTraversal {}
pub enum WithResolverAndSymlinkTraversal {}

impl StringPathBuilder<Base> {
    pub fn new(path: impl Into<Path>) -> Self {
        StringPathBuilder::<Base> {
            path: path.into(),
            separator: '/',
            _phantom_data: PhantomData,
        }
    }

    pub fn with_separator(mut self, separator: impl Into<char>) -> Self {
        self.separator = separator.into();
        self
    }

    pub fn with_resolver(self) -> StringPathBuilder<WithResolver> {
        StringPathBuilder::<WithResolver> {
            path: self.path,
            separator: self.separator,
            _phantom_data: PhantomData,
        }
    }

    pub fn traverse_symlinks(self) -> StringPathBuilder<WithSymlinkTraversal> {
        StringPathBuilder::<WithSymlinkTraversal> {
            path: self.path,
            separator: self.separator,
            _phantom_data: PhantomData,
        }
    }

    pub fn build_string(self) -> String {
        build_path(self)
    }

    #[cfg(feature = "std")]
    pub fn build_os_string(self) -> OsString {
        OsString::from(self.build_string())
    }

    #[cfg(feature = "std")]
    pub fn build_std_path(self) -> std::path::PathBuf {
        std::path::PathBuf::from(self.build_string())
    }
}

impl StringPathBuilder<WithResolver> {
    pub fn traverse_symlinks(self) -> StringPathBuilder<WithResolverAndSymlinkTraversal> {
        StringPathBuilder::<WithResolverAndSymlinkTraversal> {
            path: self.path,
            separator: self.separator,
            _phantom_data: PhantomData,
        }
    }

    pub fn build_string(mut self) -> Result<String, &'static str> {
        self.path = self.path.resolve()?;
        Ok(build_path(self))
    }

    #[cfg(feature = "std")]
    pub fn build_os_string(self) -> Result<OsString, &'static str> {
        self.build_string().map(|s| OsString::from(s))
    }

    #[cfg(feature = "std")]
    pub fn build_std_path(self) -> Result<std::path::PathBuf, &'static str> {
        self.build_string().map(|s| std::path::PathBuf::from(s))
    }
}

#[cfg(feature = "std")]
impl StringPathBuilder<WithSymlinkTraversal> {
    pub fn with_resolver(self) -> StringPathBuilder<WithResolverAndSymlinkTraversal> {
        StringPathBuilder::<WithResolverAndSymlinkTraversal> {
            path: self.path,
            separator: self.separator,
            _phantom_data: PhantomData,
        }
    }

    pub fn build_string(mut self) -> Result<String, &'static str> {
        self.path = self.path.traverse_symlinks()?;
        Ok(build_path(self))
    }

    pub fn build_os_string(self) -> Result<OsString, &'static str> {
        self.build_string().map(|s| OsString::from(s))
    }

    pub fn build_std_path(self) -> Result<std::path::PathBuf, &'static str> {
        self.build_string().map(|s| std::path::PathBuf::from(s))
    }
}

#[cfg(feature = "std")]
impl StringPathBuilder<WithResolverAndSymlinkTraversal> {
    pub fn build_string(mut self) -> Result<String, &'static str> {
        self.path = self.path.resolve()?.traverse_symlinks()?;
        Ok(build_path(self))
    }

    pub fn build_os_string(self) -> Result<OsString, &'static str> {
        self.build_string().map(|s| OsString::from(s))
    }

    pub fn build_std_path(self) -> Result<std::path::PathBuf, &'static str> {
        self.build_string().map(|s| std::path::PathBuf::from(s))
    }
}

fn build_path<T>(builder: StringPathBuilder<T>) -> String {
    let mut result = String::new();

    match builder.path.prefix {
        Some(Prefix::ExtendedPath) => {
            result.push(builder.separator);
            result.push(builder.separator);
            result.push('?');
            result.push(builder.separator);
        }
        Some(Prefix::Device) => {
            result.push(builder.separator);
            result.push(builder.separator);
            result.push('.');
            result.push(builder.separator);
        }
        None => {}
    }

    if let Some(Drive { letter }) = builder.path.drive {
        result.push(letter);
        result.push(':');
    }

    match builder.path.root {
        Some(Root::Normal) => {
            result.push(builder.separator);
        }
        Some(Root::Unc) => {
            if let Some(Prefix::ExtendedPath) = builder.path.prefix {
                result.push_str("UNC");
                result.push(builder.separator);
            } else {
                result.push(builder.separator);
                result.push(builder.separator);
            }
        }
        None => {}
    }

    let len = builder.path.segments.len();

    for (i, segment) in builder.path.segments.into_iter().enumerate() {
        result.push_str(&segment.0);

        if i < len - 1 {
            result.push(builder.separator);
        }
    }

    result
}

#[cfg(test)]
mod test {
    use core::str::FromStr;
    use rstest::rstest;

    use super::*;

    #[rstest]
    fn build_with_defaults() {
        // arrange
        let path = Path::from_str("a/b/c").unwrap();

        // act
        let string = StringPathBuilder::new(path).build_string();

        // assert
        assert_eq!(string, "a/b/c");
    }

    #[rstest]
    fn build_with_backslash_separator() {
        // arrange
        let path = Path::from_str("a/b/c").unwrap();

        // act
        let string = StringPathBuilder::new(path)
            .with_separator('\\')
            .build_string();

        // assert
        assert_eq!(string, r"a\b\c");
    }

    #[rstest]
    fn build_with_resolver() {
        // arrange
        let path = Path::from_str("a/b/./c/../d").unwrap();

        // act
        let string = StringPathBuilder::new(path)
            .with_resolver()
            .build_string()
            .unwrap();

        // assert
        assert_eq!(string, "a/b/d");
    }

    #[rstest]
    fn build_with_resolver2() {
        // arrange
        let path = Path::from_str("../b/c").unwrap();

        // act
        let string = StringPathBuilder::new(path)
            .with_resolver()
            .build_string()
            .unwrap();

        // assert
        assert_eq!(string, "../b/c");
    }

    #[rstest]
    fn build_with_resolver3() {
        // arrange
        let path = Path::from_str("a/b/..").unwrap();

        // act
        let string = StringPathBuilder::new(path)
            .with_resolver()
            .build_string()
            .unwrap();

        // assert
        assert_eq!(string, "a");
    }

    #[rstest]
    fn build_with_resolver4() {
        // arrange
        let path = Path::from_str("..").unwrap();

        // act
        let string = StringPathBuilder::new(path)
            .with_resolver()
            .build_string()
            .unwrap();

        // assert
        assert_eq!(string, "..");
    }

    #[rstest]
    fn build_with_resolver5() {
        // arrange
        let path = Path::from_str("a/..").unwrap();

        // act
        let string = StringPathBuilder::new(path)
            .with_resolver()
            .build_string()
            .unwrap();

        // assert
        assert_eq!(string, "");
    }

    #[rstest]
    fn build_with_resolver6() {
        // arrange
        let path = Path::from_str(r"a/../b/c/../d").unwrap();

        // act
        let string = path.builder().with_resolver().build_string().unwrap();

        // assert
        assert_eq!(string, "b/d");
    }

    #[cfg(not(feature = "std"))]
    #[rstest]
    fn build_with_resolver7() {
        // arrange
        let path = Path::from_str(r"~/a/../b/c/../d").unwrap();

        // act
        let string = path.builder().with_resolver().build_string().unwrap();

        // assert
        assert_eq!(string, "~/b/d");
    }

    #[rstest]
    fn test_resolve() {
        // arrange
        let path = Path::from_str("a/../b/c/../d").unwrap();
        let path2 = path.clone().resolve().unwrap();
        let path3 = Path::from_str("b/d").unwrap();

        let path = path.builder().build_string();
        let path2 = path2.builder().with_resolver().build_string().unwrap();
        let path3 = path3.builder().with_resolver().build_string().unwrap();

        // assert
        assert_ne!(path, path2);
        assert_eq!(path2, path3);
    }

    #[cfg(feature = "std")]
    #[rstest]
    fn test_resolve_home() {
        // arrange
        let path = Path::from_str("~/.config").unwrap();

        // act
        let resolved = path.resolve().unwrap();

        // assert
        assert!(resolved.prefix.is_none());
        assert!(resolved.root.is_some());
        assert_eq!(resolved.root.as_ref().unwrap(), &Root::Normal);

        let home = dirs::home_dir().unwrap();
        let home_path = Path::from_str(home.to_str().unwrap()).unwrap();
        assert_eq!(
            resolved,
            home_path.join(Path::from_str(".config").unwrap()).unwrap()
        );
    }

    #[cfg(feature = "std")]
    #[rstest]
    fn test_resolve_home2() {
        // arrange
        let path = Path::from_str("~").unwrap();

        // act
        let resolved = path.resolve().unwrap();

        // assert
        let home = dirs::home_dir().unwrap();
        let home_path = Path::from_str(home.to_str().unwrap()).unwrap();
        assert_eq!(resolved, home_path);
    }

    #[rstest]
    fn tilde_segment_in_path() {
        // arrange
        let path = Path::from_str("path/~/file.txt").unwrap();

        // act
        let resolved = path.clone().resolve().unwrap();

        // assert
        assert_eq!(resolved, path);
    }
}
