use super::pinata_parser::followers_from_pinata_response;
use super::pinata_parser::reaction_times_from_response;
use super::ImporterError;
use chrono::NaiveDateTime;
use log::trace;
use reqwest::{Client, Response};
use url::Url;

pub struct PinataFetcher {
    client: Client,
    base_url: Url,
}

impl Default for PinataFetcher {
    fn default() -> Self {
        Self {
            base_url: Url::parse("https://hub.pinata.cloud/v1/").unwrap(),
            client: Client::new(),
        }
    }
}

impl PinataFetcher {
    /// used to override the default url - mostly used for testing
    pub fn with_base_url(self, url: Url) -> Self {
        Self {
            base_url: url,
            ..self
        }
    }

    pub async fn fetch_followers_for_fid(&self, fid: u64) -> Result<Vec<u64>, ImporterError> {
        let api_response = self.link_request_for_fid(fid).await?;
        trace!("api response: {api_response:?}");
        followers_from_pinata_response(api_response).await
    }

    // fetches reaction times (i.e. a collection of times when a user has either recasted or
    // liked)
    pub async fn fetch_reaction_times_for_fid(
        &self,
        fid: u64,
    ) -> Result<Vec<NaiveDateTime>, ImporterError> {
        let likes = self.likes_by_fid(fid).await?;
        let recasts = self.recasts_by_fid(fid).await?;
        let mut reaction_times = reaction_times_from_response(likes).await?;
        let mut recast_reaction_times = reaction_times_from_response(recasts).await?;
        reaction_times.append(&mut recast_reaction_times);
        Ok(reaction_times)
    }

    pub async fn casts_by_fid(&self, id: u64) -> Result<Response, ImporterError> {
        let extension = "castsByFid";
        let mut url = self.base_url.clone().join(extension).unwrap();
        url.set_query(Some(&format!("fid={id}")));
        self.client
            .get(url)
            .send()
            .await
            .map_err(|_| ImporterError::FailedApiRequest)
    }

    pub async fn link_request_for_fid(&self, fid: u64) -> Result<Response, ImporterError> {
        let extension = "linksByTargetFid";
        let mut url = self.base_url.clone().join(extension).unwrap();
        url.set_query(Some(&format!("link_type=follow&target_fid={fid}")));
        trace!("url: {url}");
        self.client
            .get(url)
            .send()
            .await
            .map_err(|_| ImporterError::FailedApiRequest)
    }

    async fn reactions_by_fid(
        &self,
        fid: u64,
        reaction: Reaction,
    ) -> Result<Response, ImporterError> {
        let reaction_str = match reaction {
            Reaction::Like => "Like",
            Reaction::Recast => "Recast",
        };

        let extension = "reactionsByFid";
        let mut url = self.base_url.clone().join(extension).unwrap();
        url.set_query(Some(&format!("reaction_type={reaction_str}&fid={fid}")));
        self.client
            .get(url)
            .send()
            .await
            .map_err(|_| ImporterError::FailedApiRequest)
    }

    pub async fn likes_by_fid(&self, fid: u64) -> Result<Response, ImporterError> {
        self.reactions_by_fid(fid, Reaction::Like).await
    }

    pub async fn recasts_by_fid(&self, fid: u64) -> Result<Response, ImporterError> {
        self.reactions_by_fid(fid, Reaction::Recast).await
    }
}

enum Reaction {
    Like,
    Recast,
}
