use farmap::pinata_importer::PinataFetcher;
use std::fs::read_to_string;
use url::Url;

#[tokio::test]
async fn test_commit_call_against_mock_github_data() {
    let mut server = mockito::Server::new_async().await;
    let url = Url::parse(&format!("{}/v1/castsByFid", server.url()))
        .expect("mock server should be valid url");
    let mock_status_data = read_to_string("../data/pinata-mock/api-body.json")
        .expect("api file should exist in data dir");
    println!("setting up server at base url {url}");
    let _ = server
        .mock("GET", "/v1/castsByFid?fid=11720")
        .with_body(mock_status_data)
        .create_async()
        .await;

    let fetcher = PinataFetcher::default().with_base_url(url);
    let response = fetcher
        .api_request_for_id(11720)
        .await
        .expect("Mock API call should not fail");
    println!("{:?}", response);
    assert_eq!(
        1,
        fetcher
            .number_of_casts_from_response(response)
            .await
            .unwrap()
    );
}
