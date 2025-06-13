use log::{error, info, trace};
use reqwest::header::HeaderMap;
use reqwest::ClientBuilder;
use std::collections::HashSet;
use std::fs;
use std::fs::File;
use std::io::Write;
use std::path::PathBuf;
use thiserror::Error;
use url::Url;

/// This struct ensures that all necessary files are available to the program.
/// It does so by checking the local file system or, if those files are unavailable or outdated,
/// the github api.
pub struct Importer {
    local_data_files: Option<Vec<PathBuf>>,
    local_data_dir: Option<PathBuf>,
    base_url: Url,
    status_url: Url,
    build_path: fn(&Url, &str) -> Result<Url, ConversionError>,
    strings_from_api_data: fn(&str) -> Result<Vec<String>, ImporterError>,
    extension: Option<String>,
    header_map: Option<HeaderMap>,
}

impl Importer {
    pub fn new(
        base_url: Url,
        build_path: fn(&Url, &str) -> Result<Url, ConversionError>,
        strings_from_api_data: fn(&str) -> Result<Vec<String>, ImporterError>,
        status_url: Url,
    ) -> Self {
        Self {
            local_data_files: None,
            local_data_dir: None,
            base_url,
            status_url,
            build_path,
            strings_from_api_data,
            extension: None,
            header_map: None,
        }
    }

    pub fn with_local_data_dir(self, directory: PathBuf) -> Result<Self, ImporterError> {
        trace!("checking local data against online data");
        trace!("local directory : {directory:?}");
        let local_data_files: Vec<PathBuf> = if !directory.exists() {
            trace!("directory does not exist, creating...");
            fs::create_dir_all(&directory).map_err(ImporterError::IoError)?;
            trace!(
                "created directory {:?}",
                std::fs::canonicalize(&directory)
                    .expect("should be able to get path to dir after it was created")
            );
            Vec::new()
        } else {
            trace!("directory already exists, checking contents...");
            let result: std::result::Result<Vec<_>, ImporterError> = fs::read_dir(&directory)
                .map_err(ImporterError::IoError)?
                .map(|x| x.map_err(ImporterError::IoError))
                .collect();
            let readable_paths = result?;
            let maybe_valid_paths: Vec<PathBuf> = readable_paths.iter().map(|x| x.path()).collect();
            maybe_valid_paths
                .iter()
                .for_each(|x| trace!("local file exists : {x:?}"));
            if maybe_valid_paths.is_empty() {
                trace!("no local files in the dir");
            }
            maybe_valid_paths
        };

        Ok(Self {
            local_data_dir: Some(directory),
            local_data_files: Some(local_data_files),
            ..self
        })
    }

    /// pass a validation criteria to use to check if file names are valid. You can, for instance,
    /// check that all of them are 40 characters or long or check that they all are digits only.
    pub fn with_local_file_name_validation(
        self,
        validator: fn(&str) -> bool,
    ) -> Result<Self, ImporterError> {
        let file_paths = self
            .local_data_files
            .as_ref()
            .ok_or(ImporterError::InvalidDirectory)?;

        let file_path_results = file_paths
            .iter()
            .inspect(|x| println!("file is {x:?}"))
            .flat_map(|x| {
                x.file_name()
                    .ok_or(ImporterError::InvalidDirectory)
                    .map(|x| x.to_str())
                    .map(|x| x.ok_or(ImporterError::InvalidDirectory))
            })
            .collect::<Result<Vec<&str>, ImporterError>>();

        let file_paths = file_path_results?;
        if file_paths.iter().all(|x| validator(x)) {
            Ok(self)
        } else {
            Err(ImporterError::InvalidDirectory)
        }
    }

    pub fn with_file_extension(self, extension: &str) -> Result<Self, ImporterError> {
        let local_data_files = self
            .local_data_files
            .as_ref()
            .ok_or(ImporterError::InvalidDirectory)?;

        trace!("checking extensions in local dir against extension {extension} ");
        // check that all files have extensions that can be parsed to a str, otherwise return an error
        let extensions = local_data_files
            .iter()
            .inspect(|x| println!("{x:?}"))
            .map(|x| {
                x.extension()
                    .and_then(|x| x.to_str())
                    .ok_or(ImporterError::InvalidDirectory)
            })
            .inspect(|x| println!("{x:?}"))
            .collect::<Result<HashSet<&str>, ImporterError>>()?;

        if extensions.iter().all(|x| *x == extension) {
            let mut result = self;
            result.extension = Some(extension.to_string());
            trace!("all files in directory are considered to have valid extensions!");
            Ok(result)
        } else {
            Err(ImporterError::InvalidDirectory)
        }
    }

    pub fn with_api_header(self, map: HeaderMap) -> Self {
        Self {
            header_map: Some(map),
            ..self
        }
    }

    /// this method is used to show what the url would be for a particual call. Its primary use
    /// case is for testing.
    pub fn api_call_from_endpoint(&self, endpoint: &str) -> Result<Url, ImporterError> {
        (self.build_path)(&self.base_url, endpoint).map_err(|_| ImporterError::InvalidEndpoint)
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

    pub async fn name_strings_from_api(&self) -> Result<Vec<String>, ImporterError> {
        let api_response = self
            .api_call(self.status_url.clone())
            .await
            .map_err(|_| ImporterError::FailedApiRequest)?;
        (self.strings_from_api_data)(&api_response)
    }

    pub fn name_strings_hash_set_from_local_data(&self) -> Result<HashSet<&str>, ImporterError> {
        let local_data_files = self
            .local_data_files
            .as_ref()
            .ok_or(ImporterError::InvalidDirectory)?;
        Ok(local_data_files
            .iter()
            .map(|x| x.file_name().unwrap().to_str().unwrap())
            .map(|x| x.split(".").next().unwrap())
            .collect::<HashSet<&str>>())
    }

    pub async fn body_from_name(&self, name: &str) -> Result<String, ImporterError> {
        let call = self.api_call_from_endpoint(name)?;
        self.api_call(call).await
    }

    /// this file should take mutable self since it should also update the state of the struct to
    /// keep track of the local filesystem.
    /// but let's keep that as a TODO
    pub async fn update_local_data_files(&self) -> Result<(), ImporterError> {
        info!("checking status against api to get missing files...");
        let local_data_dir = self
            .local_data_dir
            .as_ref()
            .ok_or(ImporterError::InvalidDirectory)?;

        let status_api_path = self.status_url.clone();
        let status_call_response = self.api_call(status_api_path).await?;
        let local_status = self
            .name_strings_hash_set_from_local_data()
            .expect("a local valid directory should exist at this point");
        let api_status = (self.strings_from_api_data)(&status_call_response).unwrap();
        let api_status_set = HashSet::from_iter(api_status.iter().map(|x| x.as_str()));
        let missing_names = api_status_set.difference(&local_status);

        for name in missing_names {
            let call_path = (self.build_path)(&self.base_url, name).unwrap();
            info!("donwloading file at {call_path:?}...");
            let body = self.api_call(call_path).await?;

            let mut new_file_path = format!("{}/{}", local_data_dir.to_str().unwrap(), name);

            new_file_path = if let Some(file_extension) = &self.extension {
                format!("{}.{}", new_file_path, file_extension)
            } else {
                new_file_path
            };

            info!("new file created: {new_file_path}");
            let mut new_file = File::create(new_file_path).expect("should be able to create file");
            new_file.write_all(body.as_bytes())?;
            info!("download completed");
        }

        Ok(())
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

    use std::collections::HashSet;
    use std::path::PathBuf;
    use url::Url;

    fn create_test_importer(
        base_dir: PathBuf,
        base_url: Url,
        status_check_url: Url,
    ) -> Result<Importer, ImporterError> {
        Ok(
            Importer::new(base_url, build_path, parse_status, status_check_url)
                .with_local_data_dir(base_dir)?
                .with_local_file_name_validation(validate_file_name)?,
        )
    }

    // these function are not used in the tests since they only test the functionality against the
    // local data without making any api calls.
    fn parse_status(_: &str) -> Result<Vec<String>, ImporterError> {
        panic!();
    }

    // this function is not used in the tests since it only tests the functionality against the
    // local data without making any api calls.
    fn build_path(_: &Url, _: &str) -> Result<Url, ConversionError> {
        panic!();
    }

    // this function is not used in the tests since it only tests the functionality against the
    // local data without making any api calls.
    fn validate_file_name(name: &str) -> bool {
        let res = !matches!(name, "invalid-file");
        dbg!(res);
        res
    }

    fn check_names_from_local_data(dir: PathBuf, expected_result: Vec<&str>) {
        let base_url = Url::parse("https://caz.pub").unwrap();
        let status_url = Url::parse("https://caz.pub").unwrap();
        let importer = Importer::new(base_url, build_path, parse_status, status_url)
            .with_local_data_dir(dir)
            .unwrap()
            .with_local_file_name_validation(validate_file_name)
            .unwrap();

        let actual_names: Vec<String> = importer
            .name_strings_hash_set_from_local_data()
            .unwrap()
            .iter()
            .map(|x| x.to_string())
            .collect();

        for expected in &expected_result {
            assert!(actual_names.contains(&expected.to_string()));
        }
        assert_eq!(expected_result.len(), actual_names.len())
    }

    fn check_dummy_new_ok_with_jsonl_file_as_valid(dir: PathBuf) {
        let base_url = Url::parse("https://caz.pub").unwrap();
        let status_url = Url::parse("https://caz.pub").unwrap();
        let importer = create_test_importer(dir, base_url, status_url)
            .unwrap()
            .with_file_extension("jsonl");
        assert!(importer.is_ok());
    }

    fn check_dummy_new_err_with_jsonl_file_as_valid(dir: PathBuf) {
        let base_url = Url::parse("https://caz.pub").unwrap();
        let status_url = Url::parse("https://caz.pub").unwrap();
        let importer = create_test_importer(dir, base_url, status_url)
            .unwrap()
            .with_file_extension("jsonl");
        assert!(importer.is_err());
    }

    fn check_names_with_jsonl_extension(dir: PathBuf, expected: &[&str]) {
        let base_url = Url::parse("https://caz.pub").unwrap();
        let status_url = Url::parse("https://caz.pub").unwrap();
        let expected_files: HashSet<&str> = HashSet::from_iter(expected.iter().copied());
        let importer = create_test_importer(dir, base_url, status_url)
            .unwrap()
            .with_file_extension("jsonl")
            .unwrap();
        let filenames = importer.name_strings_hash_set_from_local_data().unwrap();
        assert_eq!(filenames, expected_files);
    }

    #[test]
    fn valid_files_without_jsonl_should_error_with_jsonl_extension() {
        check_dummy_new_err_with_jsonl_file_as_valid(PathBuf::from(
            "data/unit-tests-importer/existing-dir-with-valid-files",
        ));
    }

    #[test]
    fn jsonl_files_should_be_ok_with_jsonl_files_extension() {
        check_dummy_new_ok_with_jsonl_file_as_valid(PathBuf::from(
            "data/unit-tests-importer/existing-dir-with-jsonl/",
        ));
    }

    #[test]
    fn jsonl_files_names() {
        check_names_with_jsonl_extension(
            PathBuf::from("data/unit-tests-importer/existing-dir-with-jsonl/"),
            &["valid"],
        );
    }

    #[test]
    fn empty_dir_should_be_ok_with_jsonl_files_extension() {
        check_dummy_new_ok_with_jsonl_file_as_valid(PathBuf::from(
            "data/unit-tests-importer/existing-empty-dir",
        ));
    }

    fn check_dummy_new_ok(dir: PathBuf) {
        let base_url = Url::parse("https://caz.pub").unwrap();
        let status_url = Url::parse("https://caz.pub").unwrap();
        let importer = create_test_importer(dir, base_url, status_url);
        assert!(importer.is_ok());
    }

    fn check_dummy_new_err(dir: PathBuf) {
        let base_url = Url::parse("https://caz.pub").unwrap();
        let status_url = Url::parse("https://caz.pub").unwrap();
        let importer = create_test_importer(dir, base_url, status_url);
        assert!(importer.is_err());
    }

    #[test]
    fn new_should_work_on_existing_empty_dir() {
        check_dummy_new_ok(PathBuf::from(
            "data/unit-tests-importer/existing-empty-dir/",
        ));
    }

    #[test]
    fn new_should_work_on_dir_with_valid_file() {
        check_dummy_new_ok(PathBuf::from(
            "data/unit-tests-importer/existing-dir-with-valid-files/",
        ));
    }

    #[test]
    fn new_should_work_on_nonexisting_dir() {
        let path = PathBuf::from("data/unit-tests-importer/nonexisting-empty-dir/");
        if path.exists() {
            std::fs::remove_dir(path).unwrap();
        }

        check_dummy_new_ok(PathBuf::from(
            "data/unit-tests-importer/nonexisting-empty-dir/",
        ));
    }

    #[test]
    fn new_should_err_on_invalid_file_in_dir() {
        check_dummy_new_err(PathBuf::from(
            "data/unit-tests-importer/existing-dir-with-invalid-file",
        ));
    }

    #[test]
    fn valid_file_name() {
        check_names_from_local_data(
            PathBuf::from("data/unit-tests-importer/existing-dir-with-valid-files"),
            vec!["valid-file"],
        );
    }

    #[test]
    fn should_be_no_local_names_from_empty_dir() {
        let expected_result: Vec<&str> = Vec::new();
        check_names_from_local_data(
            PathBuf::from("data/unit-tests-importer/existing-empty-dir"),
            expected_result,
        );
    }
}
