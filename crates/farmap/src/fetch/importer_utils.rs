use super::ImporterError;
use log::trace;
use reqwest::Response;
use serde_json::Value;

pub async fn parse_json_from_response(response: Response) -> Result<Value, ImporterError> {
    if !response.status().is_success() {
        return Err(ImporterError::FailedApiRequest);
    };

    let response_text = response
        .text()
        .await
        .map_err(|_| ImporterError::FailedApiRequest)?;

    trace!("response text: {:?}", response_text);
    serde_json::from_str(&response_text)
        .map_err(|_| ImporterError::BadApiResponse(response_text.to_string()))
}
