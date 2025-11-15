use crate::collidable::Collidable;
use crate::spam_score::SpamEntries;
use crate::spam_score::SpamRecord;
use crate::spam_score::SpamScore;
use crate::user_value::AnyUserValue;
use crate::UserValue;
use chrono::Local;
use chrono::NaiveDate;
use chrono::NaiveDateTime;
use itertools::*;
use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Debug, PartialEq, Eq, Serialize, Deserialize, Clone, Hash)]
pub struct User {
    fid: usize,
    #[serde(rename = "entries")]
    labels: Option<SpamEntries>,

    /// Some(Empty vec): has been checked and there were no cast records.
    /// None: Has not been checked.
    reaction_times: Option<Vec<NaiveDateTime>>,
    latest_reaction_time_update_date: Option<NaiveDateTime>,
    latest_cast_record_check_date: Option<NaiveDate>,
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
    pub fn try_add_user_value<T>(&mut self, value: T) -> Result<(), UserValueError>
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
            Err(UserValueError::CollisionError)
        }
    }

    /// Insert a new [`UserValue`]. Returns Ok() on duplicate.
    pub fn add_user_value<T>(&mut self, value: T) -> Result<(), UserValueError>
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

    pub fn new_without_labels(fid: usize) -> Self {
        Self {
            fid,
            labels: None,
            latest_cast_record_check_date: None,
            reaction_times: None,
            latest_reaction_time_update_date: None,
            user_values: None,
        }
    }

    pub fn update_reaction_times(
        &mut self,
        reaction_times: Vec<NaiveDateTime>,
    ) -> Option<Vec<NaiveDateTime>> {
        self.latest_reaction_time_update_date = Some(Local::now().naive_utc());
        self.reaction_times.replace(reaction_times)
    }

    pub fn latest_reaction_time(&self) -> Option<&NaiveDateTime> {
        if let Some(reaction_times) = &self.reaction_times {
            Some(reaction_times.iter().max()?)
        } else {
            None
        }
    }

    pub fn fid(&self) -> usize {
        self.fid
    }

    pub fn all_spam_records_with_opt(&self) -> Option<Vec<SpamRecord>> {
        let records = self
            .labels
            .as_ref()?
            .all_spam_entries()
            .iter()
            .cloned()
            .map(|x| x.record())
            .collect_vec();
        Some(records)
    }

    pub fn latest_reaction_time_update_date(&self) -> Option<NaiveDateTime> {
        self.latest_reaction_time_update_date
    }

    pub fn reaction_times(&self) -> &Option<Vec<NaiveDateTime>> {
        &self.reaction_times
    }

    pub fn latest_cast_record_check_date(&self) -> Option<NaiveDate> {
        self.latest_cast_record_check_date
    }

    pub fn latest_spam_score_update_date_with_opt(&self) -> Option<NaiveDate> {
        Some(self.labels.as_ref()?.last_spam_entry().date())
    }

    pub fn earliest_spam_score_date_with_opt(&self) -> Option<NaiveDate> {
        Some(self.labels.as_ref()?.earliest_spam_entry().date())
    }

    pub fn latest_spam_score_date_with_opt(&self) -> Option<NaiveDate> {
        Some(self.labels.as_ref()?.last_spam_entry().date())
    }
}

#[derive(Error, Debug, PartialEq)]
pub enum UserValueError {
    #[error(
        "User Value collides with existing user value. A User cannot contain colliding user values"
    )]
    CollisionError,
}

#[derive(Error, Debug, PartialEq)]
pub enum UserError {
    #[error("User {0} already has a spam record for date {1}. The existing spam score at the date is {} but a spamscore of {} is now trying to be set.", .fid, . date) ]
    SpamScoreCollision {
        fid: usize,
        date: NaiveDate,
        old_spam_score: SpamScore,
        new_spam_score: SpamScore,
    },
    #[error("Trying to merge users with different fids. For merge_user to work both input users must have the same fid. provided fid_1: {} and provided fid_2 {}", .fid_1, .fid_2)]
    DifferentFidMerge { fid_1: usize, fid_2: usize },
}

#[derive(Error, Debug, PartialEq)]
pub enum InvalidInputError {
    #[error("SpamScore was {0}, not zero, one or two.", .label)]
    SpamScoreError { label: usize },
    #[error("Timestamp was {0}, which is invalid.", . timestamp)]
    DateError { timestamp: usize },
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
        User::new_without_labels(fid)
    }

    pub fn valid_user_value_add<T: UserValue>(user: &mut User, value: T) {
        user.add_user_value::<T>(value).unwrap()
    }

    #[test]
    fn test_user_values_of_kind_is_none_on_none_user_values() {
        let user = User::new_without_labels(1);
        assert!(user.user_values_of_kind::<DatedSpamUpdate>().is_empty());
    }
}
