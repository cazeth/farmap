use crate::is_user::IsUser;

pub trait UserSet<'a>: IntoIterator<Item: IsUser<'a>> {
    fn user_count(&self) -> usize;
    fn user(&'a self, fid: usize) -> Option<impl IsUser<'a>>;
}
