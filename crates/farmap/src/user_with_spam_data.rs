use crate::is_user::IsUser;
use crate::spam_score::DatedSpamUpdate;
use crate::SpamScore;
use crate::{fetch::ConversionError, User};
use chrono::NaiveDate;
use itertools::Itertools;

/// A User guaranteed to have at least one [SpamUpdate](crate::spam_score::SpamUpdate).
#[derive(Debug, Clone, Hash, PartialEq, Eq)]
pub struct UserWithSpamData<'a> {
    user: &'a User,
}

impl<'a> TryFrom<&'a User> for UserWithSpamData<'a> {
    type Error = ConversionError;
    fn try_from(value: &'a User) -> Result<Self, Self::Error> {
        optioned_user_to_user_with_spam_data_conversion(value)
            .ok_or(ConversionError::ConversionError)
    }
}

impl<'a> UserWithSpamData<'a> {
    pub fn spam_score_at_date(&self, date: NaiveDate) -> Option<SpamScore> {
        self.dated_spam_updates()
            .iter()
            .filter(|x| x.date() <= date)
            .max_by_key(|x| x.date())
            .map(|update| update.score())
    }

    pub fn dated_spam_updates(&self) -> Vec<&DatedSpamUpdate> {
        self.user
            .all_user_values()
            .as_ref()
            .expect("cannot be empty")
            .iter()
            .flat_map(|user_value| user_value.0.specify_ref::<DatedSpamUpdate>())
            .collect_vec()
    }

    pub fn user(&self) -> &'a User {
        self.user
    }

    pub fn fid(&self) -> usize {
        self.user.fid()
    }

    pub fn earliest_spam_update(&self) -> DatedSpamUpdate {
        **self
            .dated_spam_updates()
            .iter()
            .min_by_key(|user| user.date())
            .expect("cannot be empty")
    }

    pub fn latest_spam_update(&self) -> DatedSpamUpdate {
        **self
            .dated_spam_updates()
            .iter()
            .max_by_key(|user| user.date())
            .expect("cannot be empty")
    }
}

fn optioned_user_to_user_with_spam_data_conversion(value: &User) -> Option<UserWithSpamData> {
    if value
        .all_user_values()
        .as_ref()?
        .iter()
        .flat_map(|user_value| user_value.0.specify_ref::<DatedSpamUpdate>())
        .count()
        != 0
    {
        Some(UserWithSpamData { user: value })
    } else {
        None
    }
}

impl<'a> IsUser<'a> for UserWithSpamData<'a> {
    fn fid(&self) -> usize {
        self.fid()
    }

    fn user(&self) -> &'a User {
        self.user
    }
}

#[cfg(test)]
pub mod tests {
    use crate::spam_score::DatedSpamUpdate;
    use crate::spam_score::SpamScore;
    use crate::user::tests::create_user;
    use crate::user::tests::valid_user_value_add;
    use crate::User;
    use chrono::Days;
    use chrono::NaiveDate;

    use super::UserWithSpamData;
    use crate::user_collection::tests::dummy_data;

    pub fn create_user_with_m_spam_scores(
        fid: u64,
        m: u64,
        first_spam_score_date: NaiveDate,
    ) -> User {
        let mut user = User::new_without_labels(fid as usize);
        let mut date = first_spam_score_date;

        for i in 0..m {
            let spam_update =
                DatedSpamUpdate::from(date, SpamScore::try_from((i % 3) as usize).unwrap());
            user.add_user_value(spam_update)
                .expect("should not cause collision");
            date = date.checked_add_days(Days::new(1)).unwrap();
        }
        user
    }

    pub fn valid_spam_user(user: &User) -> UserWithSpamData {
        UserWithSpamData::try_from(user).unwrap()
    }

    pub fn add_spam_score(user: &mut User, spam_score: u64, date: &str) {
        let date = NaiveDate::parse_from_str(date, "%Y-%m-%d").unwrap();
        let spam_score = SpamScore::try_from(spam_score as usize).unwrap();
        let dated_spam_update =
            <DatedSpamUpdate as From<(SpamScore, NaiveDate)>>::from((spam_score, date));
        valid_user_value_add(user, dated_spam_update);
    }

    mod test_spam_score_at_date {
        use super::*;

        fn check_spam_score_at_date(
            user: &UserWithSpamData,
            spam_score: Option<usize>,
            date: &str,
        ) {
            let date = NaiveDate::parse_from_str(date, "%Y-%m-%d").unwrap();
            assert_eq!(
                user.spam_score_at_date(date),
                spam_score.map(|score| SpamScore::try_from(score).unwrap())
            );
        }

        #[test]
        fn test_user_with_single_spam_score() {
            let mut user = create_user(1);
            add_spam_score(&mut user, 1, "2024-03-05");
            let spam_user = valid_spam_user(&user);
            check_spam_score_at_date(&spam_user, None, "2024-03-04");
            check_spam_score_at_date(&spam_user, Some(1), "2024-03-05");
        }

        #[test]
        pub fn test_spam_scores_on_dummy_data() {
            let collection = dummy_data();
            let user = valid_spam_user(collection.user(1).unwrap());

            check_spam_score_at_date(&user, None, "2023-01-25");
            check_spam_score_at_date(&user, Some(1), "2024-01-25");
            check_spam_score_at_date(&user, Some(1), "2025-01-20");
            check_spam_score_at_date(&user, Some(0), "2025-01-23");
            check_spam_score_at_date(&user, Some(0), "2025-01-25");
        }
    }

    pub mod earliest_spam_update {

        use super::*;

        fn check_earliest_spam_update_date(user: &UserWithSpamData, date: &str) {
            let date = NaiveDate::parse_from_str(date, "%Y-%m-%d").unwrap();
            assert_eq!(user.earliest_spam_update().date(), date);
        }

        pub fn earliest_spam_date_before_date_filter(
            user: &UserWithSpamData,
            date: NaiveDate,
        ) -> bool {
            user.earliest_spam_update().date() <= date
        }

        #[test]
        pub fn dummy_data_earliest_spam_date() {
            let collection = dummy_data();
            let user = valid_spam_user(collection.user(1).unwrap());
            check_earliest_spam_update_date(&user, "2024-01-01");
            let user = valid_spam_user(collection.user(2).unwrap());
            check_earliest_spam_update_date(&user, "2025-01-23");
        }
    }
}
