use farmap::fetch::GithubFetcher;
use std::fs::read_to_string;
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

    let status_url = Url::parse(&format!("{url}/repos/warpcast/labels/commits")).unwrap();
    let base_url = Url::parse(&url).unwrap();

    println!("status url is {status_url}");
    let importer = GithubFetcher::default()
        .with_base_url(base_url)
        .with_status_url(status_url);

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
