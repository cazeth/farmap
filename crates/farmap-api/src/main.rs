mod app;
mod data;
use app::build_app;
use data::get_data;
use log::info;
use std::sync::Arc;

#[tokio::main]
async fn main() {
    simple_logger::SimpleLogger::new()
        .with_level(tracing::log::LevelFilter::Info)
        .init()
        .unwrap();
    info!("starting data import procedure... ");
    let users = get_data().await;

    info!("number of users are {}", users.user_count());
    info!("data import done!");
    let shared_users = Arc::new(users);

    let app = build_app(shared_users);

    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await.unwrap();
    axum::serve(listener, app).await.unwrap();
}
