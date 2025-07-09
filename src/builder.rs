use alloc::string::String;

use crate::path::Path;

pub struct StringPathBuilder {
    path: Path,
    separator: String,
    resolve: bool,
    traverse_symlinks: bool,
}

impl StringPathBuilder {
    pub fn new(path: impl Into<Path>) -> Self {
        StringPathBuilder {
            path: path.into(),
            separator: String::from("/"),
            resolve: false,
            traverse_symlinks: false,
        }
    }

    pub fn with_separator(mut self, separator: impl Into<String>) -> Self {
        self.separator = separator.into();
        self
    }

    pub fn with_resolver(mut self, resolve: bool) -> Self {
        self.resolve = resolve;
        self
    }

    pub fn traverse_symlinks(mut self, traverse_symlinks: bool) -> Self {
        self.traverse_symlinks = traverse_symlinks;
        self
    }

    pub fn build(mut self) -> String {
        if self.resolve {
            self.path = self.path.resolve();
        }

        if self.traverse_symlinks {
            self.path = self.path.traverse_symlinks();
        }

        let mut result = String::new();

        if self.path.is_absolute() {
            result.push_str(&self.separator);
        }

        for segment in self.path.components().into_iter() {
            if !result.is_empty() {
                result.push_str(&self.separator);
            }

            result.push_str(segment);
        }

        result
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
        let string = StringPathBuilder::new(path).with_separator(r"\").build();

        // assert
        assert_eq!(string, r"a\b\c");
    }

    #[rstest]
    fn build_with_resolver() {
        // arrange
        let path = Path::from_str("a/b/./c/../d").unwrap();

        // act
        let string = StringPathBuilder::new(path).with_resolver(true).build();

        // assert
        assert_eq!(string, "a/b/d");
    }

    #[rstest]
    fn build_with_resolver2() {
        // arrange
        let path = Path::from_str("../b/c").unwrap();

        // act
        let string = StringPathBuilder::new(path).with_resolver(true).build();

        // assert
        assert_eq!(string, "../b/c");
    }

    #[rstest]
    fn build_with_resolver3() {
        // arrange
        let path = Path::from_str("a/b/..").unwrap();

        // act
        let string = StringPathBuilder::new(path).with_resolver(true).build();

        // assert
        assert_eq!(string, "a");
    }

    #[rstest]
    fn build_with_resolver4() {
        // arrange
        let path = Path::from_str("..").unwrap();

        // act
        let string = StringPathBuilder::new(path).with_resolver(true).build();

        // assert
        assert_eq!(string, "..");
    }

    #[rstest]
    fn build_with_resolver5() {
        // arrange
        let path = Path::from_str("a/..").unwrap();

        // act
        let string = StringPathBuilder::new(path).with_resolver(true).build();

        // assert
        assert_eq!(string, "");
    }

    #[rstest]
    fn build_with_resolver6() {
        // arrange
        let path = Path::from_str(r"a/../b/c/../d").unwrap();

        // act
        let string = path.builder().with_resolver(true).build();

        // assert
        assert_eq!(string, "b/d");
    }

    #[rstest]
    fn test_resolve() {
        // arrange
        let path = Path::from_str("a/../b/c/../d").unwrap();
        let path2 = path.clone().resolve();
        let path3 = Path::from_str("b/d").unwrap();

        let path = path.builder().build();
        let path2 = path2.builder().with_resolver(true).build();
        let path3 = path3.builder().with_resolver(true).build();

        // assert
        assert_ne!(path, path2);
        assert_eq!(path2, path3);
    }
}
