#![allow(unused)]
use crate::cast_meta::CastMeta;
use crate::cast_meta::CastType;
use crate::import::ImporterError;
use chrono::Duration;
use chrono::NaiveDate;
use reqwest::Response;
use serde_json::Value;

/// a collection of functions to parse data from the pinata farcaster api.
pub async fn cast_meta_from_pinata_response(
    response: Response,
) -> Result<Vec<CastMeta>, ImporterError> {
    if !response.status().is_success() {
        return Err(ImporterError::FailedApiRequest);
    };

    let response_text = response
        .text()
        .await
        .map_err(|_| ImporterError::FailedApiRequest)?;

    let json: Value = serde_json::from_str(&response_text)
        .map_err(|_| ImporterError::BadApiResponse(response_text.clone()))?;

    let json_vec = json["messages"].as_array().unwrap();
    json_vec
        .iter()
        .map(|x| {
            date_from_object(x)
                .and_then(|date| type_from_object(x).map(|cast_type| (date, cast_type)))
                .and_then(|(date, cast_type)| fid_from_object(x).map(|fid| (date, cast_type, fid)))
                .map(|x| CastMeta::new(x.0, x.1, x.2))
        })
        .collect::<Result<Vec<CastMeta>, ImporterError>>()
}

fn date_from_object(input: &Value) -> Result<NaiveDate, ImporterError> {
    let farcaster_epoch = NaiveDate::from_ymd_opt(2021, 1, 1).unwrap();
    input["data"]["timestamp"]
        .to_string()
        .parse::<i64>()
        .map_err(|_| ImporterError::BadApiResponse(input.to_string()))
        .map(|x| farcaster_epoch + Duration::seconds(x))
}

fn fid_from_object(input: &Value) -> Result<u64, ImporterError> {
    input["data"]["fid"]
        .as_number()
        .ok_or(ImporterError::FailedApiRequest)
        .and_then(|x| x.as_u64().ok_or(ImporterError::FailedApiRequest))
}

fn type_from_object(input: &Value) -> Result<CastType, ImporterError> {
    let cast_type: CastType = input["data"]["castAddBody"]["type"]
        .as_str()
        .ok_or(ImporterError::BadApiResponse(input.to_string().clone()))?
        .try_into()
        .map_err(|_| ImporterError::BadApiResponse(input.to_string().clone()))?;
    Ok(cast_type)
}

pub async fn number_of_casts_from_response(response: Response) -> Result<u64, ImporterError> {
    Ok(cast_meta_from_pinata_response(response).await?.len() as u64)
}
