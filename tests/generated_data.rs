use chrono::{Days, Duration, NaiveDate};
use farmap::spam_score::SpamScore;
use farmap::subset::UsersSubset;
use farmap::user::{User, Users};

/// Create n users by increminting fid and incrementing one day from 20200101, all with spam label
/// one.
fn create_users_with_spam_label_one(n: usize) -> Users {
    let start_date = NaiveDate::from_ymd_opt(2020, 1, 1).unwrap();
    let mut users = Users::default();
    let mut date = start_date;
    for i in 0..n {
        date = date.checked_add_signed(Duration::days(1)).unwrap();
        let user = User::new(i, (SpamScore::One, date));
        users.push(user);
    }

    users
}

/// Create n users by increminting fid and incrementing one day from 20200101, all with spam label
/// one.
fn create_users_with_spam_label_one_then_two(n: usize) -> Users {
    let start_date = NaiveDate::from_ymd_opt(2020, 1, 1).unwrap();
    let mut users = Users::default();
    let mut date = start_date;
    for i in 0..n {
        let user = User::new(i, (SpamScore::One, date));
        users.push(user);
        date = date.checked_add_signed(Duration::days(1)).unwrap();
    }

    // create a spam record with score 2 for incrementing dates.
    for i in 0..n {
        users.push(User::new(i, (SpamScore::Two, date)));
        date = date.checked_add_signed(Duration::days(1)).unwrap();
    }

    users
}

#[test]
fn distribution_should_be_ones() {
    let users = create_users_with_spam_label_one(10);
    let subset = UsersSubset::from(&users);
    assert_eq!(subset.user_count(), 10);
    assert_eq!(
        subset.current_spam_score_distribution().unwrap(),
        [0.0, 1.0, 0.0]
    );
}

#[test]
fn distribution_should_be_twos() {
    let n: u64 = 10;

    let users = create_users_with_spam_label_one_then_two(n as usize);
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
fn change_matrix_test_for_one_then_two_data() {
    let n: u64 = 10;

    let users = create_users_with_spam_label_one_then_two(n as usize);
    let subset = UsersSubset::from(&users);
    let matrix = subset.spam_change_matrix(
        NaiveDate::from_ymd_opt(2020, 1, 1)
            .unwrap()
            .checked_add_days(Days::new(n - 1))
            .unwrap(),
        Days::new(0),
    );

    assert_eq!(matrix, [[0, 0, 0], [0, n as usize, 0], [0, 0, 0]]);

    let matrix = subset.spam_change_matrix(
        NaiveDate::from_ymd_opt(2020, 1, 1)
            .unwrap()
            .checked_add_days(Days::new(n - 1))
            .unwrap(),
        Days::new(n),
    );
    println!("{:?}", matrix);
    assert_eq!(matrix, [[0, 0, 0], [0, 0, n as usize], [0, 0, 0]]);
}

#[test]
fn change_matrix_should_be_n_in_center() {
    let n = 10;
    let users = create_users_with_spam_label_one(n);
    let subset = UsersSubset::from(&users);
    assert_eq!(
        subset.spam_change_matrix(NaiveDate::from_ymd_opt(2020, 12, 5).unwrap(), Days::new(1)),
        [[0, 0, 0], [0, n, 0], [0, 0, 0]]
    );
}
