use crate::dated::Dated;
use crate::is_user::IsUser;
use crate::try_from_user::TryFromUser;
use crate::user::User;
use crate::CastType;
use thiserror::Error;

/// A reference to a [`User`] of lifetime a that contains at least one CastType.
#[derive(Debug, Clone, PartialEq)]
pub struct UserWithCastData<'a> {
    user: &'a User,
}

impl<'a> IsUser<'a> for UserWithCastData<'a> {
    fn fid(&self) -> usize {
        self.user.fid()
    }

    fn user(&self) -> &'a User {
        self.user
    }
}

impl<'a> TryFrom<&'a User> for UserWithCastData<'a> {
    type Error = NoCastDataError;
    fn try_from(value: &'a User) -> Result<Self, Self::Error> {
        if value.has::<Dated<CastType>>() {
            Ok(Self { user: value })
        } else {
            Err(NoCastDataError)
        }
    }
}

#[derive(Error, Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[error("no cast data in user")]
pub struct NoCastDataError;

impl<'a, T: IsUser<'a>> TryFromUser<'a, T> for UserWithCastData<'a> {
    type Error = NoCastDataError;
    fn try_from_user(value: T) -> Result<Self, Self::Error> {
        if value.user().has::<Dated<CastType>>() {
            Ok(UserWithCastData { user: value.user() })
        } else {
            Err(NoCastDataError)
        }
    }
}
