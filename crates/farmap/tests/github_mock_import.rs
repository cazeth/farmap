use farmap::fetch::github_importer;
use std::fs::read_to_string;
use std::path::PathBuf;
use url::Url;

#[tokio::test]
async fn test_commit_call_against_mock_github_data() {
    let mut server = mockito::Server::new_async().await;
    let url = server.url();
    let mock_status_data = read_to_string("./data/local-data-api-mirror/commit_call")
        .expect("api file should exist in data dir");
    println!("setting up server at {url}");
    println!("mock status data: {mock_status_data:?}");
    let _ = server
        .mock("GET", "/repos/warpcast/labels/commits")
        .with_body(mock_status_data)
        .create_async()
        .await;

    let status_url = format!("{url}/repos/warpcast/labels/commits");
    println!("status url is {status_url}");
    let data_dir_path = PathBuf::from("./data/mock-tests-github/");

    let importer = github_importer::new_github_importer_with_specific_status_url_and_base_url(
        Url::parse(&url).unwrap(),
        Url::parse(&status_url).unwrap(),
    )
    .with_local_data_dir(data_dir_path)
    .unwrap();
    let status = importer.name_strings_from_api().await.unwrap();
    println!("here are the statuses!");
    for stat in &status {
        println!("{}", stat);
    }

    assert_eq!(status.len(), 15);
    let lengths: Vec<usize> = status
        .iter()
        .map(|x| x.to_string().chars().count())
        .collect();
    assert!(lengths.iter().all(|x| *x == 40));
}
