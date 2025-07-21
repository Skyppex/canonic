pub struct ZipGreedy<L, R> {
    left: L,
    right: R,
}

impl<L, R> ZipGreedy<L, R> {
    pub fn new(left: L, right: R) -> ZipGreedy<L, R> {
        ZipGreedy { left, right }
    }
}

impl<T, U, L: Iterator<Item = T>, R: Iterator<Item = U>> Iterator for ZipGreedy<L, R> {
    type Item = (Option<T>, Option<U>);

    fn next(&mut self) -> Option<Self::Item> {
        let l = self.left.next();
        let r = self.right.next();

        if l.is_none() && r.is_none() {
            return None;
        }

        Some((l, r))
    }
}

pub fn zip_greedy<L, R>(left: L, right: R) -> ZipGreedy<L, R> {
    ZipGreedy::new(left, right)
}
