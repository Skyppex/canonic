use core::marker::PhantomData;

use alloc::string::String;

use crate::path::{Drive, Path, Prefix, Root};

pub struct StringPathBuilder<T> {
    path: Path,
    separator: char,
    resolve: bool,
    traverse_symlinks: bool,
    _phantom_data: PhantomData<T>,
}

pub(crate) enum BaseState {}
pub(crate) enum HasResolverState {}

impl StringPathBuilder<BaseState> {
    pub fn new(path: impl Into<Path>) -> Self {
        StringPathBuilder::<BaseState> {
            path: path.into(),
            separator: '/',
            resolve: false,
            traverse_symlinks: false,
            _phantom_data: PhantomData,
        }
    }

    pub fn with_separator(mut self, separator: impl Into<char>) -> Self {
        self.separator = separator.into();
        self
    }

    pub fn with_resolver(self) -> StringPathBuilder<HasResolverState> {
        StringPathBuilder::<HasResolverState> {
            path: self.path,
            separator: self.separator,
            resolve: true,
            traverse_symlinks: self.traverse_symlinks,
            _phantom_data: PhantomData,
        }
    }

    pub fn traverse_symlinks(mut self, traverse_symlinks: bool) -> Self {
        self.traverse_symlinks = traverse_symlinks;
        self
    }

    pub fn build(mut self) -> String {
        if self.traverse_symlinks {
            self.path = self.path.traverse_symlinks();
        }

        let mut result = String::new();

        match self.path.prefix {
            Some(Prefix::ExtendedPath) => {
                result.push(self.separator);
                result.push(self.separator);
                result.push('?');
                result.push(self.separator);
            }
            Some(Prefix::Device) => {
                result.push(self.separator);
                result.push(self.separator);
                result.push('.');
                result.push(self.separator);
            }
            None => {}
        }

        if let Some(Drive { letter }) = self.path.drive {
            result.push(letter);
            result.push(':');
        }

        match self.path.root {
            Some(Root::Normal) => {
                result.push(self.separator);
            }
            Some(Root::Unc) => {
                if let Some(Prefix::ExtendedPath) = self.path.prefix {
                    result.push_str("UNC");
                    result.push(self.separator);
                } else {
                    result.push(self.separator);
                    result.push(self.separator);
                }
            }
            None => {}
        }

        let len = self.path.segments.len();
        for (i, segment) in self.path.segments.into_iter().enumerate() {
            result.push_str(&segment.0);

            if i < len - 1 {
                result.push(self.separator);
            }
        }

        result
    }
}

impl StringPathBuilder<HasResolverState> {
    pub fn with_separator(mut self, separator: impl Into<char>) -> Self {
        self.separator = separator.into();
        self
    }

    pub fn traverse_symlinks(mut self, traverse_symlinks: bool) -> Self {
        self.traverse_symlinks = traverse_symlinks;
        self
    }

    pub fn build(mut self) -> Result<String, &'static str> {
        if self.resolve {
            self.path = self.path.resolve()?;
        }

        if self.traverse_symlinks {
            self.path = self.path.traverse_symlinks();
        }

        let mut result = String::new();

        match self.path.prefix {
            Some(Prefix::ExtendedPath) => {
                result.push(self.separator);
                result.push(self.separator);
                result.push('?');
                result.push(self.separator);
            }
            Some(Prefix::Device) => {
                result.push(self.separator);
                result.push(self.separator);
                result.push('.');
                result.push(self.separator);
            }
            None => {}
        }

        if let Some(Drive { letter }) = self.path.drive {
            result.push(letter);
            result.push(':');
        }

        match self.path.root {
            Some(Root::Normal) => {
                result.push(self.separator);
            }
            Some(Root::Unc) => {
                if let Some(Prefix::ExtendedPath) = self.path.prefix {
                    result.push_str("UNC");
                    result.push(self.separator);
                } else {
                    result.push(self.separator);
                    result.push(self.separator);
                }
            }
            None => {}
        }

        let len = self.path.segments.len();
        for (i, segment) in self.path.segments.into_iter().enumerate() {
            result.push_str(&segment.0);

            if i < len - 1 {
                result.push(self.separator);
            }
        }

        Ok(result)
    }
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
        let string = StringPathBuilder::new(path).build();

        // assert
        assert_eq!(string, "a/b/c");
    }

    #[rstest]
    fn build_with_backslash_separator() {
        // arrange
        let path = Path::from_str("a/b/c").unwrap();

        // act
        let string = StringPathBuilder::new(path).with_separator('\\').build();

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
            .build()
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
            .build()
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
            .build()
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
            .build()
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
            .build()
            .unwrap();

        // assert
        assert_eq!(string, "");
    }

    #[rstest]
    fn build_with_resolver6() {
        // arrange
        let path = Path::from_str(r"a/../b/c/../d").unwrap();

        // act
        let string = path.builder().with_resolver().build().unwrap();

        // assert
        assert_eq!(string, "b/d");
    }

    #[rstest]
    fn test_resolve() {
        // arrange
        let path = Path::from_str("a/../b/c/../d").unwrap();
        let path2 = path.clone().resolve().unwrap();
        let path3 = Path::from_str("b/d").unwrap();

        let path = path.builder().build();
        let path2 = path2.builder().with_resolver().build().unwrap();
        let path3 = path3.builder().with_resolver().build().unwrap();

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
