use crate::spam_score::SpamScore;
use crate::user::User;
use crate::user::Users;
use crate::utils::distribution_from_counts;
use chrono::Days;
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

    /// Returns a matrix that records the spam score changes between two dates. If matrix[i][j] = 1
    /// it means that 1 user has moved from spam score i to spam score j during the period.
    pub fn spam_change_matrix(&self, initial_date: NaiveDate, days: Days) -> [[usize; 3]; 3] {
        let end_date = initial_date
            .checked_add_days(days)
            .unwrap_or(NaiveDate::MAX);

        let mut result: [[usize; 3]; 3] = [[0; 3]; 3];

        for user in self.map.values() {
            if let Some(from_spam_score) = user.spam_score_at_date(&initial_date) {
                let from_index = *from_spam_score as usize;
                let to_spam_score = user.spam_score_at_date(&end_date).unwrap(); // must be Some if
                                                                                 // intial_date
                                                                                 // is Some.
                let to_index = *to_spam_score as usize;
                result[from_index][to_index] += 1;
            }
        }

        result
    }

    /// Returns the distribution of spam scores at a certain date. Excludes users that did not
    /// exist at the given date.
    /// Returns none if the struct contains no users or if no users existed at the provided date.
    pub fn spam_score_distribution_at_date(&self, date: NaiveDate) -> Option<[f32; 3]> {
        let mut counts = [0; 3];

        for spam_score in self
            .map
            .iter()
            .filter_map(|(_, user)| user.spam_score_at_date(&date))
        {
            match spam_score {
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

    pub fn user(&self, fid: usize) -> Option<&User> {
        self.map.get(&fid).copied()
    }
}

impl<'a> From<&'a Users> for UsersSubset<'a> {
    fn from(users: &'a Users) -> Self {
        let map: HashMap<usize, &User> = users
            .data()
            .iter()
            .map(|(key, value)| (*key, value))
            .collect();
        Self { map }
    }
}

#[cfg(test)]
mod tests {

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
        let mut set = UsersSubset::from(&users);
        set.filter(|user: &User| {
            !user.created_at_or_after_date(NaiveDate::from_ymd_opt(2023, 12, 29).unwrap())
        });
        assert_eq!(set.user_count(), 0);
        let mut set = UsersSubset::from_filter(&users, |_: &User| true);
        set.filter(|user: &User| {
            !user.created_at_or_after_date(NaiveDate::from_ymd_opt(2024, 6, 1).unwrap())
        });
        assert_eq!(set.user_count(), 1);
    }

    #[test]
    fn filter_test() {
        let users = Users::create_from_dir("data/dummy-data");
        let mut set = UsersSubset::from(&users);
        assert_eq!(set.user_count(), 2);
        set.filter(|user: &User| user.fid() != 3);
        assert_eq!(set.user_count(), 2);
        set.filter(|user: &User| user.fid() == 1);
        assert_eq!(set.user_count(), 1);
    }

    #[test]
    fn test_spam_score_distribution_at_date() {
        let users = Users::create_from_dir("data/dummy-data");
        assert_eq!(users.user_count(), 2);
        let subset = UsersSubset::from_filter(&users, |user: &User| {
            user.created_at_or_after_date(NaiveDate::from_ymd_opt(2024, 6, 1).unwrap())
        });

        assert!(subset
            .spam_score_distribution_at_date(NaiveDate::from_ymd_opt(2024, 6, 1).unwrap())
            .is_none(),);

        assert_eq!(
            subset
                .spam_score_distribution_at_date(NaiveDate::from_ymd_opt(2025, 1, 23).unwrap())
                .unwrap(),
            [0.0, 0.0, 1.0]
        );
    }

    #[test]
    fn test_spam_change_matrix() {
        let users = Users::create_from_dir("data/dummy-data");
        let set = UsersSubset::from(&users);
        let change_matrix =
            set.spam_change_matrix(NaiveDate::from_ymd_opt(2024, 1, 1).unwrap(), Days::new(700));
        assert_eq!(change_matrix, [[0, 0, 0], [1, 0, 0], [0, 0, 0]]);
        let change_matrix = set.spam_change_matrix(
            NaiveDate::from_ymd_opt(2025, 1, 23).unwrap(),
            Days::new(700),
        );
        assert_eq!(change_matrix, [[1, 0, 0], [0, 0, 0], [0, 0, 1]]);
    }

    #[test]
    fn test_get_user() {
        let users = Users::create_from_dir("data/dummy-data");
        let set = UsersSubset::from(&users);
        assert!(set.user(3).is_none());
        assert_eq!(
            set.user(1).unwrap().earliest_spam_record().1,
            NaiveDate::from_ymd_opt(2024, 1, 1).unwrap()
        );

        assert_eq!(
            set.user(2).unwrap().earliest_spam_record().1,
            NaiveDate::from_ymd_opt(2025, 1, 23).unwrap()
        );
    }

    #[test]
    fn test_full_set_from_data() {
        let users = Users::create_from_dir("data/dummy-data");
        let set = UsersSubset::from(&users);
        assert_eq!(users.user_count(), set.user_count());
    }
}
