use chrono::{Days, Duration, NaiveDate};
use farmap::fid_score_shift::ShiftSource;
use farmap::fid_score_shift::ShiftTarget;
use farmap::spam_score::SpamScore;
use farmap::subset::UsersSubset;
use farmap::user::{User, UserError};
use farmap::user_collection::UserCollection;
use farmap::FidScoreShift;
use std::collections::HashSet;

/// Create n users by incrementing fid and incrementing one day from 20200101, all with spam label
/// one.
fn create_users_with_spam_label_one(n: usize) -> Result<UserCollection, UserError> {
    let start_date = NaiveDate::from_ymd_opt(2020, 1, 1).unwrap();
    let mut users = UserCollection::default();
    let mut date = start_date;
    for i in 0..n {
        let user = User::new(i, (SpamScore::One, date));
        users.push_with_res(user)?;
        date = date.checked_add_signed(Duration::days(1)).unwrap();
    }

    Ok(users)
}

fn every_other_user_has_spam_label_one_and_two(n: usize) -> Result<UserCollection, UserError> {
    let start_date = NaiveDate::from_ymd_opt(2020, 1, 1).unwrap();
    let mut users = UserCollection::default();
    let mut date = start_date;

    for i in 0..n {
        date = date.checked_add_signed(Duration::days(1)).unwrap();
        if i % 2 == 0 {
            let user = User::new(i, (SpamScore::One, date));
            users.push_with_res(user)?;
        } else {
            let user = User::new(i, (SpamScore::Two, date));
            users.push_with_res(user)?;
        }
    }

    Ok(users)
}

/// Create n users by increminting fid and incrementing one day from 20200101, all with spam label
/// one.
fn create_users_with_spam_label_one_then_two(n: usize) -> Result<UserCollection, UserError> {
    let start_date = NaiveDate::from_ymd_opt(2020, 1, 1).unwrap();
    let mut users = UserCollection::default();
    let mut date = start_date;
    for i in 0..n {
        let user = User::new(i, (SpamScore::One, date));
        users.push_with_res(user)?;
        date = date.checked_add_signed(Duration::days(1)).unwrap();
    }

    // create a spam record with score 2 for incrementing dates.
    for i in 0..n {
        users.push_with_res(User::new(i, (SpamScore::Two, date)))?;
        date = date.checked_add_signed(Duration::days(1)).unwrap();
    }

    Ok(users)
}

#[test]
fn distribution_should_be_ones() {
    let users = create_users_with_spam_label_one(10).unwrap();
    let subset = UsersSubset::from(&users);
    assert_eq!(subset.user_count(), 10);
    assert_eq!(
        subset.current_spam_score_distribution().unwrap(),
        [0.0, 1.0, 0.0]
    );
}

#[test]
fn distribution_should_be_ones_and_twos() {
    let n: u64 = 2;
    let users = every_other_user_has_spam_label_one_and_two(n as usize).unwrap();
    let subset = UsersSubset::from(&users);
    assert_eq!(subset.user_count(), n as usize);
    assert_eq!(
        subset.current_spam_score_distribution().unwrap(),
        [0.0, 0.5, 0.5]
    );
}

#[test]
fn distribution_should_be_ones_and_twos_with_n_100() {
    let n: u64 = 100;
    let users = every_other_user_has_spam_label_one_and_two(n as usize).unwrap();
    let subset = UsersSubset::from(&users);
    assert_eq!(subset.user_count(), n as usize);
    assert_eq!(
        subset.current_spam_score_distribution().unwrap(),
        [0.0, 0.5, 0.5]
    );
}

#[test]
fn distribution_should_be_ones_and_twos_with_n_5() {
    let n: u64 = 5;
    let users = every_other_user_has_spam_label_one_and_two(n as usize).unwrap();
    let subset = UsersSubset::from(&users);
    assert_eq!(subset.user_count(), n as usize);
    assert_eq!(
        subset.current_spam_score_distribution().unwrap(),
        [0.0, 0.6, 0.4]
    );
}

#[test]
fn distribution_should_be_twos() {
    let n: u64 = 10;

    let users = create_users_with_spam_label_one_then_two(n as usize).unwrap();
    let subset = UsersSubset::from(&users);

    assert_eq!(subset.user_count(), n as usize);

    assert_eq!(
        subset.current_spam_score_distribution().unwrap(),
        [0.0, 0.0, 1.0]
    );

    // check that spam record for each fid is stored correctly
    for i in 0..10 {
        let spam_record = subset.user(i).unwrap().all_spam_records();
        println!("{:?}", spam_record);
        assert_eq!(
            spam_record[0].1,
            NaiveDate::from_ymd_opt(2020, 1, 1)
                .unwrap()
                .checked_add_days(Days::new(i.try_into().unwrap()))
                .unwrap()
        );

        assert_eq!(spam_record[0].0, SpamScore::One);

        assert_eq!(
            spam_record[1].1,
            NaiveDate::from_ymd_opt(2020, 1, 1)
                .unwrap()
                .checked_add_days(Days::new(n + i as u64))
                .unwrap()
        );

        assert_eq!(spam_record[1].0, SpamScore::Two);
    }
}

#[test]
fn fid_shift_test_for_one_then_two_data() {
    let n: u64 = 10;

    let users = create_users_with_spam_label_one_then_two(n as usize).unwrap();
    let subset = UsersSubset::from(&users);
    let shifts = subset.spam_changes_with_fid_score_shift(
        NaiveDate::from_ymd_opt(2020, 1, 1)
            .unwrap()
            .checked_add_days(Days::new(n - 1))
            .unwrap(),
        Days::new(0),
    );

    let expected_shift = FidScoreShift::new(ShiftSource::One, ShiftTarget::One, n as usize);
    assert_eq!(shifts[0], expected_shift);
    assert_eq!(shifts.len(), 1);

    let shifts = subset.spam_changes_with_fid_score_shift(
        NaiveDate::from_ymd_opt(2020, 1, 1)
            .unwrap()
            .checked_add_days(Days::new(n - 1))
            .unwrap(),
        Days::new(n),
    );
    let expected_shift = FidScoreShift::new(ShiftSource::One, ShiftTarget::Two, n as usize);
    assert_eq!(expected_shift, shifts[0]);
    //assert_eq!(matrix, [[0, 0, 0], [0, 0, n as usize], [0, 0, 0]]);
}

#[test]
fn shift_struct_should_be_n_in_one_to_one() {
    let n = 10;
    let users = create_users_with_spam_label_one(n).unwrap();
    let subset = UsersSubset::from(&users);
    assert_eq!(
        subset.spam_changes_with_fid_score_shift(
            NaiveDate::from_ymd_opt(2020, 12, 5).unwrap(),
            Days::new(1)
        )[0],
        FidScoreShift::new(ShiftSource::One, ShiftTarget::One, n)
    );
    assert_eq!(
        subset
            .spam_changes_with_fid_score_shift(
                NaiveDate::from_ymd_opt(2020, 12, 5).unwrap(),
                Days::new(1)
            )
            .len(),
        1
    )
}

#[test]
fn test_weekly_spam_score_count() {
    let n = 365;
    let users = create_users_with_spam_label_one(n).unwrap();
    let set = UsersSubset::from(&users);

    let results = set.weekly_spam_score_counts();

    // check that all dates are unique
    let mut uniques = HashSet::<NaiveDate>::new();
    for r in &results {
        uniques.insert(r.date());
    }
    assert_eq!(uniques.len(), results.len());

    //check first date
    assert_eq!(
        results[0].date(),
        NaiveDate::from_ymd_opt(2020, 1, 1).unwrap()
    );

    let mut previous_date: Option<NaiveDate> = None;
    //check that the time between each date expect last is seven days.
    for (i, r) in results.iter().enumerate() {
        if previous_date.is_none() {
            previous_date = Some(r.date());
            continue;
        } else if i == results.len() - 1 {
            assert!(
                r.date()
                    <= previous_date
                        .unwrap()
                        .checked_add_days(Days::new(7))
                        .unwrap(),
            );

            break;
        };

        assert_eq!(
            r.date(),
            previous_date
                .unwrap()
                .checked_add_days(Days::new(7))
                .unwrap()
        );
        previous_date = Some(r.date());
    }
}

#[test]
pub fn update_count_should_be_one_per_day() {
    let n = 365;
    let users = create_users_with_spam_label_one(n).unwrap();
    let set = UsersSubset::from(&users);
    let sum: usize = set.count_updates().values().sum();
    assert_eq!(sum, n);
    assert!(set.count_updates().values().all(|n| *n == 1));
}
