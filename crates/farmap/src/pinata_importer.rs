use crate::import::ImporterError;
use reqwest::{Client, Response};
use url::Url;

pub struct PinataFetcher {
    client: Client,
    base_url: Url,
}

impl Default for PinataFetcher {
    fn default() -> Self {
        Self {
            base_url: Url::parse("https://hub.pinata.cloud/v1/castsByFid").unwrap(),
            client: Client::new(),
        }
    }
}

impl PinataFetcher {
    pub fn with_base_url(self, url: Url) -> Self {
        Self {
            base_url: url,
            ..self
        }
    }

    pub async fn api_request_for_id(&self, id: u64) -> Result<Response, ImporterError> {
        self.client
            .get(
                Url::parse_with_params(self.base_url.as_str(), &[("fid", id.to_string())])
                    .map_err(|_| ImporterError::InvalidEndpoint)?,
            )
            .send()
            .await
            .map_err(|_| ImporterError::FailedApiRequest)
    }

    pub async fn number_of_casts_from_response(
        &self,
        response: Response,
    ) -> Result<u64, ImporterError> {
        if !response.status().is_success() {
            return Err(ImporterError::FailedApiRequest);
        };
        let response_text = response
            .text()
            .await
            .map_err(|_| ImporterError::FailedApiRequest)?;

        let json: serde_json::Value = serde_json::from_str(&response_text)
            .map_err(|_| ImporterError::BadApiResponse(response_text.clone()))?;

        let number_of_casts = json["messages"]
            .as_array()
            .ok_or(ImporterError::BadApiResponse(response_text.clone()))?
            .len();

        Ok(number_of_casts as u64)
    }
}
