use super::ImporterError;
use crate::dated::Dated;
use crate::fidded::Fidded;
use crate::CastType;
use crate::Fid;
use chrono::Duration;
use chrono::NaiveDate;
use chrono::NaiveDateTime;
use chrono::NaiveTime;
use reqwest::Response;
use serde_json::Value;

async fn raw_json_from_response(response: Response) -> Result<Value, ImporterError> {
    if !response.status().is_success() {
        return Err(ImporterError::FailedApiRequest);
    };

    let response_text = response
        .text()
        .await
        .map_err(|_| ImporterError::FailedApiRequest)?;

    let json: Value = serde_json::from_str(&response_text)
        .map_err(|_| ImporterError::BadApiResponse(response_text.clone()))?;

    Ok(json)
}

fn date_from_object(input: &Value) -> Result<NaiveDate, ImporterError> {
    let farcaster_epoch = NaiveDate::from_ymd_opt(2021, 1, 1).unwrap();
    input["data"]["timestamp"]
        .to_string()
        .parse::<i64>()
        .map_err(|_| ImporterError::BadApiResponse(input.to_string()))
        .map(|x| farcaster_epoch + Duration::seconds(x))
}

fn date_time_from_object(input: &Value) -> Result<NaiveDateTime, ImporterError> {
    let farcaster_epoch = NaiveDateTime::new(
        NaiveDate::from_ymd_opt(2021, 1, 1).unwrap(),
        NaiveTime::from_hms_opt(0, 0, 0).unwrap(),
    );

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

pub async fn reaction_times_from_response(
    response: Response,
) -> Result<Vec<NaiveDateTime>, ImporterError> {
    let json = raw_json_from_response(response).await?;
    let json_vec = json["messages"].as_array().unwrap();
    json_vec.iter().map(date_time_from_object).collect()
}

pub async fn cast_meta_from_pinata_response(
    response: Response,
) -> Result<Vec<Fidded<Dated<CastType>>>, ImporterError> {
    let json = raw_json_from_response(response).await?;
    let json_vec = json["messages"].as_array().unwrap();
    json_vec
        .iter()
        .map(|x| {
            date_from_object(x)
                .and_then(|date| type_from_object(x).map(|cast_type| (date, cast_type)))
                .and_then(|(date, cast_type)| {
                    fid_from_object(x).map(|fid| (date, cast_type, fid as usize))
                })
                .map(|x| {
                    let dated_cast_type = Dated::<CastType>::from(x.0, x.1);
                    let fid = Fid::from(x.2);
                    let result: Fidded<Dated<CastType>> = (dated_cast_type, fid).into();
                    result
                })
        })
        .collect::<Result<Vec<Fidded<Dated<CastType>>>, ImporterError>>()
}

pub async fn followers_from_pinata_response(response: Response) -> Result<Vec<u64>, ImporterError> {
    let json = raw_json_from_response(response).await?;
    let json_vec = json["messages"]
        .as_array()
        .ok_or(ImporterError::BadApiResponse(json.to_string()))?;

    json_vec
        .iter()
        .map(|x| {
            x["data"]["fid"]
                .as_u64()
                .ok_or(ImporterError::BadApiResponse(x.to_string()))
        })
        .collect::<Result<Vec<u64>, ImporterError>>()
}
