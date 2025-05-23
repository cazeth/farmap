use std::{
    collections::HashMap,
    path::{Path, PathBuf},
};

use farmap::{UserCollection, UsersSubset};

/// number_of_casts maps fid -> cast_number in data
fn check_total_cast(input_file: &Path, expected_number_of_casts: HashMap<u64, u64>) {
    let collection = UserCollection::create_from_db(input_file).unwrap();
    let set = UsersSubset::from(&collection);
    expected_number_of_casts
        .iter()
        .for_each(|(fid, v)| assert_eq!(set.user(*fid as usize).unwrap().cast_count().unwrap(), *v))
}

/// number_of_casts maps fid -> cast_number in data
fn check_monthly_average_casts(input_file: &Path, expected_cast_average: HashMap<u64, f32>) {
    let collection = UserCollection::create_from_db(input_file).unwrap();
    let float_equality_accuracy = 0.1;
    let set = UsersSubset::from(&collection);

    expected_cast_average.iter().for_each(|(fid, v)| {
        let actual_cast_average = set
            .user(*fid as usize)
            .unwrap()
            .average_monthly_cast_rate()
            .unwrap();

        assert!(
            actual_cast_average <= *v + float_equality_accuracy / 2.0
                && actual_cast_average >= *v - float_equality_accuracy / 2.0
        );
    })
}

#[test]
fn total_casts_from_single_object() {
    let path = PathBuf::from("./test-data/has_cast_data_1.json");
    let map = HashMap::from([(11720, 19)]);
    check_total_cast(&path, map);
    let map = HashMap::from([(11720, 6.3333)]);
    check_monthly_average_casts(&path, map);
}

#[test]
fn total_casts_from_two_objects() {
    let path = PathBuf::from("./test-data/has_cast_data_2.json");
    let map = HashMap::from([(11720, 19), (1, 20)]);
    check_total_cast(&path, map);
    let map = HashMap::from([(11720, 6.3333), (1, 20.0 / 3.0)]);
    check_monthly_average_casts(&path, map);
}
