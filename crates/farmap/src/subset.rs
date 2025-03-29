use crate::fid_score_shift::ShiftSource;
use crate::fid_score_shift::ShiftTarget;
use crate::spam_score::SpamScore;
use crate::spam_score::SpamScoreCount;
use crate::user::User;
use crate::user_collection::UserCollection;
use crate::utils::distribution_from_counts;
use crate::FidScoreShift;
use chrono::Datelike;
use chrono::Days;
use chrono::Duration;
use chrono::Months;
use chrono::NaiveDate;
use std::collections::HashMap;

#[derive(Clone)]
pub struct UsersSubset<'a> {
    map: HashMap<usize, &'a User>,
    earliest_spam_score_date: Option<NaiveDate>,
    latest_spam_score_date: Option<NaiveDate>,
}

impl<'a> UsersSubset<'a> {
    pub fn from_filter<F>(users: &'a UserCollection, filter: F) -> Self
    where
        F: Fn(&User) -> bool,
    {
        let mut earliest_spam_score_date: Option<NaiveDate> = None;
        let mut latest_spam_score_date: Option<NaiveDate> = None;

        let mut filtered_map: HashMap<usize, &'a User> = HashMap::new();
        for user in users.iter() {
            if filter(user) {
                filtered_map.insert(user.fid(), user);
                if earliest_spam_score_date.unwrap_or(NaiveDate::MAX)
                    > user.earliest_spam_score_date()
                {
                    earliest_spam_score_date = Some(user.earliest_spam_score_date())
                }
                if latest_spam_score_date.unwrap_or(NaiveDate::MIN)
                    < user.last_spam_score_update_date()
                {
                    latest_spam_score_date = Some(user.last_spam_score_update_date())
                }
            }
        }

        Self {
            map: filtered_map,
            earliest_spam_score_date,
            latest_spam_score_date,
        }
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

        // update earliest_spam_score
        let mut earliest_spam_score_date: Option<NaiveDate> = None;
        let mut latest_spam_score_date: Option<NaiveDate> = None;

        for user in self.map.values() {
            if user.earliest_spam_score_date() < earliest_spam_score_date.unwrap_or(NaiveDate::MAX)
            {
                self.earliest_spam_score_date = Some(user.earliest_spam_score_date());
                earliest_spam_score_date = Some(user.earliest_spam_score_date());
            };

            if user.last_spam_score_update_date() > latest_spam_score_date.unwrap_or(NaiveDate::MIN)
            {
                self.latest_spam_score_date = Some(user.last_spam_score_update_date());
                latest_spam_score_date = Some(user.last_spam_score_update_date())
            }
        }
    }

    /// return a new struct with filter applied
    pub fn filtered<F>(&self, filter: F) -> Self
    where
        F: Fn(&User) -> bool,
    {
        let mut new = self.clone();
        new.filter(filter);
        new
    }

    /// Returns none if the subset is empty
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

    /// Returns the spam score count for a set at a weekly cadence. The first value is at the
    /// earliest spam score date in the set and the last value is always the current date even if
    /// it is the fewer than seven days between it and the next-to-last value.
    pub fn weekly_spam_score_counts(&self) -> Vec<SpamScoreCount> {
        if self.map.is_empty() {
            return Vec::new();
        }
        // since the struct is not empty the unwrap should never trigger.
        let mut date = self.earliest_spam_score_date.unwrap();
        let end_date = self.latest_spam_score_date.unwrap();
        let mut result: Vec<SpamScoreCount> = Vec::new();
        while date <= end_date {
            result.push(self.spam_score_count_at_date(date).unwrap());
            date += Duration::days(7);
        }

        // always include the last date.
        if date < end_date {
            // since end date is a valid date the unwrap should never trigger.
            result.push(self.spam_score_count_at_date(end_date).unwrap());
        };

        result
    }

    pub fn spam_score_count_at_date(&self, date: NaiveDate) -> Option<SpamScoreCount> {
        if date < self.earliest_spam_score_date? {
            return None;
        };
        if self.user_count() == 0 {
            return None;
        };

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
        Some(SpamScoreCount::new(date, counts[0], counts[1], counts[2]))
    }

    pub fn current_spam_score_count(&self) -> SpamScoreCount {
        let date = self.latest_spam_score_date.unwrap();
        self.spam_score_count_at_date(date).unwrap()
    }

    /// Returns a matrix that records the spam score changes between two dates. If matrix[i][j] = 1
    /// it means that 1 user has moved from spam score i to spam score j during the period.
    #[doc(hidden)]
    #[deprecated(note = "use spam changes with fid score shift instead")]
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

    pub fn spam_changes_with_fid_score_shift(
        &self,
        initial_date: NaiveDate,
        days: Days,
    ) -> Vec<FidScoreShift> {
        #[allow(deprecated)]
        let matrix = self.spam_change_matrix(initial_date, days);
        let mut shifts: Vec<FidScoreShift> = Vec::new();
        let sources = [
            ShiftSource::Zero,
            ShiftSource::One,
            ShiftSource::Two,
            ShiftSource::New,
        ];
        let targets = [ShiftTarget::Zero, ShiftTarget::One, ShiftTarget::Two];
        for (i, source) in sources.iter().enumerate().take(3) {
            for (j, target) in targets.iter().enumerate() {
                if matrix[i][j] > 0 {
                    shifts.push(FidScoreShift::new(*source, *target, matrix[i][j]))
                };
            }
        }

        // also add new users.

        let new_users = self.filtered(|user: &User| {
            user.created_at_or_after_date(initial_date.checked_add_days(Days::new(1)).unwrap())
        });

        let new_user_counts = new_users.spam_score_count_at_date(
            initial_date
                .checked_add_days(days)
                .unwrap_or(NaiveDate::MAX),
        );

        if let Some(counts) = new_user_counts {
            if counts.spam() != 0 {
                shifts.push(FidScoreShift::new(
                    ShiftSource::New,
                    ShiftTarget::Zero,
                    counts.spam() as usize,
                ));
            }

            if counts.maybe_spam() != 0 {
                shifts.push(FidScoreShift::new(
                    ShiftSource::New,
                    ShiftTarget::One,
                    counts.maybe_spam() as usize,
                ))
            }

            if counts.non_spam() != 0 {
                shifts.push(FidScoreShift::new(
                    ShiftSource::New,
                    ShiftTarget::Two,
                    counts.non_spam() as usize,
                ))
            }
        }

        shifts
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

    /// Returns a hashmap of the update count that occured at each date.
    pub fn count_updates(&self) -> HashMap<NaiveDate, usize> {
        let mut result: HashMap<NaiveDate, usize> = HashMap::new();
        for date in self
            .iter()
            .flat_map(|user| user.all_spam_records())
            .map(|(_, date)| date)
        {
            if let Some(current_count) = result.get_mut(date) {
                *current_count += 1;
            } else {
                result.insert(*date, 1);
            }
        }
        result
    }

    /// Checks the distribution at each month from the first spam score that exists in the set to
    /// the last. The check is done the first of each month.
    pub fn monthly_spam_score_distributions(&self) -> Vec<(NaiveDate, [f32; 3])> {
        // return an empty vec if the set is empty.
        if self.map.is_empty() {
            return Vec::new();
        }

        let mut result: Vec<(NaiveDate, [f32; 3])> = Vec::new();
        let mut date = self.earliest_spam_score_date.unwrap();
        let end_date = self.latest_spam_score_date.unwrap();
        let date_of_month = 1; // determines which date of the month the check is done.
        while date <= end_date {
            result.push((date, self.spam_score_distribution_at_date(date).unwrap()));
            if date.day0() != 0 {
                date = date.with_day(date_of_month).unwrap();
            }
            date = date.checked_add_months(Months::new(1)).unwrap();
        }
        result.push((date, self.spam_score_distribution_at_date(date).unwrap()));

        result
    }

    /// Checks the distribution, starting at the date of the earliest spam score date an
    /// incrementing by seven days until the last spam score change in the data.
    pub fn weekly_spam_score_distributions(&self) -> Vec<(NaiveDate, [f32; 3])> {
        // return an empty vec if the set is empty.
        if self.map.is_empty() {
            return Vec::new();
        }

        let mut result: Vec<(NaiveDate, [f32; 3])> = Vec::new();
        let mut date = self.earliest_spam_score_date.unwrap();
        let end_date = self.latest_spam_score_date.unwrap();
        while date <= end_date {
            result.push((date, self.spam_score_distribution_at_date(date).unwrap()));
            date += Duration::days(7);
        }
        result.push((date, self.spam_score_distribution_at_date(date).unwrap()));

        result
    }

    pub fn user_count(&self) -> usize {
        self.map.len()
    }

    pub fn user(&self, fid: usize) -> Option<&User> {
        self.map.get(&fid).copied()
    }

    pub fn iter(&self) -> impl Iterator<Item = &User> {
        self.map.values().copied()
    }
}

impl<'a> From<&'a UserCollection> for UsersSubset<'a> {
    fn from(users: &'a UserCollection) -> Self {
        let map: HashMap<usize, &User> = users
            .data()
            .iter()
            .map(|(key, value)| (*key, value))
            .collect();

        let mut earliest_spam_score_date: Option<NaiveDate> = None;
        let mut latest_spam_score_date: Option<NaiveDate> = None;

        for user in users.iter() {
            if user.earliest_spam_score_date() < earliest_spam_score_date.unwrap_or(NaiveDate::MAX)
            {
                earliest_spam_score_date = Some(user.earliest_spam_score_date());
            }

            if user.last_spam_score_update_date() > latest_spam_score_date.unwrap_or(NaiveDate::MIN)
            {
                latest_spam_score_date = Some(user.last_spam_score_update_date());
            }
        }

        Self {
            map,
            earliest_spam_score_date,
            latest_spam_score_date,
        }
    }
}

#[cfg(test)]
mod tests {

    use super::*;

    #[test]
    fn from_filter_test_new() {
        let users = UserCollection::create_from_dir_with_res("data/dummy-data").unwrap();
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
    fn test_filtered() {
        let users = UserCollection::create_from_dir_with_res("data/dummy-data").unwrap();
        let filter = |user: &User| {
            user.earliest_spam_record().1 > NaiveDate::from_ymd_opt(2024, 6, 1).unwrap()
        };

        let mut full_set = UsersSubset::from(&users);
        let filtered_set = full_set.filtered(filter).current_spam_score_distribution();
        full_set.filter(filter);
        assert_eq!(filtered_set, full_set.current_spam_score_distribution());
    }

    // this test has been replaced with from_filter_test_new and will be removed once deprecated
    // methods are removed
    #[test]
    #[allow(deprecated)]
    fn from_filter_test() {
        let users = UserCollection::create_from_dir("data/dummy-data");
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
    fn test_current_spam_score_count() {
        let users = UserCollection::create_from_dir_with_res("data/dummy-data").unwrap();
        let set = UsersSubset::from(&users);
        assert_eq!(set.current_spam_score_count().spam(), 1);
        assert_eq!(set.current_spam_score_count().non_spam(), 1);
        assert_eq!(set.current_spam_score_count().maybe_spam(), 0);
    }

    #[test]
    fn test_user_count_with_new() {
        let users = UserCollection::create_from_dir_with_res("data/dummy-data").unwrap();
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
    fn test_earliest_date() {
        let users = UserCollection::create_from_dir_with_res("data/dummy-data").unwrap();
        let set = UsersSubset::from(&users);
        let date = NaiveDate::from_ymd_opt(2024, 1, 1).unwrap();
        assert_eq!(set.earliest_spam_score_date.unwrap(), date);
    }

    #[test]
    fn test_earliest_date_after_filter() {
        let users = UserCollection::create_from_dir_with_res("data/dummy-data").unwrap();
        let mut set = UsersSubset::from(&users);
        let filter_date = NaiveDate::from_ymd_opt(2025, 1, 1).unwrap();
        set.filter(|user: &User| user.created_at_or_after_date(filter_date));
        assert_eq!(
            set.earliest_spam_score_date.unwrap(),
            NaiveDate::from_ymd_opt(2025, 1, 23).unwrap()
        );
    }

    #[test]
    fn test_latest_data() {
        let users = UserCollection::create_from_dir_with_res("data/dummy-data").unwrap();
        let set = UsersSubset::from(&users);
        let date = NaiveDate::from_ymd_opt(2025, 1, 23).unwrap();
        assert_eq!(set.latest_spam_score_date.unwrap(), date);
    }

    #[test]
    #[allow(deprecated)]
    fn test_user_count() {
        let users = UserCollection::create_from_dir("data/dummy-data");
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
    fn filter_test_with_new() {
        let users = UserCollection::create_from_dir_with_res("data/dummy-data").unwrap();
        let mut set = UsersSubset::from(&users);
        assert_eq!(set.user_count(), 2);
        set.filter(|user: &User| user.fid() != 3);
        assert_eq!(set.user_count(), 2);
        set.filter(|user: &User| user.fid() == 1);
        assert_eq!(set.user_count(), 1);
    }

    #[test]
    fn test_dates_in_monthly_spam_score_distributions() {
        let users = UserCollection::create_from_dir_with_res("data/dummy-data").unwrap();
        let set = UsersSubset::from(&users);
        let monthly_distributions = set.monthly_spam_score_distributions();
        assert_eq!(
            monthly_distributions.first().unwrap().0,
            NaiveDate::from_ymd_opt(2024, 1, 1).unwrap()
        );

        assert_eq!(
            monthly_distributions.last().unwrap().0,
            NaiveDate::from_ymd_opt(2025, 2, 1).unwrap()
        );
    }

    #[test]
    fn test_weekly_spam_score_counts() {
        let users =
            UserCollection::create_from_file_with_res("data/dummy-data/spam_2.jsonl").unwrap();
        let set = UsersSubset::from(&users);
        let result = set.weekly_spam_score_counts();
        assert_eq!(result.len(), 1);
    }

    #[test]
    fn test_dates_in_weekly_spam_score_distributions() {
        let users = UserCollection::create_from_dir_with_res("data/dummy-data").unwrap();
        let set = UsersSubset::from(&users);
        let weekly_distributions = set.weekly_spam_score_distributions();
        assert_eq!(
            weekly_distributions.first().unwrap().0,
            NaiveDate::from_ymd_opt(2024, 1, 1).unwrap()
        );

        assert!(
            weekly_distributions.last().unwrap().0 >= NaiveDate::from_ymd_opt(2025, 1, 23).unwrap()
        );

        assert!(
            weekly_distributions.last().unwrap().0 <= NaiveDate::from_ymd_opt(2025, 1, 30).unwrap()
        );
    }

    #[test]
    #[allow(deprecated)]
    fn filter_test() {
        let users = UserCollection::create_from_dir("data/dummy-data");
        let mut set = UsersSubset::from(&users);
        assert_eq!(set.user_count(), 2);
        set.filter(|user: &User| user.fid() != 3);
        assert_eq!(set.user_count(), 2);
        set.filter(|user: &User| user.fid() == 1);
        assert_eq!(set.user_count(), 1);
    }

    #[test]
    fn test_spam_score_distribution_at_date_with_new() {
        let users = UserCollection::create_from_dir_with_res("data/dummy-data").unwrap();
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
    #[allow(deprecated)]
    fn test_spam_score_distribution_at_date() {
        let users = UserCollection::create_from_dir("data/dummy-data");
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
    #[allow(deprecated)]
    fn test_spam_change_matrix_with_new_with_deprecated_spam() {
        let users = UserCollection::create_from_dir_with_res("data/dummy-data").unwrap();
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
    fn test_spam_change_with_new() {
        let users = UserCollection::create_from_dir_with_res("data/dummy-data").unwrap();
        let set = UsersSubset::from(&users);
        let shifts = set.spam_changes_with_fid_score_shift(
            NaiveDate::from_ymd_opt(2024, 1, 1).unwrap(),
            Days::new(700),
        );
        let expected_shift = FidScoreShift::new(ShiftSource::One, ShiftTarget::Zero, 1);
        let expected_new = FidScoreShift::new(ShiftSource::New, ShiftTarget::Two, 1);
        assert!(shifts.contains(&expected_shift));
        assert!(shifts.contains(&expected_new));
        assert_eq!(shifts.len(), 2);
        let change_matrix = set.spam_changes_with_fid_score_shift(
            NaiveDate::from_ymd_opt(2025, 1, 23).unwrap(),
            Days::new(700),
        );
        let expected_shift = FidScoreShift::new(ShiftSource::Zero, ShiftTarget::Zero, 1);
        assert_eq!(change_matrix[0], expected_shift);
    }

    #[test]
    #[allow(deprecated)]
    fn test_spam_change_matrix() {
        let users = UserCollection::create_from_dir("data/dummy-data");
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
    fn test_get_user_with_new() {
        let users = UserCollection::create_from_dir_with_res("data/dummy-data").unwrap();
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
    #[allow(deprecated)]
    fn test_get_user() {
        let users = UserCollection::create_from_dir("data/dummy-data");
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
    fn test_full_set_from_data_with_new() {
        let users = UserCollection::create_from_dir_with_res("data/dummy-data").unwrap();
        let set = UsersSubset::from(&users);
        assert_eq!(users.user_count(), set.user_count());
    }

    #[test]
    #[allow(deprecated)]
    fn test_full_set_from_data() {
        let users = UserCollection::create_from_dir("data/dummy-data");
        let set = UsersSubset::from(&users);
        assert_eq!(users.user_count(), set.user_count());
    }

    #[test]
    fn test_update_counts() {
        let users = UserCollection::create_from_dir_with_res("data/dummy-data").unwrap();
        let set = UsersSubset::from(&users);
        let result = set.count_updates();
        let sum: usize = result.values().sum();
        assert_eq!(sum, 3);
    }
}
