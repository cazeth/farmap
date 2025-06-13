use crate::import::ImporterError;
use crate::wield_parser;
use log::{trace, warn};
use reqwest::Response;
use std::str::FromStr;
use thiserror::Error;

use reqwest::header;
use reqwest::Client;
use reqwest::ClientBuilder;
use url::Url;

/// Fetches data from the [wield](https://wield.xyz) api.
/// The fetcher likely won't work without an api key so make sure to follow their docs to get an
/// api_key and set set_api_key_from_env_var method to set your api key before trying to use this
/// struct to make calls.
/// You also need to build before using.
pub struct WieldFetcher {
    client: Client,
    client_builder: Option<ClientBuilder>,
    base_url: Url,
    api_key: String,
}

impl Default for WieldFetcher {
    fn default() -> Self {
        Self {
            client: Client::default(),
            client_builder: None,
            base_url: Url::from_str("https://build.wield.xyz/farcaster/v2/").unwrap(),
            api_key: String::new(),
        }
    }
}

impl WieldFetcher {
    pub fn set_api_key_from_env_var(mut self, api_key_var: &str) -> Result<Self, ApiKeyError> {
        let api_key = std::env::var(api_key_var).inspect_err(|_| {
            warn!("could not set api key for wield fetcher");
        })?;
        self.api_key = api_key;
        trace!("found api key for wield fetcher");

        let mut headers = header::HeaderMap::new();
        let mut auth_value = header::HeaderValue::from_str(&self.api_key)?;
        auth_value.set_sensitive(true);
        headers.insert("API-KEY", auth_value);

        self.client_builder = Some(Client::builder().default_headers(headers));

        Ok(self)
    }

    pub fn build(mut self) -> Result<Self, BuildError> {
        let client_builder = std::mem::take(&mut self.client_builder);
        if let Some(client_builder) = client_builder {
            self.client = client_builder.build()?;
            Ok(self)
        } else {
            Err(BuildError::NoApiKeyError)
        }
    }

    pub async fn fetch_followers(&self, fid: u64) -> Result<Vec<u64>, ImporterError> {
        trace!("trying to fetch followers...");
        let response = self
            .fetch_follower_response_for_fid(fid)
            .await
            .inspect_err(|e| trace!("fetch failed with error {e:?}"))?;
        trace!("fetching followers, response is {:?}", response);
        let followers = wield_parser::parse_follow_response(response).await?;
        Ok(followers)
    }

    pub async fn fetch_follower_response_for_fid(
        &self,
        fid: u64,
    ) -> Result<Response, ImporterError> {
        let extension = "followers";
        let mut url = self.base_url.clone().join(extension).unwrap();
        url.set_query(Some(&format!("fid={fid}")));
        trace!("calling followers for fid {fid}, with url {url}");
        self.client
            .get(url)
            .send()
            .await
            .map_err(|_| ImporterError::FailedApiRequest)
    }
}

#[derive(Error, Debug)]
pub enum ApiKeyError {
    #[error("could not read api key")]
    VarError(#[from] std::env::VarError),

    #[error("invalid api key found, could not use as header")]
    InvalidHeaderError(#[from] reqwest::header::InvalidHeaderValue),
}

#[derive(Error, Debug)]
pub enum BuildError {
    #[error("could not read api key")]
    BuildError(#[from] reqwest::Error),

    #[error("need an api key to build")]
    NoApiKeyError,
}
