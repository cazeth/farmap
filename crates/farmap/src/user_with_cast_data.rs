use crate::dated::Dated;
use crate::is_user::IsUser;
use crate::try_from_user::TryFromUser;
use crate::user::UserStoreWithNativeUserValue;
use crate::CastType;
use crate::Fid;
use thiserror::Error;

/// A reference to a [`User`] of lifetime a that contains at least one CastType.
#[derive(Debug, Clone, PartialEq)]
pub struct UserWithCastData<'a> {
    user: &'a UserStoreWithNativeUserValue,
}

impl<'a> IsUser<'a> for UserWithCastData<'a> {
    fn fid(&self) -> Fid {
        self.user.fid()
    }

    fn user(&self) -> &'a UserStoreWithNativeUserValue {
        self.user
    }
}

impl<'a> TryFrom<&'a UserStoreWithNativeUserValue> for UserWithCastData<'a> {
    type Error = NoCastDataError;
    fn try_from(value: &'a UserStoreWithNativeUserValue) -> Result<Self, Self::Error> {
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
