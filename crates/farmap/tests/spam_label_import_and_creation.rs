//!Some tests for local_spam_label_importer and user_collection
use farmap::fetch::local_spam_label_importer;
use farmap::spam_score::DatedSpamUpdate;
use farmap::Fidded;
use farmap::User;
use farmap::UserCollection;

#[test]
pub fn test_spam_score_collision_with_error_collect() {
    let user_lines = local_spam_label_importer::import_data_from_file_with_collected_res(
        "data/invalid-data/collision_data.jsonl",
    )
    .unwrap();
    let updates =
        local_spam_label_importer::import_data_from_file("data/invalid-data/collision_data.jsonl")
            .unwrap();

    let mut user_collection = UserCollection::default();
    let first_line = updates[0];

    let mut user = User::new(first_line.fid());

    user.add_user_value(first_line.unfid())
        .expect("should not collide");

    let first_line_result = user_collection.add_user(user);
    let second_line = user_lines[1].as_ref().unwrap();
    let mut user = User::new(second_line.fid());
    user.add_user_value(first_line.unfid()).unwrap();
    let second_line_result = user_collection.add_user(user);

    assert!(first_line_result.is_ok());
    assert!(second_line_result.is_err());
}

#[test]
pub fn test_error_on_invalid_fid() {
    let mut userlines = local_spam_label_importer::import_data_from_file_with_res(
        "data/invalid-data/invalid_spamscore.jsonl",
    )
    .unwrap();

    let userline = userlines.pop().unwrap();
    let new_entry: Result<Fidded<DatedSpamUpdate>, _> = userline.try_into();
    assert!(new_entry.is_err());
}
