use farmap::fetch::ConversionError;
use farmap::fetch::GithubFetcher;
use farmap::fetch::ImporterError;
use mockito::Server;
use std::path::Path;
use std::path::PathBuf;
use url::Url;

#[tokio::test]
async fn simple_mockito_test() {
    let server = create_mock_server();
    let url = Url::parse(&server.url()).unwrap();
    let status_url = Url::parse(&format!("{url}status")).unwrap();
    let data_dir_path = PathBuf::from("./data/mock-tests-1");

    clear_data_dir(&data_dir_path);
    let importer = create_mock_standard_importer(data_dir_path.clone(), url, status_url);
    assert!(importer
        .name_strings_hash_set_from_local_data()
        .unwrap()
        .is_empty());

    let _ = importer.update_local_data_files().await;

    let mut files: Vec<u8> = data_dir_path
        .read_dir()
        .unwrap()
        .map(|x| x.unwrap().file_name())
        .map(|x| x.to_str().unwrap().to_string())
        .map(|x| x.parse::<u8>().unwrap())
        .collect();
    files.sort();
    assert_eq!(files, vec![1, 2, 3]);
}

#[tokio::test]
async fn mock_with_existing_file() {
    let server = create_mock_server();
    let url = Url::parse(&server.url()).unwrap();
    let status_url = Url::parse(&format!("{url}status")).unwrap();
    let data_dir_path = PathBuf::from("data/mock-tests-2");
    let mut existing_file_path = data_dir_path.clone();
    existing_file_path.push("1");
    clear_data_dir(&data_dir_path);
    std::fs::File::create(&existing_file_path).unwrap();
    let importer = create_mock_standard_importer(data_dir_path.clone(), url, status_url);

    let _ = importer.update_local_data_files().await;
    let mut files: Vec<u8> = data_dir_path
        .read_dir()
        .unwrap()
        .map(|x| x.unwrap().file_name())
        .map(|x| x.to_str().unwrap().to_string())
        .map(|x| x.parse::<u8>().unwrap())
        .collect();
    files.sort();
    assert_eq!(files, vec![1, 2, 3]);
}

fn create_mock_standard_importer(
    data_dir_path: PathBuf,
    base_url: Url,
    status_url: Url,
) -> GithubFetcher {
    fn parse_status(_: &str) -> Result<Vec<String>, ImporterError> {
        Ok(vec!["1".to_string(), "2".to_string(), "3".to_string()])
    }

    fn build_path(base_url: &Url, _input: &str) -> Result<Url, ConversionError> {
        let url_string = format!("{}", base_url);
        Url::parse(&url_string).map_err(|_| ConversionError::ConversionError)
    }

    GithubFetcher::new(base_url, build_path, parse_status, status_url)
        .with_local_data_dir(data_dir_path)
        .unwrap()
}

fn create_mock_server() -> Server {
    let opts = mockito::ServerOpts {
        ..Default::default()
    };
    let mut server = mockito::Server::new_with_opts(opts);
    server.mock("GET", "/").with_body("hello world").create();
    server
        .mock("GET", "/status")
        .with_body("here is your status")
        .create();
    server
}

fn clear_data_dir(dir: &Path) {
    // clear out the data_dir folder so the test is the same each time if it exists
    if !dir.exists() {
        return;
    }

    for file in dir.read_dir().unwrap() {
        println!("removing file {:?}", file);
        std::fs::remove_file(file.unwrap().path()).unwrap();
    }
}
