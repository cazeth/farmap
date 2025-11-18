use crate::collidable::Collidable;
use crate::user_value::AnyUserValue;
use crate::UserValue;
use chrono::Local;
use chrono::NaiveDateTime;
use itertools::*;
use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Debug, PartialEq, Eq, Serialize, Deserialize, Clone, Hash)]
pub struct User {
    fid: usize,
    user_values: Option<Vec<(AnyUserValue, NaiveDateTime)>>,
}

impl User {
    /// Check if a User has at least one value of type T.
    pub fn has<T: UserValue>(&self) -> bool {
        if let Some(user_values) = &self.user_values {
            user_values
                .iter()
                .any(|(val, _)| T::from_any_user_value_ref(val).is_some())
        } else {
            false
        }
    }

    /// Add a [`UserValue`] to a [`User`]. Returns an error in case of Collision. This method
    /// relies on Ts implementation of Collidable to determine collisions.
    pub fn try_add_user_value<T>(&mut self, value: T) -> Result<(), UserError>
    where
        T: UserValue + Collidable,
    {
        if self
            .user_values_of_kind::<T>()
            .iter()
            .all(|x| !Collidable::is_collision(*x, &value))
        {
            self.add_user_value(value)
        } else {
            Err(UserError::CollisionError)
        }
    }

    /// Insert a new [`UserValue`]. Returns Ok() on duplicate.
    pub fn add_user_value<T>(&mut self, value: T) -> Result<(), UserError>
    where
        T: UserValue,
    {
        if self.update_time_if_duplicate(&value).is_some() {
            return Ok(());
        };

        let any_user_value = value.into_any_user_value();
        if let Some(value_vec) = &mut self.user_values {
            value_vec.push((any_user_value, Self::now()))
        } else {
            self.user_values = Some(vec![(any_user_value, Self::now())]);
        }
        Ok(())
    }

    pub fn all_user_values(&self) -> &Option<Vec<(AnyUserValue, NaiveDateTime)>> {
        &self.user_values
    }

    /// Get all the [`UserValue`]s of type T. If there are no such values, the method returns an
    /// empty vec.
    pub fn user_values_of_kind<T>(&self) -> Vec<&T>
    where
        T: UserValue,
    {
        if let Some(user_values) = &self.user_values {
            user_values
                .iter()
                .flat_map(|(user_value, _)| user_value.specify_ref::<T>())
                .collect_vec()
        } else {
            Vec::new()
        }
    }

    fn now() -> NaiveDateTime {
        Local::now().naive_local()
    }

    fn update_time_if_duplicate<T>(&mut self, value: &T) -> Option<NaiveDateTime>
    where
        T: UserValue,
    {
        if let Some(user_values) = &mut self.user_values {
            let duplicate_position = user_values
                .iter()
                .position(|prev| prev.0 == value.as_any_user_value())?;

            let new_time = Self::now();
            let old_time = std::mem::replace(&mut user_values[duplicate_position].1, new_time);
            Some(old_time)
        } else {
            None
        }
    }

    pub fn new(fid: usize) -> Self {
        Self {
            fid,
            user_values: None,
        }
    }

    pub fn fid(&self) -> usize {
        self.fid
    }
}

#[derive(Error, Debug, PartialEq)]
#[non_exhaustive]
pub enum UserError {
    #[error(
        "User Value collides with existing user value. A User cannot contain colliding user values"
    )]
    CollisionError,
}

#[cfg(test)]
pub mod tests {
    use crate::spam_score::DatedSpamUpdate;

    use super::*;

    pub mod test_fid {
        use super::*;

        pub fn is_fid(user: &User, fid: usize) -> bool {
            user.fid() == fid
        }
    }

    pub fn create_user(fid: usize) -> User {
        User::new(fid)
    }

    pub fn valid_user_value_add<T: UserValue>(user: &mut User, value: T) {
        user.add_user_value::<T>(value).unwrap()
    }

    #[test]
    fn test_user_values_of_kind_is_none_on_none_user_values() {
        let user = create_user(1);
        assert!(user.user_values_of_kind::<DatedSpamUpdate>().is_empty());
    }
}
