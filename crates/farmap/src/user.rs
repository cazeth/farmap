use crate::cast_meta::CastMeta;
use crate::collidable::Collidable;
use crate::spam_score::SpamEntries;
use crate::spam_score::SpamEntry;
use crate::spam_score::SpamRecord;
use crate::spam_score::SpamScore;
use crate::user_value::AnyUserValue;
use crate::UnprocessedUserLine;
use crate::UserValue;
use chrono::Datelike;
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
    cast_records: Option<Vec<CastMeta>>,
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

    /// Insert a new [`UserValue`]. Returns an error on collision. Returns an Ok on duplicate.
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

    // added for migration from SpamEntries to user_values<SpamUpdate> for test data
    pub(crate) fn labels(&self) -> Option<SpamEntries> {
        self.labels.clone()
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
            cast_records: None,
            latest_cast_record_check_date: None,
            reaction_times: None,
            latest_reaction_time_update_date: None,
            user_values: None,
        }
    }

    pub fn add_cast_records(&mut self, records: Vec<CastMeta>, check_date: NaiveDate) {
        self.cast_records = Some(records);
        self.latest_cast_record_check_date = Some(check_date);
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

    pub fn latest_spam_record(&self) -> Option<SpamRecord> {
        Some(self.labels.as_ref()?.last_spam_entry().record())
    }

    pub fn earliest_spam_record(&self) -> Option<SpamRecord> {
        Some(self.labels.as_ref()?.earliest_spam_entry().record())
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

    /// None: there is no spam_score data in the dataset.
    pub fn created_at_or_after_date_with_opt(&self, date: NaiveDate) -> Option<bool> {
        Some(self.earliest_spam_record()?.1 >= date)
    }

    pub fn created_at_or_before_date_with_opt(&self, date: NaiveDate) -> Option<bool> {
        Some(self.earliest_spam_record()?.1 <= date)
    }

    pub fn latest_reaction_time_update_date(&self) -> Option<NaiveDateTime> {
        self.latest_reaction_time_update_date
    }

    /// Adds a new spam record to a user. There are three scenarios that may happen:
    ///
    /// The user already has a spam record with the same date and the same record. The method
    /// returns Ok without doing anything.
    ///
    /// The user already has a spam record with a different spam score at the same date.
    /// The method does not change the struct and returns an error.
    ///
    /// There is no collision and the list is updated while remaining sorted.
    ///
    pub fn add_spam_record(&mut self, new_record: SpamRecord) -> Result<(), UserError> {
        let new_entry = SpamEntry::WithoutSourceCommit(new_record);
        if let Some(labels) = &mut self.labels {
            labels
                .add_spam_entry(new_entry)
                .map_err(|_| UserError::SpamScoreCollision {
                    fid: self.fid(),
                    date: new_entry.date(),
                    old_spam_score: self
                        .spam_score_at_date_with_owned(&new_entry.date())
                        .unwrap(),
                    new_spam_score: new_entry.score(),
                })
        } else {
            self.labels = Some(SpamEntries::new(new_entry));
            Ok(())
        }
    }

    pub fn cast_count(&self) -> Option<u64> {
        Some(self.cast_records.as_ref()?.len() as u64)
    }

    pub fn reaction_times(&self) -> &Option<Vec<NaiveDateTime>> {
        &self.reaction_times
    }

    pub fn average_monthly_cast_rate(&self) -> Option<f32> {
        let [sum, count] = self
            .monthly_cast_counts()?
            .iter()
            .fold([0, 0], |acc, (x, _)| [acc[0] + x, acc[1] + 1]);

        Some(sum as f32 / count as f32)
    }

    pub fn monthly_cast_counts(&self) -> Option<Vec<(usize, NaiveDate)>> {
        let cast_records = if let Some(cast_records) = &self.cast_records {
            cast_records
        } else {
            return None;
        };

        Some(
            cast_records
                .iter()
                .map(|x| {
                    NaiveDate::from_ymd_opt(x.cast_date().year(), x.cast_date().month(), 1)
                        .expect("date parsing inside monthly_cast_count should work")
                })
                .sorted()
                .dedup_with_count()
                .collect::<Vec<_>>(),
        )
    }

    pub fn has_cast_data(&self) -> bool {
        self.cast_records.is_some()
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

    /// If the user didn't exist at the date, the function returns none.
    pub fn spam_score_at_date_with_owned(&self, date: &NaiveDate) -> Option<SpamScore> {
        if date < &self.earliest_spam_record()?.1 {
            return None;
        };
        Some(self.labels.as_ref()?.spam_score_at_date(*date)?)
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

impl TryFrom<UnprocessedUserLine> for User {
    type Error = InvalidInputError;

    fn try_from(value: UnprocessedUserLine) -> Result<Self, Self::Error> {
        let label_value = SpamScore::try_from(value.label_value())?;
        let fid = value.fid();
        let date = value.date()?;
        let record: SpamRecord = (label_value, date);
        let entries = SpamEntries::new(SpamEntry::WithoutSourceCommit(record));

        Ok(Self {
            fid,
            labels: Some(entries),
            cast_records: None,
            latest_cast_record_check_date: None,
            reaction_times: None,
            latest_reaction_time_update_date: None,
            user_values: None,
        })
    }
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
