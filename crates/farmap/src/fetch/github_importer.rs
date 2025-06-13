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

#[cfg(test)]
pub mod tests {

    use super::*;
    use std::path::PathBuf;

    #[test]
    pub fn names_from_local_data() {
        let path = PathBuf::from("./data/fake-name-import");
        let importer = GithubFetcher::default().with_local_data_dir(path).unwrap();
        let res = importer
            .name_strings_hash_set_from_local_data()
            .unwrap()
            .iter()
            .map(|x| x.to_string())
            .collect::<String>();

        assert!(res.contains("007f371f557b181ab7d82f5f8852290712b71828"));
    }
}
