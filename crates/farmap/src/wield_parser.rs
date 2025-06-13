use crate::import::ImporterError;
use crate::importer_utils::parse_json_from_response;
use log::trace;
use reqwest::Response;
use serde_json::Value;

pub async fn parse_follow_response(response: Response) -> Result<Vec<u64>, ImporterError> {
    let json = parse_json_from_response(response).await?;
    trace!("succesfully parsed into raw json {:?}", json);
    parse_raw_json(json)
}

fn parse_raw_json(json: Value) -> Result<Vec<u64>, ImporterError> {
    json.as_object()
        .ok_or(ImporterError::BadApiResponse(json.to_string()))?
        .get("result")
        .ok_or(ImporterError::BadApiResponse(json.to_string()))?
        .as_object()
        .ok_or(ImporterError::BadApiResponse(json.to_string()))?
        .get("users")
        .ok_or(ImporterError::BadApiResponse(json.to_string()))?
        .as_array()
        .ok_or(ImporterError::BadApiResponse(json.to_string()))?
        .iter()
        .map(|object| {
            object
                .as_object()
                .ok_or(ImporterError::BadApiResponse(json.to_string()))
                .and_then(|object| {
                    object
                        .get("fid")
                        .ok_or(ImporterError::BadApiResponse(json.to_string()))
                })
                .and_then(|fid_str| {
                    fid_str
                        .as_str()
                        .ok_or(ImporterError::BadApiResponse(json.to_string()))
                })
                .and_then(|fid_str| {
                    fid_str
                        .parse::<u64>()
                        .map_err(|_| ImporterError::BadApiResponse(json.to_string()))
                })
        })
        .collect::<Result<_, _>>()
}

#[cfg(test)]
pub mod test {
    use super::*;

    #[test]
    fn parse_example_response() {
        let example = r#"{
  "result": {
    "users": [
      {
        "fid": "111",
        "followingCount": 123,
        "followerCount": 123,
        "pfp": {
          "url": "test.com",
          "verified": true
        },
        "bio": {
          "text": "a test",
          "mentions": [
            "<string>"
          ]
        },
        "external": true,
        "custodyAddress": "....",
        "username": "....",
        "displayName": "....",
        "registeredAt": "2023-11-07T05:31:56Z"
      }
    ]
  },
  "next": "<string>",
  "source": "v2"
}"#;
        let json: Value = serde_json::from_str(example).unwrap();
        assert_eq!(*parse_raw_json(json).unwrap().first().unwrap(), 111);
    }
}
