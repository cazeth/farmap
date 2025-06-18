use farmap::fetch::GithubFetcher;
use std::fs::read_to_string;
use url::Url;

#[tokio::test]
async fn test_commit_call_against_mock_github_data() {
    let mut server = mockito::Server::new_async().await;
    let url = server.url();
    let commit_hash = "7f4a9e2b8c6d1a5f3e9b7c4d2a8f6e1b9c5d7a3f"; // fake commit hash
    let mock_status_data = read_to_string("./data/local-data-api-mirror/commit_call")
        .expect("api file should exist in data dir");
    println!("setting up server at {url}");
    println!("mock status data: {mock_status_data:?}");
    let commit_fetch_body = read_to_string("./data/dummy-data/spam.jsonl").unwrap();

    let _ = server
        .mock("GET", "/repos/warpcast/labels/commits")
        .with_body(mock_status_data)
        .create_async()
        .await;

    let _ = server
        .mock(
            "GET",
            format!("/warpcast/labels/{commit_hash}/spam.jsonl").as_str(),
        )
        .with_body(commit_fetch_body)
        .create_async()
        .await;

    let status_url = Url::parse(&format!("{url}/repos/warpcast/labels/commits")).unwrap();
    let base_url = Url::parse(&format!("{url}/warpcast/labels/")).unwrap();

    println!("status url is {status_url}");
    let importer = GithubFetcher::default()
        .with_base_url(base_url)
        .with_status_url(status_url);

    let hashes = importer.fetch_all_commit_hashes().await.unwrap();
    let result = importer.fetch(hashes.first().unwrap()).await.unwrap().0;
    assert_eq!(result.len(), 3);
}
