//!Some tests for local_spam_label_importer and user_collection
use farmap::fetch::local_spam_label_importer;
use farmap::UserCollection;
use std::path::PathBuf;

#[test]
pub fn test_user_count_on_file_with_res() {
    let path = PathBuf::from("data/dummy-data/spam.jsonl");
    let unprocessed =
        local_spam_label_importer::import_data_from_file_with_res(path.to_str().unwrap()).unwrap();
    let users = UserCollection::create_from_unprocessed_user_lines_and_collect_non_fatal_errors(
        unprocessed,
    )
    .0;
    assert_eq!(users.user_count(), 2);
}

#[test]
pub fn test_spam_score_collision_with_error_collect() {
    let mut user_lines = local_spam_label_importer::import_data_from_file_with_collected_res(
        "data/invalid-data/collision_data.jsonl",
    )
    .unwrap();

    let mut user_collection = UserCollection::default();

    let first_line_result =
        user_collection.push_unprocessed_user_line(user_lines.pop().unwrap().unwrap());
    let second_line_result =
        user_collection.push_unprocessed_user_line(user_lines.pop().unwrap().unwrap());

    assert!(first_line_result.is_ok());
    assert!(second_line_result.is_err());
}

#[test]
pub fn test_error_on_invalid_fid() {
    let mut userlines = local_spam_label_importer::import_data_from_file_with_res(
        "data/invalid-data/invalid_spamscore.jsonl",
    )
    .unwrap();

    let mut users = UserCollection::default();
    let result = users.push_unprocessed_user_line(userlines.pop().unwrap());
    assert!(result.is_err());
}
