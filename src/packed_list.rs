use core::ops::{Index, IndexMut};

use crate::path::PathSegment;
use alloc::vec::Vec;

#[derive(Debug, Clone, Eq)]
pub(crate) struct PathSegmentList {
    nodes: Vec<Node>,
    pub(crate) head: Option<usize>,
    pub(crate) tail: Option<usize>,
    free_list: Vec<usize>,
}

#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub(crate) struct Node {
    pub(crate) value: PathSegment,
    pub(crate) prev: Option<usize>,
    pub(crate) next: Option<usize>,
}

impl PathSegmentList {
    pub fn new() -> Self {
        PathSegmentList {
            nodes: Vec::new(),
            head: None,
            tail: None,
            free_list: Vec::new(),
        }
    }

    pub fn len(&self) -> usize {
        self.nodes.len() - self.free_list.len()
    }

    pub(crate) fn head(&self) -> Option<&Node> {
        self.head.and_then(|index| self.nodes.get(index))
    }

    pub(crate) fn next(&self, node: &Node) -> Option<&Node> {
        node.next.and_then(|next_index| self.nodes.get(next_index))
    }

    pub(crate) fn free(&mut self, index: usize) -> bool {
        if index < self.nodes.len() {
            // self.nodes[index] = Node::default();
            self.free_list.push(index);
            true
        } else {
            false
        }
    }

    pub(crate) fn push(&mut self, value: impl Into<PathSegment>) {
        let value = value.into();
        let index = if let Some(free_index) = self.free_list.pop() {
            free_index
        } else {
            self.nodes.len()
        };

        if index >= self.nodes.len() {
            self.nodes.push(Node {
                value,
                prev: None,
                next: None,
            });
        } else {
            self.nodes[index].value = value;
        }

        if self.head.is_none() {
            self.head = Some(index);
        }

        if let Some(tail) = self.tail {
            self.nodes[tail].next = Some(index);
            self.nodes[index].prev = Some(tail);
        }

        self.tail = Some(index);
    }

    pub(crate) fn remove(&mut self, index: usize) -> Option<Node> {
        // Temporarily take the node out to avoid double borrowing
        let (prev, next, value) = {
            let node = self.nodes.get_mut(index)?;
            (node.prev, node.next, core::mem::take(node))
        };

        // Now, patch up the prev and next links if needed
        if let Some(prev) = prev {
            self.nodes[prev].next = next;
        } else {
            self.head = next;
        }

        if let Some(next) = next {
            self.nodes[next].prev = prev;
        } else {
            self.tail = prev;
        }

        self.free_list.push(index);
        Some(value)
    }

    pub(crate) fn remove_last(&mut self) -> Option<Node> {
        if let Some(tail_index) = self.tail {
            let node = self.remove(tail_index)?;

            if self.tail == Some(tail_index) {
                self.tail = node.prev;
            }

            Some(node)
        } else {
            None
        }
    }

    pub fn iter(&self) -> PathSegmentListIter {
        PathSegmentListIter {
            list: self,
            current: self.head(),
        }
    }
}

impl Index<usize> for PathSegmentList {
    type Output = Node;
    fn index(&self, index: usize) -> &Self::Output {
        if let Some(node) = self.nodes.get(index) {
            node
        } else {
            panic!("Index out of bounds");
        }
    }
}

impl IndexMut<usize> for PathSegmentList {
    fn index_mut(&mut self, index: usize) -> &mut Self::Output {
        if let Some(node) = self.nodes.get_mut(index) {
            node
        } else {
            panic!("Index out of bounds");
        }
    }
}

impl IntoIterator for PathSegmentList {
    type Item = PathSegment;
    type IntoIter = PathSegmentListIntoIter;

    fn into_iter(mut self) -> Self::IntoIter {
        let current = self.head.and_then(|head| self.remove(head));

        PathSegmentListIntoIter {
            list: self,
            current,
        }
    }
}

impl<P: Into<PathSegment>> FromIterator<P> for PathSegmentList {
    fn from_iter<I: IntoIterator<Item = P>>(iter: I) -> Self {
        let mut list = PathSegmentList::new();

        for item in iter {
            list.push(item);
        }

        list
    }
}

pub(crate) struct PathSegmentListIntoIter {
    list: PathSegmentList,
    current: Option<Node>,
}

impl Iterator for PathSegmentListIntoIter {
    type Item = PathSegment;

    fn next(&mut self) -> Option<Self::Item> {
        let current_node = self.current.take()?;
        let value = current_node.value;
        let next_index = current_node.next;
        self.current = next_index.and_then(|idx| self.list.remove(idx));
        Some(value)
    }
}

pub(crate) struct PathSegmentListIter<'a> {
    list: &'a PathSegmentList,
    current: Option<&'a Node>,
}

impl<'a> Iterator for PathSegmentListIter<'a> {
    type Item = &'a PathSegment;

    fn next(&mut self) -> Option<Self::Item> {
        let current = self.current?;
        self.current = self.list.next(current);
        Some(&current.value)
    }
}

impl<'a> IntoIterator for &'a PathSegmentList {
    type Item = &'a PathSegment;
    type IntoIter = PathSegmentListIter<'a>;

    fn into_iter(self) -> Self::IntoIter {
        PathSegmentListIter {
            list: self,
            current: self.head(),
        }
    }
}

impl<P: Into<PathSegment>> From<P> for PathSegmentList {
    fn from(segment: P) -> Self {
        let mut list = PathSegmentList::new();
        list.push(segment);
        list
    }
}

impl PartialEq for PathSegmentList {
    fn eq(&self, other: &Self) -> bool {
        if self.len() != other.len() {
            return false;
        }

        let mut self_iter = self.iter();
        let mut other_iter = other.iter();

        while let (Some(self_segment), Some(other_segment)) = (self_iter.next(), other_iter.next())
        {
            if self_segment != other_segment {
                return false;
            }
        }

        true
    }
}

#[cfg(test)]
mod test {
    use alloc::string::ToString;
    use rstest::rstest;

    pub(crate) use super::*;

    #[rstest]
    fn from_iter() {
        // arrange
        let segments = Vec::from([
            PathSegment("a".to_string()),
            PathSegment("b".to_string()),
            PathSegment("c".to_string()),
        ]);

        // act
        let packed_list = segments.into_iter().collect::<PathSegmentList>();

        // assert
        assert_eq!(packed_list.len(), 3);
        let first = packed_list.head().unwrap();
        assert_eq!(first.value.0, "a");
        let second = packed_list.next(first).unwrap();
        assert_eq!(second.value.0, "b");
        let third = packed_list.next(second).unwrap();
        assert_eq!(third.value.0, "c");
    }

    #[rstest]
    fn remove() {
        // arrange
        let segments = Vec::from([
            PathSegment("a".to_string()),
            PathSegment("b".to_string()),
            PathSegment("c".to_string()),
        ]);

        let mut packed_list = segments.into_iter().collect::<PathSegmentList>();

        // act
        packed_list.remove(1);

        // assert
        assert_eq!(packed_list.len(), 2);
        let first = packed_list.head().unwrap();
        assert_eq!(first.value.0, "a");
        let second = packed_list.next(first).unwrap();
        assert_eq!(second.value.0, "c");
        assert!(packed_list.next(second).is_none());
    }
}
