use crate::is_user::IsUser;

/// Create from [`IsUser`].
///
/// This trait is preferred to the more general [`TryFrom`] because it is likely that you want to create from something that itself implements [`IsUser`], which is not allowed for [`TryFrom`].
pub trait TryFromUser<'a, T: IsUser<'a>>: Sized {
    type Error;
    fn try_from_user(value: T) -> Result<Self, Self::Error>;
}
