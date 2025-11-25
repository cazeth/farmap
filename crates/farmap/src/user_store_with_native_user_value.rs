use crate::user_serde::UserSerde;
use crate::user_value::AnyNativeUserValue;
use crate::user_value::NativeUserValue;
use crate::Collidable;
use crate::Fid;
use crate::UserError;
use itertools::*;
use serde::{Deserialize, Serialize};

#[derive(Debug, PartialEq, Eq, Serialize, Deserialize, Clone, Hash)]
#[serde(from = "UserSerde")]
#[serde(into = "UserSerde")]
pub struct UserStoreWithNativeUserValue {
    fid: Fid,
    user_values: Option<Vec<AnyNativeUserValue>>,
}

impl UserStoreWithNativeUserValue {
    /// Check if a User has at least one value of type T.
    pub fn has<T: NativeUserValue>(&self) -> bool {
        if let Some(user_values) = &self.user_values {
            user_values
                .iter()
                .any(|val| T::from_any_user_value_ref(val).is_some())
        } else {
            false
        }
    }

    /// Add a [`UserValue`] to a [`User`]. Returns an error in case of Collision. This method
    /// relies on Ts implementation of Collidable to determine collisions.
    pub fn try_add_user_value<T>(&mut self, value: T) -> Result<(), UserError>
    where
        T: NativeUserValue + Collidable,
    {
        if self
            .user_values_of_kind::<T>()
            .iter()
            .all(|x| !Collidable::is_collision(*x, &value))
        {
            self.add_user_value(value);
            Ok(())
        } else {
            Err(UserError::CollisionError)
        }
    }

    /// Insert a new [`UserValue`]. Returns Ok() on duplicate.
    /// This method does not check for collisions.
    pub fn add_user_value<T>(&mut self, value: T)
    where
        T: NativeUserValue,
    {
        let any_user_value = value.into_any_user_value();
        if let Some(value_vec) = &mut self.user_values {
            value_vec.push(any_user_value)
        } else {
            self.user_values = Some(vec![any_user_value]);
        }
    }

    pub fn all_user_values(&self) -> &Option<Vec<AnyNativeUserValue>> {
        &self.user_values
    }

    /// Get all the [`UserValue`]s of type T. If there are no such values, the method returns an
    /// empty vec.
    pub fn user_values_of_kind<T>(&self) -> Vec<&T>
    where
        T: NativeUserValue,
    {
        if let Some(user_values) = &self.user_values {
            user_values
                .iter()
                .flat_map(|user_value| user_value.specify_ref::<T>())
                .collect_vec()
        } else {
            Vec::new()
        }
    }

    pub fn new(fid: impl Into<Fid>) -> Self {
        let fid = fid.into();
        Self {
            fid,
            user_values: None,
        }
    }

    pub fn fid(&self) -> Fid {
        self.fid
    }

    pub(crate) fn from_user_values(fid: Fid, values: Vec<AnyNativeUserValue>) -> Self {
        if !values.is_empty() {
            Self {
                fid,
                user_values: Some(values),
            }
        } else {
            Self {
                fid,
                user_values: None,
            }
        }
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
