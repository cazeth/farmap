use super::ImporterError;
use serde_json::Value;

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
