use crate::spam_score::DatedSpamUpdate;
use crate::Fidded;
use crate::UnprocessedUserLine;

use super::ImporterError;
use itertools::Itertools;
use serde_json::Value;
use serde_jsonlines::JsonLinesReader;

pub fn parse_status(input: &str) -> Result<Vec<String>, ImporterError> {
    let json_value: Value = serde_json::from_str(input)
        .map_err(|_| ImporterError::BadApiResponse(input.to_string()))?;

    let array = json_value
        .as_array()
        .ok_or(ImporterError::BadApiResponse(input.to_string()))?;

    array
        .iter()
        .map(|x| {
            x.as_object()
                .ok_or(ImporterError::BadApiResponse(input.to_string()))
                .and_then(|x| {
                    x.get("sha")
                        .ok_or(ImporterError::BadApiResponse(input.to_string()))
                })
                .map(|x| x.to_string().replace("\"", ""))
        })
        .collect::<Result<Vec<String>, ImporterError>>()
}

/// read all the lines of a body. If any particular line cannot be processed it is collected into the
/// Vec<ImporterError>. All the valid lines are still collected into the Vec<UnprocessedUserLine>.
pub fn parse_commit_hash_body(body: &str) -> (Vec<UnprocessedUserLine>, Vec<ImporterError>) {
    JsonLinesReader::new(body.as_bytes())
        .read_all::<UnprocessedUserLine>()
        .map(|x| x.map_err(|res| ImporterError::BadApiResponse(format!("{res:?}"))))
        .partition_result()
}

pub fn into_fidded_user_value_iter(
    previous_iter: impl IntoIterator<Item = UnprocessedUserLine>,
) -> impl Iterator<Item = Fidded<DatedSpamUpdate>> {
    previous_iter
        .into_iter()
        .map(|x| Fidded::<DatedSpamUpdate>::try_from(x).unwrap())
}

#[cfg(test)]
pub mod tests {
    use super::*;
    use std::fs::read_to_string;

    pub fn fidded_user_values_from_raw_spam_data_file(
    ) -> impl Iterator<Item = Fidded<DatedSpamUpdate>> {
        let file_path = "data/dummy-data/spam.jsonl";
        let body = read_to_string(file_path).unwrap();
        let raw_iter = parse_commit_hash_body(&body).0;
        into_fidded_user_value_iter(raw_iter)
    }

    #[test]
    fn test_on_spam_data() {
        let spam_data = fidded_user_values_from_raw_spam_data_file();
        assert_eq!(3, spam_data.count())
    }
}
