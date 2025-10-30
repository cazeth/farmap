use crate::User;

/// A type that references user data.
pub trait IsUser<'a> {
    fn fid(&self) -> usize;

    fn user(&self) -> &'a User;
}

impl<'a, T: IsUser<'a>> IsUser<'a> for &T {
    fn fid(&self) -> usize {
        T::fid(self)
    }

    fn user(&self) -> &'a User {
        T::user(self)
    }
}
