use chrono::NaiveDate;
use chrono::NaiveDateTime;
use chrono::NaiveTime;
use farmap::fetch::pinata_parser::{
    followers_from_pinata_response, number_of_casts_from_response, reaction_times_from_response,
};
use farmap::fetch::PinataFetcher;
use std::collections::HashSet;
use std::fs::read_to_string;
use url::Url;

#[tokio::test]
async fn test_commit_call_against_mock_pinata_data() {
    let mut server = mockito::Server::new_async().await;
    let mock_status_data = read_to_string("./test-data/pinata-mock/api-body.json")
        .expect("api file should exist in data dir");
    let _ = server
        .mock("GET", "/v1/castsByFid?fid=11720")
        .with_body(mock_status_data)
        .create_async()
        .await;

    let fetcher = PinataFetcher::default()
        .with_base_url(Url::parse(&format!("{}/v1/", &server.url())).unwrap());

    let response = fetcher
        .casts_by_fid(11720)
        .await
        .expect("Mock API call should not fail");
    println!("{:?}", response);
    assert_eq!(1, number_of_casts_from_response(response).await.unwrap());
}

#[tokio::test]
async fn test_followers_from_pinata_data() {
    let mut server = mockito::Server::new_async().await;
    let mock_data = read_to_string("./test-data/pinata-mock/api-body-link.json")
        .expect("api file should exist in data dir");
    let _ = server
        .mock(
            "GET",
            "/v1/linksByTargetFid?link_type=follow&target_fid=11720",
        )
        .with_body(mock_data)
        .create_async()
        .await;

    let fetcher = PinataFetcher::default()
        .with_base_url(Url::parse(&format!("{}/v1/", &server.url())).unwrap());
    let response = fetcher
        .link_request_for_fid(11720)
        .await
        .expect("Mock API call should not fail");
    let result = followers_from_pinata_response(response)
        .await
        .unwrap()
        .into_iter()
        .collect::<HashSet<u64>>();
    assert!(result.contains(&1));
    assert!(result.contains(&2));
}

#[tokio::test]
async fn test_likes_from_pinata_data() {
    let mut server = mockito::Server::new_async().await;
    let mock_data = read_to_string("./test-data/pinata-mock/api-body-likes.json")
        .expect("api file should exist in data dir");
    let _ = server
        .mock("GET", "/v1/reactionsByFid?reaction_type=Like&fid=11720")
        .with_body(mock_data)
        .create_async()
        .await;

    let fetcher = PinataFetcher::default()
        .with_base_url(Url::parse(&format!("{}/v1/", &server.url())).unwrap());
    let response = fetcher
        .likes_by_fid(11720)
        .await
        .expect("Mock API call should not fail");
    dbg!(&response);
    let result = reaction_times_from_response(response)
        .await
        .unwrap()
        .into_iter()
        .collect::<HashSet<NaiveDateTime>>();
    let expected_result = HashSet::from([
        NaiveDateTime::new(
            NaiveDate::from_ymd_opt(2021, 1, 2).unwrap(),
            NaiveTime::from_hms_opt(0, 0, 0).unwrap(),
        ),
        NaiveDateTime::new(
            NaiveDate::from_ymd_opt(2022, 1, 1).unwrap(),
            NaiveTime::from_hms_opt(0, 0, 0).unwrap(),
        ),
    ]);
    assert_eq!(result, expected_result);
}
