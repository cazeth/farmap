use crate::core::UserStore;
use crate::user_serde::UserSerde;
use crate::user_value::AnyNativeUserValue;
use crate::user_value::NativeUserValue;
use crate::Collidable;
use crate::Fid;
use crate::UserError;
use serde::{Deserialize, Serialize};

#[derive(Debug, PartialEq, Eq, Serialize, Deserialize, Clone, Hash)]
#[serde(from = "UserSerde")]
#[serde(into = "UserSerde")]
pub struct UserStoreWithNativeUserValue {
    fid: Fid,
    core: UserStore<AnyNativeUserValue>,
}

impl UserStoreWithNativeUserValue {
    /// Check if a User has at least one value of type T.
    pub fn has<T: NativeUserValue>(&self) -> bool {
        self.core.has::<T>()
    }

    /// Add a [`UserValue`] to a [`User`]. Returns an error in case of Collision. This method
    /// relies on Ts implementation of Collidable to determine collisions.
    pub fn try_add_user_value<T>(&mut self, value: T) -> Result<(), UserError>
    where
        T: NativeUserValue + Collidable,
    {
        self.core.try_add_user_value(value)
    }

    /// Insert a new [`UserValue`]. Returns Ok() on duplicate.
    /// This method does not check for collisions.
    pub fn add_user_value<T>(&mut self, value: T)
    where
        T: NativeUserValue,
    {
        self.core.add_user_value(value);
    }

    pub fn all_user_values(&self) -> &Vec<AnyNativeUserValue> {
        self.core.all_user_values()
    }

    /// Get all the [`UserValue`]s of type T. If there are no such values, the method returns an
    /// empty vec.
    pub fn user_values_of_kind<T>(&self) -> Vec<&T>
    where
        T: NativeUserValue,
    {
        self.core.user_values_of_kind::<T>().collect()
    }

    pub fn new(fid: impl Into<Fid>) -> Self {
        let fid = fid.into();
        Self {
            fid,
            core: UserStore::<AnyNativeUserValue>::from(fid),
        }
    }

    pub fn fid(&self) -> Fid {
        self.core.fid()
    }

    pub(crate) fn from_user_values(fid: Fid, values: Vec<AnyNativeUserValue>) -> Self {
        let core = UserStore::<AnyNativeUserValue>::from_generic_user_values(fid, values);
        Self { fid, core }
    }
}

#[cfg(test)]
pub mod tests {
    use crate::spam_score::DatedSpamUpdate;
    use crate::Fid;

    use super::*;

    pub mod test_fid {
        use super::*;

        pub fn is_fid(user: &UserStoreWithNativeUserValue, fid: impl Into<Fid>) -> bool {
            let fid = fid.into();
            user.fid() == fid
        }
    }

    pub fn create_new_user(fid: impl TryInto<Fid>) -> UserStoreWithNativeUserValue {
        let fid: Fid = fid
            .try_into()
            .unwrap_or_else(|_| panic!("could not convert"));
        UserStoreWithNativeUserValue::new(fid)
    }

    pub fn valid_user_value_add<T: NativeUserValue>(
        user: &mut UserStoreWithNativeUserValue,
        value: T,
    ) {
        user.add_user_value::<T>(value)
    }

    #[test]
    fn test_user_values_of_kind_is_none_on_none_user_values() {
        let user = create_new_user(1);
        assert!(user.user_values_of_kind::<DatedSpamUpdate>().is_empty());
    }
}
