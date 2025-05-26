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

    pub async fn api_request_for_id(&self, id: u64) -> Result<Response, ImporterError> {
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
        println!("{}", url);
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
