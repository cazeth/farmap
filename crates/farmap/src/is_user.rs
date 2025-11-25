use crate::Fid;
use crate::UserStoreWithNativeUserValue;

/// A type that references user data.
pub trait IsUser<'a> {
    fn fid(&self) -> Fid;

    fn user(&self) -> &'a UserStoreWithNativeUserValue;
}

impl<'a, T: IsUser<'a>> IsUser<'a> for &T {
    fn fid(&self) -> Fid {
        T::fid(self)
    }

    fn user(&self) -> &'a UserStoreWithNativeUserValue {
        T::user(self)
    }
}
