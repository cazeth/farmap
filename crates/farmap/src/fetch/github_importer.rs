use super::import::new_github_importer_with_specific_status_url_and_base_url;
use super::GithubFetcher;
use url::Url;

#[deprecated(note = "use the default implementation of GithubFetcher instead")]
pub fn new_github_importer() -> GithubFetcher {
    let base_url = Url::parse("https://raw.githubusercontent.com/warpcast/labels/").unwrap();
    let status_check_url =
        Url::parse("https://api.github.com/repos/warpcast/labels/commits").unwrap();

    new_github_importer_with_specific_status_url_and_base_url(base_url, status_check_url)
}
