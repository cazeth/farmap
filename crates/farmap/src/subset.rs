use crate::fid_score_shift::ShiftSource;
use crate::fid_score_shift::ShiftTarget;
use crate::spam_score::{DatedSpamScoreCount, DatedSpamScoreDistribution};
use crate::user::User;
use crate::user_collection::UserCollection;
use crate::FidScoreShift;
use chrono::Datelike;
use chrono::Days;
use chrono::Duration;
use chrono::Months;
use chrono::NaiveDate;
use chrono::NaiveDateTime;
use std::collections::HashMap;

#[derive(Clone, Debug, Default, PartialEq)]
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
        let filtered_map: HashMap<usize, &'a User> = users
            .iter()
            .filter(|user| filter(user))
            .map(|user| (user.fid(), user))
            .collect();

        let mut res = Self {
            map: filtered_map,
            earliest_spam_score_date: None,
            latest_spam_score_date: None,
        };

        res.update_earliest_spam_score_date();
        res.update_latest_spam_score_date();
        res
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

        self.update_earliest_spam_score_date();
        self.update_latest_spam_score_date();
    }

    fn update_earliest_spam_score_date(&mut self) {
        self.earliest_spam_score_date = self
            .map
            .values()
            .flat_map(|user| user.earliest_spam_score_date_with_opt())
            .min()
    }

    fn update_latest_spam_score_date(&mut self) {
        self.latest_spam_score_date = self
            .map
            .values()
            .flat_map(|user| user.latest_spam_score_date_with_opt())
            .max();
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

    /// Returns the spam score count for a set at a weekly cadence. The first value is at the
    /// earliest spam score date in the set and the last value is always the current date even if
    /// it is the fewer than seven days between it and the next-to-last value.
    pub fn weekly_spam_score_counts(&self) -> Vec<DatedSpamScoreCount> {
        if self.map.is_empty() {
            return Vec::new();
        }
        // since the struct is not empty the unwrap should never trigger.
        let mut date = self.earliest_spam_score_date.unwrap();
        let end_date = self.latest_spam_score_date.unwrap();
        let mut result: Vec<DatedSpamScoreCount> = Vec::new();
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

    pub fn into_map(self) -> HashMap<usize, &'a User> {
        self.map
    }

    pub fn drop_fid(&mut self, fid: usize) -> Option<&User> {
        self.map.get(&fid).map(|v| &**v)
    }
    pub fn spam_score_count_at_date(&self, date: NaiveDate) -> Option<DatedSpamScoreCount> {
        if date < self.earliest_spam_score_date? {
            return None;
        };

        if self.user_count() == 0 {
            return None;
        };

        Some(
            self.map
                .iter()
                .filter_map(|(_, user)| user.spam_score_at_date_with_owned(&date))
                .fold(
                    DatedSpamScoreCount::default_with_date(date),
                    |mut acc, user| {
                        acc.add(user);
                        acc
                    },
                ),
        )
    }

    /// Returns none when the set is empty
    pub fn current_spam_score_count_with_opt(&self) -> Option<DatedSpamScoreCount> {
        self.spam_score_count_at_date(self.latest_spam_score_date?)
    }

    pub fn current_spam_score_count(&self) -> DatedSpamScoreCount {
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
            if let Some(from_spam_score) = user.spam_score_at_date_with_owned(&initial_date) {
                let from_index = from_spam_score as usize;
                let to_spam_score = user.spam_score_at_date_with_owned(&end_date).unwrap(); // must be Some if
                                                                                            // intial_date
                                                                                            // is Some.
                let to_index = to_spam_score as usize;
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
            user.created_at_or_after_date_with_opt(
                initial_date.checked_add_days(Days::new(1)).unwrap(),
            )
            .unwrap_or(false)
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

    pub fn spam_score_distribution_at_date_with_dedicated_type(
        &self,
        date: NaiveDate,
    ) -> Option<DatedSpamScoreDistribution> {
        self.spam_score_count_at_date(date)?.try_map_into().ok()
    }

    /// Returns the distribution of spam scores at a certain date. Excludes users that did not
    /// exist at the given date.
    /// Returns none if the struct contains no users or if no users existed at the provided date.
    #[deprecated(
        since = "TBD",
        note = "use spam_score_distribution_at_date_with_dedicated_type instead"
    )]
    pub fn spam_score_distribution_at_date(&self, date: NaiveDate) -> Option<[f32; 3]> {
        let distributions: DatedSpamScoreDistribution =
            self.spam_score_count_at_date(date)?.try_map_into().ok()?;
        Some(distributions.into_inner().into())
    }

    /// Returns the average total casts of the users in the group along with the fraction of users
    /// in the group where this data is available. If no data is available or if the set is empty the option is none.
    pub fn average_total_casts(&self) -> Option<[f32; 2]> {
        let total = self.map.len();
        let [sum, count] = self
            .map
            .values()
            .filter_map(|x| x.cast_count())
            .fold([0, 0], |acc, x| [acc[0] + x, acc[1] + 1]);
        if count > 0 {
            Some([sum as f32 / count as f32, count as f32 / total as f32])
        } else {
            None
        }
    }

    pub fn casts_data_fill_rate(&self) -> f32 {
        let filled_count = self.iter().filter(|user| user.has_cast_data()).count();
        let total = self.user_count();
        filled_count as f32 / total as f32
    }

    pub fn reaction_times(&self) -> Option<Vec<&NaiveDateTime>> {
        if self.iter().map(|x| x.reaction_times()).all(|x| x.is_none()) {
            return None;
        };

        Some(
            self.iter()
                .flat_map(|x| x.reaction_times())
                .flat_map(|x| x.iter())
                .collect(),
        )
    }

    /// Returns a hashmap of the update count that occured at each date.
    pub fn count_updates(&self) -> HashMap<NaiveDate, usize> {
        let mut result: HashMap<NaiveDate, usize> = HashMap::new();
        for date in self
            .iter()
            .flat_map(|user| user.all_spam_records_with_opt())
            .flatten()
            .map(|(_, date)| date)
        {
            if let Some(current_count) = result.get_mut(&date) {
                *current_count += 1;
            } else {
                result.insert(date, 1);
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

    #[allow(deprecated)]
    pub fn weekly_spam_score_distributions_with_dedicated_type(
        &self,
    ) -> Vec<(NaiveDate, DatedSpamScoreDistribution)> {
        // return an empty vec if the set is empty.
        if self.map.is_empty() {
            return Vec::new();
        }

        let mut result: Vec<(NaiveDate, DatedSpamScoreDistribution)> = Vec::new();
        let mut date = self.earliest_spam_score_date.unwrap();
        let end_date = self.latest_spam_score_date.unwrap();
        while date <= end_date {
            result.push((
                date,
                self.spam_score_distribution_at_date_with_dedicated_type(date)
                    .unwrap(),
            ));
            date += Duration::days(7);
        }
        result.push((
            date,
            self.spam_score_distribution_at_date_with_dedicated_type(date)
                .unwrap(),
        ));

        result
    }

    /// Checks the distribution, starting at the date of the earliest spam score date an
    /// incrementing by seven days until the last spam score change in the data.
    #[deprecated(
        since = "0.1.2",
        note = "use weekly_spam_score_distribution_with_dedicated_type instead"
    )]
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

impl<'a> IntoIterator for UsersSubset<'a> {
    type Item = &'a User;
    type IntoIter = std::collections::hash_map::IntoValues<usize, &'a User>;

    fn into_iter(self) -> Self::IntoIter {
        self.map.into_values()
    }
}

impl<'a> From<HashMap<usize, &'a User>> for UsersSubset<'a> {
    fn from(value: HashMap<usize, &'a User>) -> Self {
        Self {
            map: value,
            ..Default::default()
        }
    }
}

impl<'a> From<&'a UserCollection> for UsersSubset<'a> {
    fn from(users: &'a UserCollection) -> Self {
        let map: HashMap<usize, &User> = users
            .data()
            .iter()
            .map(|(key, value)| (*key, value))
            .collect();

        let earliest_spam_score_date = users
            .iter()
            .flat_map(|user| user.earliest_spam_score_date_with_opt())
            .min();

        let latest_spam_score_date = users
            .iter()
            .flat_map(|user| user.latest_spam_score_date_with_opt())
            .max();

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
    use crate::user_collection::tests::dummy_data;

    #[test]
    fn empty_set() {
        let users = UserCollection::default();
        let set = UsersSubset::from(&users);
        assert_eq!(set.user_count(), 0);
    }

    pub fn create_set(collection: &UserCollection) -> UsersSubset {
        UsersSubset::from(collection)
    }

    mod test_user_count {
        use super::*;
        use crate::user::tests::test_fid::is_fid;

        #[track_caller]
        pub fn check_user_count(set: &UsersSubset, count: usize) {
            assert_eq!(set.user_count(), count);
        }

        #[test]
        fn test_user_count_with_new() {
            let users = dummy_data();
            let set = create_set(&users);
            check_user_count(&set, 2);
        }

        #[test]
        fn test_user_count_on_empty_set() {
            let users = crate::user_collection::tests::empty_collection();
            let set = create_set(&users);
            check_user_count(&set, 0);
        }

        #[test]
        fn test_user_count_before_and_after_filter() {
            let users = dummy_data();
            let mut set = create_set(&users);
            let fid_filter = |user: &User| is_fid(user, 1);
            check_user_count(&set, 2);
            test_filter::check_filter(&mut set, fid_filter);
            check_user_count(&set, 1);
        }

        #[test]
        fn test_user_count_before_and_after_filter_two() {
            let users = dummy_data();
            let mut set = create_set(&users);
            let fid_filter = |user: &User| !is_fid(user, 3);
            check_user_count(&set, 2);
            test_filter::check_filter(&mut set, fid_filter);
            check_user_count(&set, 2);
        }
    }

    mod test_filter {
        use super::*;

        #[track_caller]
        pub fn check_filter(set: &mut UsersSubset, filter: impl Fn(&User) -> bool) {
            set.filter(filter);
        }
    }
}
