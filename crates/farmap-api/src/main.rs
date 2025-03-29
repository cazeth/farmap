use axum::{
    extract::{Path, Query, State},
    http::{HeaderValue, Method, StatusCode},
    routing::get,
    Json, Router,
};
use chrono::prelude::*;
use chrono::{Days, Months, NaiveDate};
use farmap::{User, UserCollection, UsersSubset};
use serde::Deserialize;
use serde_json::{json, Value};
use std::sync::Arc;
use tower_http::cors::CorsLayer;
use tower_http::trace::{self, TraceLayer};
use tracing::Level;

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .init();

    let cors_layer = CorsLayer::new()
        .allow_origin(vec![HeaderValue::from_static("http://localhost:5173")]) // Open access to selected route
        .allow_methods([Method::GET, Method::POST]);

    let (users, _) =
        UserCollection::create_from_dir_and_collect_non_fatal_errors("./data/data/").unwrap();
    println!("data import done!");

    let shared_users = Arc::new(users);

    let app = Router::new()
        .route("/", get(root))
        .route("/{fid}", get(fid))
        .route(
            "/spam_score_distributions/{year}/{month}",
            get(spam_score_distributions_for_cohort),
        )
        .route(
            "/spam_score_distribution",
            get(current_spam_score_distribution),
        )
        .route(
            "/monthly_spam_scores",
            get(monthly_spam_score_distributions),
        )
        .route("/weekly_spam_scores", get(weekly_spam_score_distributions))
        .route("/weekly_spam_scores_counts", get(weekly_spam_score_counts))
        .route("/latest_moves", get(latest_moves))
        .layer(
            TraceLayer::new_for_http()
                .make_span_with(trace::DefaultMakeSpan::new().level(Level::INFO))
                .on_response(trace::DefaultOnResponse::new().level(Level::INFO)),
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

async fn latest_moves(
    Query(filters): Query<Filters>,
    Query(moves_filter): Query<MovesFilter>,
    State(users): State<Arc<UserCollection>>,
) -> Json<Value> {
    //last week changes.
    let current_time = Local::now().date_naive();
    let comparison_time = if let Some(days) = moves_filter.days {
        current_time.checked_sub_days(Days::new(days)).unwrap()
    } else {
        current_time.checked_sub_days(Days::new(14)).unwrap()
    };

    let users_ref: &UserCollection = &users;
    let mut set = UsersSubset::from(users_ref);
    if let Some(to_fid) = filters.to_fid {
        set.filter(|user: &User| user.fid() as u64 <= to_fid);
    };
    if let Some(from_fid) = filters.from_fid {
        set.filter(|user: &User| user.fid() as u64 >= from_fid);
    };

    let result = set.spam_changes_with_fid_score_shift(comparison_time, Days::new(21));

    Json(json!(result))
}

#[derive(Deserialize)]
struct MovesFilter {
    days: Option<u64>,
}

#[derive(Deserialize)]
struct Filters {
    from_fid: Option<u64>,
    to_fid: Option<u64>,
}

async fn monthly_spam_score_distributions(State(users): State<Arc<UserCollection>>) -> Json<Value> {
    let users_ref: &UserCollection = &users;
    let set = UsersSubset::from(users_ref);
    let result = set.monthly_spam_score_distributions();
    let result = result
        .iter()
        .map(|(date, y)| (date.to_string(), *y))
        .collect::<Vec<(String, [f32; 3])>>();
    Json(json!(result))
}

async fn weekly_spam_score_distributions(
    Query(filters): Query<Filters>,
    State(users): State<Arc<UserCollection>>,
) -> Json<Value> {
    let users_ref: &UserCollection = &users;
    let mut set = UsersSubset::from(users_ref);
    if let Some(to_fid) = filters.to_fid {
        set.filter(|user: &User| user.fid() as u64 <= to_fid);
    };
    if let Some(from_fid) = filters.from_fid {
        set.filter(|user: &User| user.fid() as u64 >= from_fid);
    };
    let result = set.weekly_spam_score_distributions();
    let result = result
        .iter()
        .map(|(date, y)| (date.to_string(), *y))
        .collect::<Vec<(String, [f32; 3])>>();
    Json(json!(result))
}

async fn fid(
    Path(fid): Path<u64>,
    State(users): State<Arc<UserCollection>>,
) -> Result<Json<Value>, StatusCode> {
    let spam_score = users.spam_score_by_fid(fid as usize);
    if let Some(score) = spam_score {
        Ok(Json(json!(score as u8)))
    } else {
        Err(StatusCode::NOT_FOUND)
    }
}

async fn weekly_spam_score_counts(
    Query(filters): Query<Filters>,
    State(users): State<Arc<UserCollection>>,
) -> Json<Value> {
    let users_ref: &UserCollection = &users;
    let mut set = UsersSubset::from(users_ref);
    if let Some(to_fid) = filters.to_fid {
        set.filter(|user: &User| user.fid() as u64 <= to_fid);
    };
    if let Some(from_fid) = filters.from_fid {
        set.filter(|user: &User| user.fid() as u64 >= from_fid);
    };

    let counts = set.weekly_spam_score_counts();
    Json(json!(counts))
}

async fn spam_score_distributions_for_cohort(
    Path((year, month)): Path<(u64, u64)>,
    State(users): State<Arc<UserCollection>>,
) -> Result<Json<Value>, StatusCode> {
    let users_ref: &UserCollection = &users;
    let mut set = UsersSubset::from(users_ref);
    let cohort_start_date =
        if let Some(date) = NaiveDate::from_ymd_opt(year as i32, month as u32, 1) {
            date
        } else {
            return Err(StatusCode::BAD_REQUEST);
        };

    println!(
        "checking spam score distributions for cohort created at of before {:?}",
        cohort_start_date
    );
    let cohort_end_date = cohort_start_date
        .checked_add_months(Months::new(1))
        .unwrap();
    set.filter(|user: &User| user.created_at_or_before_date(cohort_end_date));
    set.filter(|user: &User| user.created_at_or_after_date(cohort_start_date));
    let result = set.monthly_spam_score_distributions();
    let result = result
        .iter()
        .map(|(date, y)| (date.to_string(), *y))
        .collect::<Vec<(String, [f32; 3])>>();

    Ok(Json(json!(result)))
}
