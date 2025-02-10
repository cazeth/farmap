use crate::spam_score::SpamScore;
use crate::user::User;
use crate::user::Users;
use crate::utils::distribution_from_counts;
use chrono::NaiveDate;
use std::collections::HashMap;

pub struct UsersSubset<'a> {
    map: HashMap<usize, &'a User>,
}

impl<'a> UsersSubset<'a> {
    pub fn from_filter<F>(users: &'a Users, filter: F) -> Self
    where
        F: Fn(&User) -> bool,
    {
        let mut filtered_map: HashMap<usize, &'a User> = HashMap::new();
        for user in users.iter() {
            if filter(user) {
                filtered_map.insert(user.fid(), user);
            }
        }

        Self { map: filtered_map }
    }

    /// apply filter to existing subset and mutate subset.
    pub fn filter<F>(&mut self, filter: F)
    where
        F: Fn(&User) -> bool,
    {
        self.map = self
            .map
            .values()
            .filter(|user| filter(user))
            .map(|user| (user.fid(), *user))
            .collect::<HashMap<usize, &User>>();
    }

    pub fn current_spam_score_distribution(&self) -> Option<[f32; 3]> {
        let mut counts = [0; 3];
        for (_, user) in self.map.iter() {
            match user.latest_spam_record().0 {
                SpamScore::Zero => counts[0] += 1,
                SpamScore::One => counts[1] += 1,
                SpamScore::Two => counts[2] += 1,
            }
        }

        distribution_from_counts(&counts)
    }

    pub fn user_count(&self) -> usize {
        self.map.len()
    }
}

#[cfg(test)]
mod tests {

    use chrono::NaiveDate;

    use super::*;

    #[test]
    fn from_filter_test() {
        let users = Users::create_from_dir("data/dummy-data");
        let filter = |user: &User| {
            user.earliest_spam_record().1 > NaiveDate::from_ymd_opt(2024, 6, 1).unwrap()
        };

        let subset = UsersSubset::from_filter(&users, filter);
        assert_eq!(
            subset.current_spam_score_distribution(),
            Some([0.0, 0.0, 1.0])
        );
    }

    #[test]
    fn test_user_count() {
        let users = Users::create_from_dir("data/dummy-data");
        let mut subset = UsersSubset::from_filter(&users, |_: &User| true);
        subset.filter(|user: &User| {
            !user.created_at_or_after_date(NaiveDate::from_ymd_opt(2023, 12, 29).unwrap())
        });
        assert_eq!(subset.user_count(), 0);
        let mut subset = UsersSubset::from_filter(&users, |_: &User| true);
        subset.filter(|user: &User| {
            !user.created_at_or_after_date(NaiveDate::from_ymd_opt(2024, 6, 1).unwrap())
        });
        assert_eq!(subset.user_count(), 1);
    }

    #[test]
    fn filter_test() {
        let users = Users::create_from_dir("data/dummy-data");
        let mut subset = UsersSubset::from_filter(&users, |_: &User| true);
        assert_eq!(subset.user_count(), 2);
        subset.filter(|user: &User| user.fid() != 3);
        assert_eq!(subset.user_count(), 2);
        subset.filter(|user: &User| user.fid() == 1);
        assert_eq!(subset.user_count(), 1);
    }
}
