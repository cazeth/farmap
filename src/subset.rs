use crate::spam_score::SpamScore;
use crate::user::User;
use crate::user::Users;
use crate::utils::distribution_from_counts;
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
}
