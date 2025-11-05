use crate::fid_score_shift::ShiftSource;
use crate::spam_score::DatedSpamUpdate;
use crate::DatedSpamScoreCount;
use crate::FidScoreShift;
use crate::SpamScore;
use crate::SpamScoreDistribution;
use crate::User;
use crate::UserCollection;
use crate::UserSet;
use crate::UserWithSpamData;
use crate::UsersSubset;
use chrono::Days;
use chrono::Duration;
use chrono::NaiveDate;
use itertools::Itertools;
use std::collections::hash_set::IntoIter as HashSetIntoIter;
use std::collections::HashMap;
use std::collections::HashSet;
use thiserror::Error;

/// A set of [UserWithSpamData]
#[derive(Debug, Clone, PartialEq)]
pub struct SetWithSpamEntries<'a> {
    set: UsersSubset<'a>,
    earliest_spam_score_date: NaiveDate,
    latest_spam_score_date: NaiveDate,
}

impl<'a> SetWithSpamEntries<'a> {
    /// Creates a set that contains all the [User]s with spamdata.
    /// Returns None when a [UserCollection] doesn't have [User]s that contains at least one
    /// [SpamUpdate](crate::spam_score::DatedSpamUpdate).
    pub fn new(collection: &'a UserCollection) -> Option<Self> {
        let set = UsersSubset::from_filter(collection, |user| user.has::<DatedSpamUpdate>());
        if set.user_count() == 0 {
            None
        } else {
            let earliest_spam_score_date = earliest_spam_score_date(set.iter());
            let latest_spam_score_date = latest_spam_score_date(set.iter());
            Some(Self {
                earliest_spam_score_date,
                latest_spam_score_date,
                set,
            })
        }
    }

    /// Create a new set with the filter applied. Returns None if the filter returns an empty set.
    pub fn filtered<F>(&self, filter: F) -> Option<Self>
    where
        F: Fn(&UserWithSpamData) -> bool,
    {
        let new_map: HashMap<usize, &User> = self
            .set
            .clone()
            .into_map()
            .values()
            .map(|user| UserWithSpamData::try_from(*user).expect("should not be able to fail"))
            .filter(filter)
            .map(|user| (user.fid(), user.user()))
            .collect();
        if new_map.is_empty() {
            None
        } else {
            let new_subset = UsersSubset::from(new_map);
            SetWithSpamEntries::try_from(new_subset).ok()
        }
    }

    /// Since the set should contain users with spam entries, this method returns none (and does not
    /// filter) if the filter would result in an empty set.
    pub fn filter<F>(&mut self, filter: F) -> Option<()>
    where
        F: Fn(&UserWithSpamData) -> bool,
    {
        if self.filtered(&filter).is_none() {
            None
        } else {
            let set = std::mem::take(&mut self.set);
            let new_set: HashMap<usize, &User> = set
                .into_map()
                .values()
                .map(|user| UserWithSpamData::try_from(*user).expect("should not be able to fail"))
                .filter(filter)
                .map(|user| (user.fid(), user.user()))
                .collect();

            self.set = UsersSubset::from(new_set);
            self.earliest_spam_score_date = earliest_spam_score_date(self.set.iter());
            self.latest_spam_score_date = latest_spam_score_date(self.set.iter());
            Some(())
        }
    }

    /// Returns a [UserWithSpamData] if it is in the set. Otherwise returns None.
    pub fn fid(&'a self, fid: usize) -> Option<UserWithSpamData<'a>> {
        if let Some(user) = self.set.user(fid) {
            UserWithSpamData::try_from(user).ok()
        } else {
            None
        }
    }

    /// Returns the current [SpamScoreDistribution]. The current spam score for each user is taken
    /// to be its most recent spam score.
    pub fn current_spam_score_distribution(&self) -> SpamScoreDistribution {
        let spam_score_counts = self
            .spam_score_count_at_date(self.latest_spam_score_date)
            .expect("should be a current spam score count");
        spam_score_counts
            .try_map_into::<SpamScoreDistribution>()
            .expect("todo")
            .into_inner()
    }

    /// Returns the current [DatedSpamScoreCount]. The current spam score for each user is taken
    /// to be its most recent spam score.
    pub fn current_spam_score_count(&self) -> DatedSpamScoreCount {
        self.spam_score_count_at_date(self.latest_spam_score_date)
            .expect("set should not be empty")
    }

    /// The count of users in the set that have some spam score at a date.
    pub fn user_count_with_spam_score_count_at_date(&self, date: NaiveDate) -> u64 {
        self.set
            .iter()
            .filter(|user| user_earliest_spam_score_date(user) <= date)
            .count() as u64
    }

    /// A hashmap of the spam update count that occured at each date.
    pub fn count_updates(&self) -> HashMap<NaiveDate, usize> {
        let mut result: HashMap<NaiveDate, usize> = HashMap::new();
        for date in self
            .set
            .iter()
            .flat_map(|user| user.user_values_of_kind::<DatedSpamUpdate>())
            .map(|spam_update| spam_update.date())
        {
            if let Some(current_count) = result.get_mut(&date) {
                *current_count += 1;
            } else {
                result.insert(date, 1);
            }
        }
        result
    }

    /// Total user count in the set.
    pub fn user_count(&self) -> usize {
        self.set.user_count()
    }

    /// Returns None if the provided date is prior to any spam score in the set.
    pub fn spam_score_count_at_date(&self, date: NaiveDate) -> Option<DatedSpamScoreCount> {
        if date < self.earliest_spam_score_date {
            return None;
        };

        if self.user_count() == 0 {
            return None;
        };

        Some(
            self.set
                .iter()
                .flat_map(|user| {
                    spam_score_at_date(&user.user_values_of_kind::<DatedSpamUpdate>(), date)
                })
                .fold(
                    DatedSpamScoreCount::default_with_date(date),
                    |mut acc, user| {
                        acc.add(user);
                        acc
                    },
                ),
        )
    }

    /// The changes in spam scores that have happened from a date and a number of days from that
    /// date.
    pub fn spam_changes_with_fid_score_shift(
        &self,
        initial_date: NaiveDate,
        days: Days,
    ) -> Vec<FidScoreShift> {
        let end_date = initial_date
            .checked_add_days(days)
            .unwrap_or(NaiveDate::MAX);

        let users_with_source = self
            .set
            .iter()
            .filter(|x| end_date >= user_earliest_spam_score_date(x))
            .map(|x| {
                let user_spam_updates = user_spam_updates(x);
                let user_source = spam_score_at_date(&user_spam_updates, initial_date)
                    .map(|spam_update| spam_update.into())
                    .unwrap_or(ShiftSource::New);

                let user_target = spam_score_at_date(&user_spam_updates, end_date)
                    .map(|spam_update| spam_update.into())
                    .expect("should always have spam_score_at_end");

                FidScoreShift::new(user_source, user_target, 1)
            })
            .collect_vec();

        let shift: Vec<FidScoreShift> = users_with_source.iter().fold(
            vec![
                TryInto::<FidScoreShift>::try_into(0).unwrap(),
                TryInto::<FidScoreShift>::try_into(1).unwrap(),
                TryInto::<FidScoreShift>::try_into(2).unwrap(),
                TryInto::<FidScoreShift>::try_into(3).unwrap(),
                TryInto::<FidScoreShift>::try_into(4).unwrap(),
                TryInto::<FidScoreShift>::try_into(5).unwrap(),
                TryInto::<FidScoreShift>::try_into(6).unwrap(),
                TryInto::<FidScoreShift>::try_into(7).unwrap(),
                TryInto::<FidScoreShift>::try_into(8).unwrap(),
                TryInto::<FidScoreShift>::try_into(9).unwrap(),
                TryInto::<FidScoreShift>::try_into(10).unwrap(),
                TryInto::<FidScoreShift>::try_into(11).unwrap(),
            ],
            |mut acc, shift| {
                let shift = *shift;
                let index: usize = shift.try_into().unwrap();
                acc[index].increment();
                acc
            },
        );

        shift.into_iter().filter(|x| x.count() != 0).collect_vec()
    }

    /// Returns the spam score count for a set at a weekly cadence. The first value is at the
    /// earliest spam score date in the set and the last value is always the current date even if
    /// it is the fewer than seven days between it and the next-to-last value.
    pub fn weekly_spam_score_counts(&self) -> Vec<DatedSpamScoreCount> {
        let mut date = self.earliest_spam_score_date;
        let end_date = self.latest_spam_score_date;
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
}

impl<'a> TryFrom<UsersSubset<'a>> for SetWithSpamEntries<'a> {
    type Error = EmptySetError;
    fn try_from(value: UsersSubset<'a>) -> Result<Self, Self::Error> {
        let new_subset = value.filtered(|user| {
            user.all_user_values()
                .iter()
                .flatten()
                .flat_map(|user_value| user_value.0.specify_ref::<DatedSpamUpdate>())
                .count()
                > 0
        });

        let earliest_spam_score_date = earliest_spam_score_date(new_subset.iter());
        let latest_spam_score_date = latest_spam_score_date(new_subset.iter());

        if new_subset.user_count() > 0 {
            Ok(Self {
                set: new_subset,
                earliest_spam_score_date,
                latest_spam_score_date,
            })
        } else {
            Err(EmptySetError)
        }
    }
}

pub struct SetWithSpamEntriesIter<'a> {
    iter: HashSetIntoIter<UserWithSpamData<'a>>,
}

impl<'a> Iterator for SetWithSpamEntriesIter<'a> {
    type Item = UserWithSpamData<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        self.iter.next()
    }
}

impl<'a> IntoIterator for SetWithSpamEntries<'a> {
    type Item = UserWithSpamData<'a>;
    type IntoIter = SetWithSpamEntriesIter<'a>;
    fn into_iter(self) -> Self::IntoIter {
        let iter: HashSet<_> = self
            .set
            .into_iter()
            .map(|x| {
                x.try_into()
                    .expect("SetWithSpamEntries should only contain users that have spam data")
            })
            .collect();
        let iter = iter.into_iter();

        SetWithSpamEntriesIter { iter }
    }
}

#[allow(refining_impl_trait)]
impl<'a> UserSet<'a> for SetWithSpamEntries<'a> {
    fn user(&'a self, fid: usize) -> Option<UserWithSpamData<'a>> {
        self.set.user(fid).and_then(|x| x.try_into().ok())
    }
    fn user_count(&self) -> usize {
        self.set.user_count()
    }
}

#[derive(Debug, Error)]
#[error("empty set is not allowed")]
pub struct EmptySetError;

fn earliest_spam_score_date<'a, I>(iterator: I) -> NaiveDate
where
    I: Iterator<Item = &'a User>,
{
    iterator
        .flat_map(|user| user.all_user_values()) // flatten to remove users with no user_values
        .flatten() // second flatten iterates over AnyUserValue
        .flat_map(|x| x.0.specify_ref::<DatedSpamUpdate>())
        .map(|x| x.date())
        .min()
        .expect("internal error - SetWithSpamEntry should always have earliest spam score date")
}

fn latest_spam_score_date<'a, I>(iterator: I) -> NaiveDate
where
    I: Iterator<Item = &'a User>,
{
    iterator
        .flat_map(|user| user.all_user_values()) // flatten to remove users with no user_values
        .flatten() // second flatten iterates over AnyUserValue
        .flat_map(|x| x.0.specify_ref::<DatedSpamUpdate>())
        .map(|x| x.date())
        .max()
        .expect("internal error - SetWithSpamEntry should always have earliest spam score date")
}

fn user_spam_updates(user: &User) -> Vec<&DatedSpamUpdate> {
    user.all_user_values()
        .as_ref()
        .expect("user should have at least one user value")
        .iter()
        .flat_map(|x| x.0.specify_ref::<DatedSpamUpdate>())
        .collect()
}

fn user_earliest_spam_score_date(user: &User) -> NaiveDate {
    let user_iter = [user].into_iter();
    earliest_spam_score_date(user_iter)
}

#[allow(unused)]
fn user_latest_spam_score_date(user: &User) -> NaiveDate {
    let user_iter = [user].into_iter();
    latest_spam_score_date(user_iter)
}

fn spam_score_at_date(updates: &Vec<&DatedSpamUpdate>, date: NaiveDate) -> Option<SpamScore> {
    updates
        .iter()
        .filter(|x| x.date() <= date)
        .max_by_key(|x| x.date())
        .map(|update| update.score())
}

#[cfg(test)]
mod tests {

    use crate::fid_score_shift::ShiftTarget;
    use crate::spam_score::DatedSpamUpdate;
    use crate::user_collection::tests::basic_single_user_test_collection_with_n_spam_updates;
    use crate::user_with_spam_data::tests::create_user_with_m_spam_scores;
    use crate::SpamScore;
    use std::collections::HashSet;

    use super::*;

    fn create_set(collection: &UserCollection) -> Option<SetWithSpamEntries> {
        SetWithSpamEntries::new(collection)
    }

    // str should be of format "YYYY-MM-DD"
    fn check_earliest_date(set: &SetWithSpamEntries, expected: &str) {
        let date = NaiveDate::parse_from_str(expected, "%Y-%m-%d").unwrap();
        assert_eq!(earliest_spam_score_date(set.set.iter()), date);
    }

    fn check_latest_date(set: &SetWithSpamEntries, expected: &str) {
        let date = NaiveDate::parse_from_str(expected, "%Y-%m-%d").unwrap();
        assert_eq!(latest_spam_score_date(set.set.iter()), date);
    }

    fn check_spam_score_count_at_date(set: &SetWithSpamEntries, expected: [u64; 3], date: &str) {
        let date = NaiveDate::parse_from_str(date, "%Y-%m-%d").unwrap();
        assert_eq!(
            set.spam_score_count_at_date(date).unwrap().spam(),
            expected[0]
        );

        assert_eq!(
            set.spam_score_count_at_date(date).unwrap().maybe_spam(),
            expected[1]
        );

        assert_eq!(
            set.spam_score_count_at_date(date).unwrap().non_spam(),
            expected[2]
        );
    }

    // an iterator that returns DatedSpamUpdate with SpamScore One.
    // It begins with a user-defined date an increments from there.
    // It ends after user-define len.
    struct SpamScoreOneIter {
        pub current_date: NaiveDate,
        pub len: u64,
        pub count: u64,
    }

    impl Iterator for SpamScoreOneIter {
        type Item = DatedSpamUpdate;

        fn next(&mut self) -> Option<Self::Item> {
            if self.count < self.len {
                let value = Some(DatedSpamUpdate::from(self.current_date, SpamScore::One));
                self.count += 1;
                self.current_date.checked_add_days(Days::new(1)).unwrap();
                value
            } else {
                None
            }
        }
    }

    fn create_users_with_spam_label_one(n: usize) -> UserCollection {
        let start_date = NaiveDate::from_ymd_opt(2020, 1, 1).unwrap();
        let new_iter = SpamScoreOneIter {
            current_date: start_date,
            len: n as u64,
            count: 0,
        };

        crate::user_collection::tests::new_collection_from_user_value_iter(new_iter)
    }

    // an iterator that returns DatedSpamUpdate cycling through them: 0,1,2,0,1,2,0...
    // It begins with a user-defined date an increments from there.
    // It ends after user-define len.
    struct SpamScoreCyclingIter {
        pub current_date: NaiveDate,
        pub len: u64,
        pub count: u64,
        pub spam_score: u64,
    }

    impl Iterator for SpamScoreCyclingIter {
        type Item = DatedSpamUpdate;

        fn next(&mut self) -> Option<Self::Item> {
            if self.count < self.len {
                let value = Some(DatedSpamUpdate::from(
                    self.current_date,
                    SpamScore::try_from(self.spam_score as usize).unwrap(),
                ));
                self.count += 1;
                self.current_date.checked_add_days(Days::new(1)).unwrap();
                self.spam_score = (self.spam_score + 1) % 3;
                value
            } else {
                None
            }
        }
    }

    fn create_users_with_cycling_spam_labels(n: usize) -> UserCollection {
        let start_date = NaiveDate::from_ymd_opt(2020, 1, 1).unwrap();
        let new_iter = SpamScoreCyclingIter {
            current_date: start_date,
            len: n as u64,
            count: 0,
            spam_score: 0,
        };

        crate::user_collection::tests::new_collection_from_user_value_iter(new_iter)
    }

    mod test_new {
        use super::*;
        use crate::user_collection::tests::*;

        #[test]
        fn empty_test_set() {
            let user_collection = empty_collection();
            let set = create_set(&user_collection);
            assert!(set.is_none());
        }

        #[test]
        fn ones() {
            let collection = create_users_with_spam_label_one(10);
            let set = create_set(&collection);
            assert!(set.is_some());
        }

        #[test]
        fn valid_set() {
            let collection = basic_m_user_test_collection_with_n_spam_updates(1, 1);
            let set = create_set(&collection);
            assert!(set.is_some())
        }
    }

    mod filter {
        use crate::user_collection::tests::dummy_data;
        use crate::user_with_spam_data::tests::earliest_spam_update::earliest_spam_date_before_date_filter;

        use super::*;
        pub enum FilterValidity {
            NonEmpty,
            Empty,
        }

        pub fn check_filter(
            set: &mut SetWithSpamEntries,
            filter: impl Fn(&UserWithSpamData) -> bool,
            valid: FilterValidity,
        ) {
            match valid {
                FilterValidity::Empty => assert!(set.filter(filter).is_none()),
                FilterValidity::NonEmpty => assert!(set.filter(filter).is_some()),
            }
        }

        #[test]
        fn dummy_data_with_emptying_date_filter() {
            let dummy_data = dummy_data();
            let mut set = create_set(&dummy_data).unwrap();
            let date = NaiveDate::default();

            let filter =
                |user: &UserWithSpamData| earliest_spam_date_before_date_filter(user, date);

            check_filter(&mut set, filter, FilterValidity::Empty);
        }

        #[test]
        fn dummy_data_with_nonemptying_date_filter() {
            let dummy_data = dummy_data();
            let mut set = create_set(&dummy_data).unwrap();
            dbg!(&set);

            let date = NaiveDate::parse_from_str("2025-01-01", "%Y-%m-%d").unwrap();

            let filter =
                |user: &UserWithSpamData| earliest_spam_date_before_date_filter(user, date);

            check_filter(&mut set, filter, FilterValidity::NonEmpty);
        }
    }

    mod filtered {
        use super::*;
        enum FilterValidity {
            NonEmpty,
            Empty,
        }

        fn check_filtered(
            set: SetWithSpamEntries,
            filter: fn(&UserWithSpamData) -> bool,
            valid: FilterValidity,
        ) {
            match valid {
                FilterValidity::Empty => {
                    assert!(set.filtered(filter).is_none())
                }
                FilterValidity::NonEmpty => {
                    assert!(set.filtered(filter).is_some())
                }
            }
        }

        fn always_false(_: &UserWithSpamData) -> bool {
            false
        }

        fn always_true(_: &UserWithSpamData) -> bool {
            true
        }

        #[test]
        fn always_off_filtered_test() {
            let collection = basic_single_user_test_collection_with_n_spam_updates(1);
            let set = create_set(&collection).unwrap();
            check_filtered(set, always_false, FilterValidity::Empty);
        }

        #[test]
        fn always_on_filtered_test() {
            let collection = basic_single_user_test_collection_with_n_spam_updates(1);
            let set = create_set(&collection).unwrap();
            check_filtered(set, always_true, FilterValidity::NonEmpty);
        }
    }

    mod current_spam_counts {
        use super::*;

        #[track_caller]
        fn check_count(set: &SetWithSpamEntries, nonspam: u64, maybe: u64, spam: u64) {
            let counts = set.current_spam_score_count();
            assert_eq!(counts.non_spam(), nonspam);
            assert_eq!(counts.maybe_spam(), maybe);
            assert_eq!(counts.spam(), spam);
        }

        #[test]
        fn one_user_per_count() {
            let collection = create_users_with_cycling_spam_labels(3);
            let set = create_set(&collection).unwrap();
            check_count(&set, 1, 1, 1);
        }
    }

    mod current_spam_distributions {
        use super::*;

        fn check_distribution(set: &SetWithSpamEntries, nonspam: f32, maybe: f32, spam: f32) {
            let distribution = set.current_spam_score_distribution();
            assert_eq!(distribution.non_spam(), nonspam);
            assert_eq!(distribution.maybe_spam(), maybe);
            assert_eq!(distribution.spam(), spam);
        }

        #[test]
        fn test_ones() {
            let collection = create_users_with_spam_label_one(10);
            let set = create_set(&collection).unwrap();
            check_distribution(&set, 0.0, 1.0, 0.0);
        }
    }

    mod update_count {
        use super::*;

        fn check_update_count(set: &SetWithSpamEntries, expected: u64) {
            assert_eq!(set.user_count(), expected as usize);
        }

        fn test_ones(n: u64) {
            let collection = create_users_with_spam_label_one(n as usize);
            let set = create_set(&collection).unwrap();
            check_update_count(&set, n);
        }

        fn test_cycling(n: u64) {
            let collection = create_users_with_cycling_spam_labels(n as usize);
            let set = create_set(&collection).unwrap();
            check_update_count(&set, n);
        }

        #[test]
        fn test_ones_with_different_lens() {
            test_ones(1);
            test_ones(10);
            test_ones(100);
            test_ones(1000);
        }

        #[test]
        fn test_cycling_with_different_lens() {
            test_cycling(19);
            test_cycling(57);
            test_cycling(10000);
        }
    }

    mod earliest_date {
        use super::*;
        use crate::user_collection::tests::dummy_data;

        #[test]
        fn test_earliest_date_on_dummy_data() {
            let users = dummy_data();
            let set = create_set(&users).unwrap();
            let date = NaiveDate::from_ymd_opt(2024, 1, 1).unwrap();
            assert_eq!(set.earliest_spam_score_date, date);
        }
    }

    // check that the fid score shifts are equal to the expected values.
    // The format of each array is [from_score, to_score, count]
    fn check_fid_score_shifts(
        set: &SetWithSpamEntries,
        expected: Vec<[u8; 3]>,
        initial_date: &str,
        days: u8,
    ) {
        let mut expected_shifts: HashSet<FidScoreShift> = HashSet::new();
        for [source, target, count] in expected {
            let source = ShiftSource::try_from(source).unwrap();
            let target = ShiftTarget::try_from(target).unwrap();
            let shift = FidScoreShift::new(source, target, count as usize);
            expected_shifts.insert(shift);
        }

        let initial_date = NaiveDate::parse_from_str(initial_date, "%Y-%m-%d").unwrap();
        let days = Days::new(days as u64);

        let actual_shifts =
            HashSet::from_iter(set.spam_changes_with_fid_score_shift(initial_date, days));

        assert_eq!(expected_shifts, actual_shifts);
    }

    fn basic_test_set() -> UserCollection {
        let value =
            DatedSpamUpdate::from(NaiveDate::from_ymd_opt(2024, 1, 1).unwrap(), SpamScore::One);
        let mut user = User::new_without_labels(1);
        user.add_user_value(value).unwrap();
        let mut collection = UserCollection::default();
        collection.add_user(user).unwrap();
        collection
    }

    // one user created per day with one spam score per day m times. Starting at zero then rotating
    // through zero, one, two...
    fn basic_m_user_test_collection_with_n_spam_updates(m: u64, n: u64) -> UserCollection {
        let mut collection = UserCollection::default();
        let mut date = NaiveDate::parse_from_str("2020-1-1", "%Y-%m-%d").unwrap();
        for i in 0..m {
            let user = create_user_with_m_spam_scores(i, n, date);
            collection.add_user(user).expect("add cannot collide");
            date = date.checked_add_days(Days::new(1)).unwrap();
        }
        collection
    }

    #[test]
    fn test_earliest_and_latest_on_single_user_set() {
        let collection = basic_single_user_test_collection_with_n_spam_updates(3);
        let set = create_set(&collection).unwrap();
        check_earliest_date(&set, "2020-1-1");
        check_latest_date(&set, "2020-1-3");
    }

    #[test]
    fn test_count_on_single_user_set() {
        let collection = basic_single_user_test_collection_with_n_spam_updates(3);
        let set = create_set(&collection).unwrap();
        check_spam_score_count_at_date(&set, [1, 0, 0], "2020-1-1");
    }

    #[test]
    fn test_earliest_and_latest_spam_score_date() {
        let collection = basic_test_set();
        let set = create_set(&collection).unwrap();
        check_earliest_date(&set, "2024-01-01");
        check_latest_date(&set, "2024-01-01");
    }

    #[test]
    fn test_basic_fid_score_shift_with_single_user() {
        let collection = basic_single_user_test_collection_with_n_spam_updates(2);
        let set = create_set(&collection).unwrap();
        check_fid_score_shifts(&set, vec![[0, 1, 1]], "2020-1-1", 1);
    }

    #[test]
    fn test_fid_score_shifts_with_multiple_users() {
        let collection = basic_m_user_test_collection_with_n_spam_updates(3, 3);
        let set = create_set(&collection).unwrap();
        let expected = vec![[3, 2, 3]];
        check_fid_score_shifts(&set, expected, "2019-12-19", 100);
    }

    #[test]
    fn test_fid_score_shifts_with_dates_before_all_spam_scores() {
        let collection = basic_single_user_test_collection_with_n_spam_updates(1);
        let set = create_set(&collection).unwrap();
        let expected = Vec::new();
        check_fid_score_shifts(&set, expected, "2014-1-1", 1);
    }

    #[test]
    fn second_test_fid_score_shifts_with_multiple_users() {
        let collection = basic_m_user_test_collection_with_n_spam_updates(6, 3);
        let set = create_set(&collection).unwrap();
        let expected = vec![[0, 2, 1], [3, 2, 5]];
        check_fid_score_shifts(&set, expected, "2020-1-1", 100);
    }

    #[test]
    pub fn test_spam_score_collision_error_for_invalid_record_add() {
        let date = NaiveDate::from_ymd_opt(2020, 1, 2).unwrap();
        let earlier_date = NaiveDate::from_ymd_opt(2020, 1, 1).unwrap();

        let mut user = User::new_without_labels(1);

        assert!(user
            .try_add_user_value(DatedSpamUpdate::from(date, SpamScore::One))
            .is_ok());
        assert!(user
            .try_add_user_value(DatedSpamUpdate::from(date, SpamScore::Zero))
            .is_err());

        assert!(user
            .try_add_user_value(DatedSpamUpdate::from(earlier_date, SpamScore::Zero))
            .is_ok());

        let spam_updates = user.user_values_of_kind::<DatedSpamUpdate>();

        let earliest_spam_score_date = user_earliest_spam_score_date(&user);
        let latest_spam_score_date = user_latest_spam_score_date(&user);

        let earliest_spam_score =
            spam_score_at_date(&spam_updates, earliest_spam_score_date).unwrap();
        let latest_spam_score = spam_score_at_date(&spam_updates, latest_spam_score_date).unwrap();

        assert_eq!(
            (earliest_spam_score, earliest_spam_score_date),
            (SpamScore::Zero, earlier_date)
        );

        assert_eq!(
            (latest_spam_score, latest_spam_score_date),
            (SpamScore::One, date)
        );
    }
}
