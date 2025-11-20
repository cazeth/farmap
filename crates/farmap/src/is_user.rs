use crate::Fid;
use crate::User;

/// A type that references user data.
pub trait IsUser<'a> {
    fn fid(&self) -> Fid;

    fn user(&self) -> &'a User;
}

impl<'a, T: IsUser<'a>> IsUser<'a> for &T {
    fn fid(&self) -> Fid {
        T::fid(self)
    }

    fn user(&self) -> &'a User {
        T::user(self)
    }
}
