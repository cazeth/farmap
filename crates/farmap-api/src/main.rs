use axum::{
    extract::State,
    http::{HeaderValue, Method},
    routing::get,
    Json, Router,
};
use farmap::UserCollection;
use serde_json::{json, Value};
use std::sync::Arc;
use tower_http::cors::CorsLayer;

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();

    let cors_layer = CorsLayer::new()
        .allow_origin(vec![HeaderValue::from_static("http://localhost:5173")]) // Open access to selected route
        .allow_methods([Method::GET, Method::POST]);

    let (users, _) =
        UserCollection::create_from_dir_and_collect_non_fatal_errors("./data/data/").unwrap();
    println!("data import done!");

    let shared_users = Arc::new(users);

    let app = Router::new()
        .route("/", get(root))
        .route(
            "/spam_score_distribution",
            get(current_spam_score_distribution),
        )
        .with_state(shared_users)
        .layer(cors_layer);

    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await.unwrap();
    axum::serve(listener, app).await.unwrap();
}

async fn root() -> &'static str {
    "This is a server for farmap data."
}

async fn current_spam_score_distribution(State(users): State<Arc<UserCollection>>) -> Json<Value> {
    let spam_score_distribution = users.current_spam_score_distribution().unwrap();
    Json(json!(spam_score_distribution))
}
