use farmap::UserCollection;
use farmap_api::app::build_app;
use reqwest::StatusCode;
use serde_json::Value;
use std::net::SocketAddr;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::net::TcpListener;

async fn spawn_test_server() -> (SocketAddr, tokio::task::JoinHandle<()>) {
    let test_db_path = PathBuf::from("test-data/user-db.json");
    let users =
        UserCollection::create_from_db(&test_db_path).expect("Failed to load test database");

    let shared_users = Arc::new(users);

    std::env::remove_var("ALLOW_TOKEN");

    let app = build_app(shared_users);

    let listener = TcpListener::bind("127.0.0.1:0")
        .await
        .expect("Failed to bind to address");

    let addr = listener.local_addr().unwrap();

    let handle = tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });

    tokio::time::sleep(std::time::Duration::from_millis(100)).await;

    (addr, handle)
}

#[tokio::test]
async fn test_root_endpoint() {
    let (addr, _handle) = spawn_test_server().await;
    let client = reqwest::Client::new();

    let response = client
        .get(format!("http://{}/", addr))
        .send()
        .await
        .expect("Failed to send request");

    assert_eq!(response.status(), StatusCode::OK);
    let body = response.text().await.expect("Failed to read body");
    assert_eq!(body, "This is a server for farmap data.");
}

#[tokio::test]
async fn test_fid_endpoint() {
    let (addr, _handle) = spawn_test_server().await;
    let client = reqwest::Client::new();

    let response = client
        .get(format!("http://{}/100", addr))
        .send()
        .await
        .expect("Failed to send request");

    assert_eq!(response.status(), StatusCode::OK);
    let json: Value = response.json().await.expect("Failed to parse JSON");
    assert!(json.is_number());
    assert_eq!(json.as_u64(), Some(2)); // "Two" maps to 2

    let response = client
        .get(format!("http://{}/200", addr))
        .send()
        .await
        .expect("Failed to send request");

    assert_eq!(response.status(), StatusCode::OK);
    let json: Value = response.json().await.expect("Failed to parse JSON");
    assert_eq!(json.as_u64(), Some(0)); // "Zero" maps to 0

    let response = client
        .get(format!("http://{}/999999", addr))
        .send()
        .await
        .expect("Failed to send request");

    assert_eq!(response.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn test_current_spam_score_distribution() {
    let (addr, _handle) = spawn_test_server().await;
    let client = reqwest::Client::new();

    let response = client
        .get(format!("http://{}/spam_score_distribution", addr))
        .send()
        .await
        .expect("Failed to send request");

    assert_eq!(response.status(), StatusCode::OK);
    let json: Value = response.json().await.expect("Failed to parse JSON");

    // Validate it's valid JSON (structure depends on implementation)
    assert!(json.is_object() || json.is_array());
}

#[tokio::test]
async fn test_monthly_spam_scores() {
    let (addr, _handle) = spawn_test_server().await;
    let client = reqwest::Client::new();

    let response = client
        .get(format!("http://{}/monthly_spam_scores", addr))
        .send()
        .await
        .expect("Failed to send request");

    assert_eq!(response.status(), StatusCode::OK);
    let json: Value = response.json().await.expect("Failed to parse JSON");

    assert!(json.is_array());
    if let Some(array) = json.as_array() {
        if !array.is_empty() {
            let first = &array[0];
            assert!(first.is_array());
            if let Some(tuple) = first.as_array() {
                assert_eq!(tuple.len(), 2);
                assert!(tuple[0].is_string()); // date string
                assert!(tuple[1].is_array()); // [f32; 3]

                if let Some(scores) = tuple[1].as_array() {
                    assert_eq!(scores.len(), 3);
                    assert!(scores.iter().all(|v| v.is_number()));
                }
            }
        }
    }
}

#[tokio::test]
async fn test_weekly_spam_scores() {
    let (addr, _handle) = spawn_test_server().await;
    let client = reqwest::Client::new();

    let response = client
        .get(format!("http://{}/weekly_spam_scores", addr))
        .send()
        .await
        .expect("Failed to send request");

    assert_eq!(response.status(), StatusCode::OK);
    let json: Value = response.json().await.expect("Failed to parse JSON");

    assert!(json.is_array());
    if let Some(array) = json.as_array() {
        if !array.is_empty() {
            let first = &array[0];
            assert!(first.is_object());

            let obj = first.as_object().unwrap();
            assert!(obj.contains_key("date"));
            assert!(obj.contains_key("maybe"));
            assert!(obj.contains_key("nonspam"));
            assert!(obj.contains_key("spam"));

            assert!(obj["date"].is_string());
            assert!(obj["maybe"].is_number());
            assert!(obj["nonspam"].is_number());
            assert!(obj["spam"].is_number());
        }
    }
}

#[tokio::test]
async fn test_weekly_spam_scores_with_filters() {
    let (addr, _handle) = spawn_test_server().await;
    let client = reqwest::Client::new();

    // Test with FID range filter
    let response = client
        .get(format!(
            "http://{}/weekly_spam_scores?from_fid=100&to_fid=300",
            addr
        ))
        .send()
        .await
        .expect("Failed to send request");

    assert_eq!(response.status(), StatusCode::OK);
    let json: Value = response.json().await.expect("Failed to parse JSON");
    assert!(json.is_array());
}

#[tokio::test]
async fn test_weekly_spam_score_counts() {
    let (addr, _handle) = spawn_test_server().await;
    let client = reqwest::Client::new();

    let response = client
        .get(format!("http://{}/weekly_spam_scores_counts", addr))
        .send()
        .await
        .expect("Failed to send request");

    assert_eq!(response.status(), StatusCode::OK);
    let json: Value = response.json().await.expect("Failed to parse JSON");
    assert!(json.is_array() || json.is_object());
}

#[tokio::test]
async fn test_latest_moves() {
    let (addr, _handle) = spawn_test_server().await;
    let client = reqwest::Client::new();

    let response = client
        .get(format!("http://{}/latest_moves", addr))
        .send()
        .await
        .expect("Failed to send request");

    assert_eq!(response.status(), StatusCode::OK);
    let json: Value = response.json().await.expect("Failed to parse JSON");

    assert!(json.is_array());
    if let Some(array) = json.as_array() {
        assert!(!array.is_empty());

        let first = &array[0];
        assert!(first.is_object());

        let obj = first.as_object().unwrap();
        assert!(obj.contains_key("count"));
        assert!(obj.contains_key("source"));
        assert!(obj.contains_key("target"));

        assert!(obj["count"].is_number());
        assert!(obj["source"].is_string());
        assert!(obj["target"].is_string());

        let valid_scores = ["Zero", "One", "Two"];
        assert!(valid_scores.contains(&obj["source"].as_str().unwrap()));
        assert!(valid_scores.contains(&obj["target"].as_str().unwrap()));
    }
}

#[tokio::test]
async fn test_latest_moves_with_days_filter() {
    let (addr, _handle) = spawn_test_server().await;
    let client = reqwest::Client::new();

    let response = client
        .get(format!("http://{}/latest_moves?days=30", addr))
        .send()
        .await
        .expect("Failed to send request");

    assert_eq!(response.status(), StatusCode::OK);
    let json: Value = response.json().await.expect("Failed to parse JSON");
    assert!(json.is_array());
}

#[tokio::test]
async fn test_casts_for_moved() {
    let (addr, _handle) = spawn_test_server().await;
    let client = reqwest::Client::new();

    let response = client
        .get(format!("http://{}/casts_for_moved/0/1/30", addr))
        .send()
        .await
        .expect("Failed to send request");

    assert_eq!(response.status(), StatusCode::OK);
    let json: Value = response.json().await.expect("Failed to parse JSON");

    if let Some(array) = json.as_array() {
        assert_eq!(array.len(), 2);
        assert!(array[0].is_number());
        assert!(array[1].is_number());
    } else {
        assert!(json.is_null());
    }
}

#[tokio::test]
async fn test_casts_for_moved_invalid_params() {
    let (addr, _handle) = spawn_test_server().await;
    let client = reqwest::Client::new();

    // Test with invalid from value (>2)
    let response = client
        .get(format!("http://{}/casts_for_moved/3/0/30", addr))
        .send()
        .await
        .expect("Failed to send request");

    assert_ne!(response.status(), StatusCode::OK);
}

#[tokio::test]
async fn test_spam_score_distributions_for_cohort() {
    let (addr, _handle) = spawn_test_server().await;
    let client = reqwest::Client::new();

    // Test for January 2025 cohort
    let response = client
        .get(format!("http://{}/spam_score_distributions/2025/1", addr))
        .send()
        .await
        .expect("Failed to send request");

    assert_eq!(response.status(), StatusCode::OK);
    let json: Value = response.json().await.expect("Failed to parse JSON");

    // Should be an array of [date_string, [f32; 3]]
    assert!(json.is_array());
    if let Some(array) = json.as_array() {
        if !array.is_empty() {
            let first = &array[0];
            assert!(first.is_array());
            if let Some(tuple) = first.as_array() {
                assert_eq!(tuple.len(), 2);
                assert!(tuple[0].is_string());
                assert!(tuple[1].is_array());
            }
        }
    }
}

#[tokio::test]
async fn test_spam_score_distributions_for_cohort_invalid_date() {
    let (addr, _handle) = spawn_test_server().await;
    let client = reqwest::Client::new();

    // Test with invalid month (13)
    let response = client
        .get(format!("http://{}/spam_score_distributions/2025/13", addr))
        .send()
        .await
        .expect("Failed to send request");

    assert_eq!(response.status(), StatusCode::BAD_REQUEST);

    // Test with invalid month (0)
    let response = client
        .get(format!("http://{}/spam_score_distributions/2025/0", addr))
        .send()
        .await
        .expect("Failed to send request");

    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
}
