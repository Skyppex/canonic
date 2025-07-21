use std::path::{Path as StdPath, PathBuf as StdPathBuf};

use crate::path::Path;

impl TryFrom<&StdPath> for Path {
    type Error = &'static str;

    fn try_from(value: &StdPath) -> Result<Self, Self::Error> {
        Path::try_from(value.as_os_str())
    }
}

impl TryFrom<std::path::PathBuf> for Path {
    type Error = &'static str;

    fn try_from(value: std::path::PathBuf) -> Result<Self, Self::Error> {
        Path::try_from(value.as_os_str())
    }
}

impl Into<StdPathBuf> for Path {
    fn into(self) -> StdPathBuf {
        let path_str = self.builder().build_string();
        StdPathBuf::from(path_str)
    }
}

impl Into<StdPathBuf> for &Path {
    fn into(self) -> StdPathBuf {
        let path_str = self.clone().builder().build_string();
        StdPathBuf::from(path_str)
    }
}
