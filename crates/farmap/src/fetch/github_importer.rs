use super::ConversionError;
use super::GithubFetcher;
use super::ImporterError;
use serde_json::Value;
use url::Url;

pub fn new_github_importer() -> GithubFetcher {
    let base_url = Url::parse("https://raw.githubusercontent.com/warpcast/labels/").unwrap();
    let status_check_url =
        Url::parse("https://api.github.com/repos/warpcast/labels/commits").unwrap();

    new_github_importer_with_specific_status_url_and_base_url(base_url, status_check_url)
}

pub fn new_github_importer_with_specific_status_url_and_base_url(
    base_url: Url,
    status_check_url: Url,
) -> GithubFetcher {
    fn parse_status(input: &str) -> Result<Vec<String>, ImporterError> {
        let json_value: Value = serde_json::from_str(input)
            .map_err(|_| ImporterError::BadApiResponse(input.to_string()))?;

        let array = json_value
            .as_array()
            .ok_or(ImporterError::BadApiResponse(input.to_string()))?;

        array
            .iter()
            .map(|x| {
                x.as_object()
                    .ok_or(ImporterError::BadApiResponse(input.to_string()))
                    .and_then(|x| {
                        x.get("sha")
                            .ok_or(ImporterError::BadApiResponse(input.to_string()))
                    })
                    .map(|x| x.to_string().replace("\"", ""))
            })
            .collect::<Result<Vec<String>, ImporterError>>()
    }

    fn build_path(base_url: &Url, status: &str) -> Result<Url, ConversionError> {
        let url_string = format!("{}{}/spam.jsonl", base_url, status);
        let url = Url::parse(&url_string).map_err(|_| ConversionError::ConversionError)?;
        Ok(url)
    }

    GithubFetcher::new(base_url, build_path, parse_status, status_check_url)
}

#[cfg(test)]
pub mod tests {

    use super::*;
    use std::path::PathBuf;

    #[test]
    pub fn names_from_local_data() {
        let path = PathBuf::from("./data/fake-name-import");
        let importer = new_github_importer().with_local_data_dir(path).unwrap();
        let res = importer
            .name_strings_hash_set_from_local_data()
            .unwrap()
            .iter()
            .map(|x| x.to_string())
            .collect::<String>();

        assert!(res.contains("007f371f557b181ab7d82f5f8852290712b71828"));
    }
}
