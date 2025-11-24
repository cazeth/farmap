use crate::Fid;
use crate::HasTag;
use crate::UserValue;

/// A fid value wrapper.
#[derive(PartialEq, Eq, Hash, Debug, Clone, Copy)]
pub struct Fidded<T> {
    inner: T,
    fid: Fid,
}

impl<T> Fidded<T> {
    pub fn unfid(self) -> T {
        self.inner
    }

    pub fn fid(&self) -> Fid {
        self.fid
    }
}

impl<T> From<(T, Fid)> for Fidded<T> {
    fn from(value: (T, Fid)) -> Self {
        Self {
            inner: value.0,
            fid: value.1,
        }
    }
}

impl<T: UserValue> HasTag<Fid, T> for Fidded<T> {
    fn tag(&self) -> Fid {
        self.fid
    }

    fn untag(self) -> (Fid, T) {
        (self.fid, self.inner)
    }
}
