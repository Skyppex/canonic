// use alloc::string::ToString;
//
// use crate::path::{Path, PathSegment};
//
// pub fn parse(input: &str) -> Result<Path, &'static str> {
//     if input.is_empty() {
//         return Ok(Path::new());
//     }
//
//     let mut path = Path::new();
//
//     for segment in input.replace("\\", "/").split('/') {
//         if !segment.is_empty() {
//             path.join(PathSegment(segment.to_string()));
//         }
//     }
//
//     if input.starts_with('/') {
//         path.has_root = true;
//     }
//
//     Ok(path)
// }
