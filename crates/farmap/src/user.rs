use crate::cast_meta::CastMeta;
use crate::spam_score::SpamScore;
use crate::UnprocessedUserLine;
use chrono::DateTime;
use chrono::Datelike;
use chrono::Local;
use chrono::NaiveDate;
use chrono::NaiveDateTime;
use itertools::*;
use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Debug, PartialEq, Serialize, Deserialize)]
pub struct User {
    fid: usize,
    labels: Vec<SpamRecord>,

    /// Some(Empty vec): has been checked and there were no cast records.
    /// None: Has not been checked.
    cast_records: Option<Vec<CastMeta>>,
    reaction_times: Option<Vec<NaiveDateTime>>,
    latest_reaction_time_update_date: Option<NaiveDateTime>,
    latest_cast_record_check_date: Option<NaiveDate>,
}

type SpamRecord = (SpamScore, NaiveDate);

impl User {
    /// This method only takes a single SpamRecord as input. Therefore it cannot fail. Add more
    /// SpamRecords with add_spam_record. This function is mostly used for testing.
    pub fn new(fid: usize, labels: SpamRecord) -> Self {
        Self {
            fid,
            labels: vec![labels],
            cast_records: None,
            latest_cast_record_check_date: None,
            reaction_times: None,
            latest_reaction_time_update_date: None,
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

    /// Returns the fid of the user
    /// # Examples
    /// ```rust
    /// use farmap::User;
    /// use chrono::NaiveDate;
    /// use farmap::SpamScore;
    ///
    /// let user = User::new(1, (SpamScore::Zero, NaiveDate::from_ymd_opt(2020,1,1).unwrap()) );
    /// assert_eq!(user.fid(), 1);
    /// ````
    pub fn fid(&self) -> usize {
        self.fid
    }

    pub fn latest_spam_record(&self) -> Option<SpamRecord> {
        Some(*self.labels.last()?)
    }

    pub fn earliest_spam_record(&self) -> Option<SpamRecord> {
        Some(*self.labels.first()?)
    }

    #[deprecated(note = "use latest_spam_record instead")]
    pub fn latest_spam_record_with_opt(&self) -> Option<&SpamRecord> {
        self.labels.last()
    }

    #[deprecated(note = "use earliest_spam_record instead")]
    pub fn earliest_spam_record_with_opt(&self) -> Option<&SpamRecord> {
        self.labels.first()
    }

    pub fn all_spam_records_with_opt(&self) -> Option<Vec<SpamRecord>> {
        Some(self.labels.clone())
    }

    #[deprecated(note = "use all_spam_records_with_opt instead")]
    pub fn all_spam_records(&self) -> &Vec<SpamRecord> {
        &self.labels
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
        let position_closest_and_smaller: Option<usize> =
            self.labels.iter().position(|(_, d)| *d >= new_record.1);

        let record_closest_and_smaller = position_closest_and_smaller.map(|p| self.labels[p]);

        match record_closest_and_smaller {
            Some((value, date)) if date == new_record.1 && value == new_record.0 => Ok(()),
            None => {
                self.labels.push(new_record);
                Ok(())
            }
            Some((_, date)) if date != new_record.1 => {
                self.labels
                    .insert(position_closest_and_smaller.unwrap(), new_record);
                Ok(())
            }
            Some(_) => Err(UserError::SpamScoreCollision {
                fid: self.fid,
                date: new_record.1,
                old_spam_score: record_closest_and_smaller.unwrap().0,
                new_spam_score: new_record.0,
            }),
        }
    }

    /// Merges two user objects into one. Both must have the same FID and no contradictory spam
    /// records (i.e same date but different scores)
    #[deprecated(note = "use add spam record instead")]
    pub fn merge_user(&mut self, other: Self) -> Result<(), UserError> {
        if self.fid != other.fid() {
            return Err(UserError::DifferentFidMerge {
                fid_1: self.fid,
                fid_2: other.fid,
            });
        };

        for spam_record in other.all_spam_records_with_opt().unwrap() {
            self.add_spam_record(spam_record)?
        }

        Ok(())
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
        Some(self.labels.last()?.1)
    }

    pub fn earliest_spam_score_date_with_opt(&self) -> Option<NaiveDate> {
        self.labels.iter().map(|(_, date)| date).min().copied()
    }

    pub fn latest_spam_score_date_with_opt(&self) -> Option<NaiveDate> {
        self.labels.iter().map(|(_, date)| date).max().copied()
    }

    /// If the user didn't exist at the date, the function returns none.
    #[deprecated(note = "use spam_score_at_date_with_owned instead")]
    pub fn spam_score_at_date(&self, date: &NaiveDate) -> Option<&SpamScore> {
        if date < &self.earliest_spam_record()?.1 {
            return None;
        };

        self.labels
            .iter()
            .rev()
            .find(|(_, d)| d <= date)
            .map(|(score, _)| score)
    }

    /// If the user didn't exist at the date, the function returns none.
    pub fn spam_score_at_date_with_owned(&self, date: &NaiveDate) -> Option<SpamScore> {
        if date < &self.earliest_spam_record()?.1 {
            return None;
        };

        self.labels
            .iter()
            .copied()
            .rev()
            .find(|(_, d)| d <= date)
            .map(|(score, _)| score)
    }
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
        let date = if let Some(date) =
            DateTime::from_timestamp(value.timestamp().try_into().unwrap(), 0)
        {
            date.date_naive()
        } else {
            return Err(InvalidInputError::DateError {
                timestamp: value.timestamp(),
            });
        };

        let labels: Vec<(SpamScore, NaiveDate)> = vec![(label_value, date)];

        Ok(Self {
            fid,
            labels,
            cast_records: None,
            latest_cast_record_check_date: None,
            reaction_times: None,
            latest_reaction_time_update_date: None,
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
    use chrono::NaiveTime;
    use std::path::PathBuf;

    use super::*;
    use crate::user_collection::UserCollection;

    fn check_created_at_or_before_date(user: &User, year: u32, month: u32, date: u32) -> bool {
        user.created_at_or_before_date_with_opt(
            NaiveDate::from_ymd_opt(year as i32, month, date).unwrap(),
        )
        .unwrap()
    }

    fn check_created_at_or_after_date(user: &User, year: u32, month: u32, date: u32) -> bool {
        user.created_at_or_after_date_with_opt(
            NaiveDate::from_ymd_opt(year as i32, month, date).unwrap(),
        )
        .unwrap()
    }

    fn check_spam_score_at_date(
        user: &User,
        year: u32,
        month: u32,
        date: u32,
        spam_score: Option<SpamScore>,
    ) {
        assert_eq!(
            user.spam_score_at_date_with_owned(
                &NaiveDate::from_ymd_opt(year as i32, month, date).unwrap()
            ),
            spam_score
        )
    }

    fn check_latest_reaction_time(user: &User, time: NaiveDateTime) {
        assert_eq!(*user.latest_reaction_time().unwrap(), time);
    }

    fn dummy_data_users_with_reaction_times() -> UserCollection {
        let mut users = UserCollection::default();
        let label = NaiveDate::from_ymd_opt(2024, 1, 1).unwrap();
        let reaction_date = NaiveDate::from_ymd_opt(2025, 2, 1).unwrap();
        let reaction_time = NaiveTime::from_hms_opt(10, 1, 10).unwrap();
        let reaction_datetime = NaiveDateTime::new(reaction_date, reaction_time);
        let check_date = NaiveDate::from_ymd_opt(2025, 4, 1).unwrap();
        let check_time = NaiveTime::from_hms_opt(10, 1, 10).unwrap();
        let check_datetime = NaiveDateTime::new(check_date, check_time);
        let user = User {
            fid: 1,
            labels: vec![(SpamScore::Zero, label)],
            reaction_times: Some(vec![reaction_datetime]),
            latest_reaction_time_update_date: Some(check_datetime),
            cast_records: None,
            latest_cast_record_check_date: None,
        };
        users.push_with_res(user).unwrap();
        let label = NaiveDate::from_ymd_opt(2024, 1, 1).unwrap();
        let first_reaction_date = NaiveDate::from_ymd_opt(2025, 4, 1).unwrap();
        let first_reaction_time = NaiveTime::from_hms_opt(10, 1, 10).unwrap();
        let first_reaction_datetime = NaiveDateTime::new(first_reaction_date, first_reaction_time);
        let second_reaction_date = NaiveDate::from_ymd_opt(2025, 5, 2).unwrap();
        let second_reaction_time = NaiveTime::from_hms_opt(10, 1, 10).unwrap();
        let second_reaction_datetime =
            NaiveDateTime::new(second_reaction_date, second_reaction_time);
        let check_date = NaiveDate::from_ymd_opt(2025, 4, 1).unwrap();
        let check_time = NaiveTime::from_hms_opt(10, 1, 10).unwrap();
        let check_datetime = NaiveDateTime::new(check_date, check_time);
        let user = User {
            fid: 2,
            labels: vec![(SpamScore::Zero, label)],
            reaction_times: Some(vec![first_reaction_datetime, second_reaction_datetime]),
            latest_reaction_time_update_date: Some(check_datetime),
            cast_records: None,
            latest_cast_record_check_date: None,
        };
        users.push_with_res(user).unwrap();
        users
    }

    #[test]
    pub fn test_spam_score_collision_error_for_invalid_record_add() {
        let date = NaiveDate::from_ymd_opt(2020, 1, 2).unwrap();
        let later_date = NaiveDate::from_ymd_opt(2020, 1, 3).unwrap();
        let earlier_date = NaiveDate::from_ymd_opt(2020, 1, 1).unwrap();

        let spam_record = (SpamScore::One, date);
        let mut user = User {
            fid: 1,
            labels: vec![spam_record],
            cast_records: None,
            latest_cast_record_check_date: None,
            reaction_times: None,
            latest_reaction_time_update_date: None,
        };

        assert!(user.add_spam_record((SpamScore::Zero, date)).is_err());
        assert!(user.add_spam_record((SpamScore::One, date)).is_ok());
        assert_eq!(user.all_spam_records_with_opt().unwrap().len(), 1);
        assert!(user.add_spam_record((SpamScore::Two, later_date)).is_ok());
        assert_eq!(user.all_spam_records_with_opt().unwrap().len(), 2);

        //make sure spam_records are sorted
        assert_eq!(
            user.all_spam_records_with_opt().unwrap().first().unwrap().1,
            date
        );
        assert_eq!(
            user.all_spam_records_with_opt().unwrap().last().unwrap().1,
            later_date
        );

        assert!(user
            .add_spam_record((SpamScore::Zero, earlier_date))
            .is_ok());
        assert_eq!(
            user.all_spam_records_with_opt().unwrap().first().unwrap(),
            &(SpamScore::Zero, earlier_date)
        );
        assert_eq!(
            user.all_spam_records_with_opt().unwrap()[1],
            (SpamScore::One, date)
        );
        assert_eq!(
            user.all_spam_records_with_opt().unwrap().last().unwrap(),
            &(SpamScore::Two, later_date)
        );
    }

    #[test]
    pub fn test_dummy_data_import_with_new() {
        let fid = 1;
        let db_path = PathBuf::from("data/dummy-data_db.json");
        let users = UserCollection::create_from_db(&db_path).unwrap();
        assert_eq!(users.spam_score_by_fid(fid).unwrap(), SpamScore::Zero);
        let user = users.user(fid).unwrap();
        assert_eq!(
            user.earliest_spam_score_date_with_opt().unwrap(),
            NaiveDate::from_ymd_opt(2024, 1, 1).unwrap()
        );

        assert_eq!(
            user.latest_spam_score_update_date_with_opt().unwrap(),
            NaiveDate::from_ymd_opt(2025, 1, 23).unwrap()
        );
    }

    #[test]
    pub fn test_user_created_after_date_on_dummy_data_with_new() {
        let db_path = PathBuf::from("data/dummy-data_db.json");
        let users = UserCollection::create_from_db(&db_path).unwrap();
        let user = users.user(1).unwrap();

        assert!(check_created_at_or_after_date(user, 2023, 1, 1));
        assert!(check_created_at_or_after_date(user, 2024, 1, 1));
        assert!(!check_created_at_or_after_date(user, 2024, 6, 1));
        assert!(!check_created_at_or_after_date(user, 2025, 1, 1));
    }

    #[test]
    pub fn test_spam_score_by_date_on_dummy_data_with_new() {
        let db_path = PathBuf::from("data/dummy-data_db.json");
        let users = UserCollection::create_from_db(&db_path).unwrap();
        let user = users.user(1).unwrap();
        check_spam_score_at_date(user, 2023, 1, 25, None);
        check_spam_score_at_date(user, 2024, 1, 25, Some(SpamScore::One));
        check_spam_score_at_date(user, 2025, 1, 20, Some(SpamScore::One));
        check_spam_score_at_date(user, 2025, 1, 23, Some(SpamScore::Zero));
        check_spam_score_at_date(user, 2025, 1, 25, Some(SpamScore::Zero));
    }

    #[test]
    fn test_created_by_before_date_with_new() {
        let db_path = PathBuf::from("data/dummy-data_db.json");
        let users = UserCollection::create_from_db(&db_path).unwrap();
        let user = users.user(1).unwrap();
        assert!(!check_created_at_or_before_date(user, 2023, 12, 31));
        assert!(check_created_at_or_before_date(user, 2024, 1, 1));
        assert!(check_created_at_or_before_date(user, 2024, 1, 2));
        assert!(check_created_at_or_before_date(user, 2024, 1, 2));
        assert!(check_created_at_or_before_date(user, 2025, 12, 31));

        let user = users.user(2).unwrap();
        assert!(!check_created_at_or_before_date(user, 2023, 1, 31));
        assert!(!check_created_at_or_before_date(user, 2024, 1, 1));
        assert!(!check_created_at_or_before_date(user, 2024, 1, 2));
        assert!(!check_created_at_or_before_date(user, 2025, 1, 22));
        assert!(check_created_at_or_before_date(user, 2025, 1, 23));
        assert!(check_created_at_or_before_date(user, 2025, 1, 24));
        assert!(check_created_at_or_before_date(user, 2025, 12, 31));
    }

    #[test]
    fn test_latest_reaction_time() {
        let users = dummy_data_users_with_reaction_times();
        let user = users.user(1).unwrap();
        check_latest_reaction_time(
            user,
            NaiveDateTime::new(
                NaiveDate::from_ymd_opt(2025, 2, 1).unwrap(),
                NaiveTime::from_hms_opt(10, 1, 10).unwrap(),
            ),
        );

        let user = users.user(2).unwrap();
        check_latest_reaction_time(
            user,
            NaiveDateTime::new(
                NaiveDate::from_ymd_opt(2025, 5, 2).unwrap(),
                NaiveTime::from_hms_opt(10, 1, 10).unwrap(),
            ),
        );
    }
}
