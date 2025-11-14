use crate::HasTag;
use crate::UserValue;

/// A fid value wrapper.
#[derive(PartialEq, Eq, Hash, Debug, Clone, Copy)]
pub struct Fidded<T> {
    inner: T,
    fid: usize,
}

impl<T> Fidded<T> {
    pub fn unfid(self) -> T {
        self.inner
    }

    pub fn fid(&self) -> usize {
        self.fid
    }
}

impl<T> From<(T, usize)> for Fidded<T> {
    fn from(value: (T, usize)) -> Self {
        Self {
            inner: value.0,
            fid: value.1,
        }
    }
}

impl<T: UserValue> HasTag<u64> for Fidded<T> {
    fn tag(&self) -> u64 {
        self.fid as u64
    }

    fn untag(self) -> (impl UserValue, u64) {
        let fid = self.fid as u64;
        (self.inner, fid)
    }
}
