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

#[cfg(test)]
pub mod tests {
    use crate::spam_score::DatedSpamUpdate;
    use crate::spam_score::SpamScore;
    use crate::User;
    use chrono::Days;
    use chrono::NaiveDate;

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
}
