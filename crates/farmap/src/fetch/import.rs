use super::github_parser;
use crate::UnprocessedUserLine;
use log::{error, info, trace};
use reqwest::header::HeaderMap;
use reqwest::ClientBuilder;
use thiserror::Error;
use url::Url;

/// Fetch spam data from Farcaster Github repo.
pub struct GithubFetcher {
    base_url: Url,
    status_url: Url,
    header_map: Option<HeaderMap>,
}

impl Default for GithubFetcher {
    fn default() -> Self {
        let base_url = Url::parse("https://raw.githubusercontent.com/warpcast/labels/").unwrap();
        let status_url =
            Url::parse("https://api.github.com/repos/warpcast/labels/commits").unwrap();
        Self {
            header_map: None,
            base_url,
            status_url,
        }
    }
}

impl GithubFetcher {
    /// Set the base url of the fetches made to GitHub.
    pub fn with_base_url(mut self, base_url: Url) -> Self {
        self.base_url = base_url;
        self
    }

    /// Set the URL that returns a summary of all commits.
    pub fn with_status_url(mut self, status_url: Url) -> Self {
        self.status_url = status_url;
        self
    }

    /// Set the API header used for calls.
    pub fn with_api_header(self, map: HeaderMap) -> Self {
        Self {
            header_map: Some(map),
            ..self
        }
    }

    /// Show the URL for a call without actually making a call.
    pub fn api_call_from_endpoint(&self, endpoint: &str) -> Result<Url, ImporterError> {
        self.build_path(endpoint)
            .map_err(|_| ImporterError::InvalidEndpoint)
    }

    /// method used internally to make all api calls.
    async fn api_call(&self, api_call: Url) -> Result<String, ImporterError> {
        trace!("making api call to {api_call}");
        let client = if let Some(map) = &self.header_map {
            ClientBuilder::new()
                .user_agent("farmap")
                .default_headers(map.clone())
                .build()?
        } else {
            ClientBuilder::new().user_agent("farmap").build()?
        };
        let res = client.get(api_call.to_string()).send().await?;
        trace!("header of response: {:?}", res.headers());

        info!("response with statuscode {}", res.status());
        if !res.status().is_success() {
            error!("api call {} failed: {}", api_call, res.status());
            return Err(ImporterError::FailedApiRequest);
        }

        res.text().await.map_err(ImporterError::NetworkError)
    }

    pub async fn fetch_all_commit_hashes(&self) -> Result<Vec<String>, ImporterError> {
        let api_response = self
            .api_call(self.status_url.clone())
            .await
            .map_err(|_| ImporterError::FailedApiRequest)?;
        github_parser::parse_status(&api_response)
    }

    pub async fn fetch_commit_hash_body(&self, name: &str) -> Result<String, ImporterError> {
        let call = self.api_call_from_endpoint(name)?;
        self.api_call(call).await
    }

    /// Returns an error when the api call could not be made with a good result. If particular line
    /// in the response cannot be parsed the method returns those error in the inner
    /// Vec<ImporterError>. The method partitions the lines into valid and invalid lines.
    pub async fn fetch(
        &self,
        commit_hash: &str,
    ) -> Result<(Vec<UnprocessedUserLine>, Vec<ImporterError>), ImporterError> {
        let body = self.fetch_commit_hash_body(commit_hash).await?;
        Ok(github_parser::parse_commit_hash_body(&body))
    }

    fn build_path(&self, status: &str) -> Result<Url, ConversionError> {
        let url_string = format!("{}{}/spam.jsonl", self.base_url, status);
        let url = Url::parse(&url_string).map_err(|_| ConversionError::ConversionError)?;
        Ok(url)
    }
}

#[derive(Error, Debug)]
pub enum ImporterError {
    #[error("I/O error: {0}")]
    IoError(#[from] std::io::Error),

    #[error("Network error: {0}")]
    NetworkError(#[from] reqwest::Error),

    #[error("Invalid directory")]
    InvalidDirectory,

    #[error("Invalid Endpoint")]
    InvalidEndpoint,

    #[error("StatusChecker Error")]
    StatusChecker,

    #[error("Bad API Response: `{0}`")]
    BadApiResponse(String),

    #[error("Failed API Request")]
    FailedApiRequest,
}

#[derive(Error, Debug)]
pub enum ConversionError {
    #[error("Could not perform conversion")]
    ConversionError,
}

#[cfg(test)]
pub mod tests {
    use super::*;

    use url::Url;

    fn create_test_importer(
        base_url: Url,
        status_check_url: Url,
    ) -> Result<GithubFetcher, ImporterError> {
        Ok(GithubFetcher::default()
            .with_base_url(base_url)
            .with_status_url(status_check_url))
    }

    #[test]
    fn check_dummy_new_ok() {
        let base_url = Url::parse("https://caz.pub").unwrap();
        let status_url = Url::parse("https://caz.pub").unwrap();
        let importer = create_test_importer(base_url, status_url);
        assert!(importer.is_ok());
    }
}
