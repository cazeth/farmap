use axum::{
    extract::{Path, Query, State},
    http::{HeaderValue, Method, StatusCode},
    routing::get,
    Json, Router,
};
use chrono::prelude::*;
use chrono::{Days, Months, NaiveDate};
use farmap::{new_github_importer, user::UnprocessedUserLine};
use farmap::{User, UserCollection, UsersSubset};
use log::{error, info};
use serde::Deserialize;
use serde_json::{json, Value};
use serde_jsonlines::JsonLinesReader;
use std::cell::Cell;
use std::fs::File;
use std::path::PathBuf;
use std::sync::Arc;
use std::{collections::HashSet, io::Write};
use tower_http::cors::CorsLayer;
use tower_http::trace::{self, TraceLayer};
use tracing::Level;

#[tokio::main]
async fn main() {
    let local_data_dir = PathBuf::from("./data/auto-import/".to_string());
    let users_db_path = PathBuf::from("./data/auto-import/user-db.json".to_string());
    let names_data_path = PathBuf::from("./data/app_data/names".to_string());

    let names_data_dir = names_data_path.parent().unwrap();
    let readwrite_to_filesystem: Cell<bool> = Cell::new(true);

    let handle_rw_error = || {
        error! {"read-write error - using application without reading or writing to local filesystem"};
        readwrite_to_filesystem.set(false);
    };

    if readwrite_to_filesystem.get()
        && !std::fs::exists(names_data_dir).unwrap_or_else(|_| {
            handle_rw_error();
            false
        })
    {
        std::fs::create_dir_all(names_data_dir).unwrap_or_else(|_| handle_rw_error());
    };

    if readwrite_to_filesystem.get()
        && !std::fs::exists(&local_data_dir).unwrap_or_else(|_| {
            handle_rw_error();
            false
        })
    {
        std::fs::create_dir_all(&local_data_dir).unwrap_or_else(|_| handle_rw_error());
    };

    simple_logger::SimpleLogger::new()
        .with_level(tracing::log::LevelFilter::Trace)
        .init()
        .unwrap();

    let local_names: HashSet<String> = if readwrite_to_filesystem.get()
        && std::fs::exists(&names_data_path).unwrap_or_else(|_| {
            handle_rw_error();
            false
        }) {
        HashSet::from_iter(
            std::fs::read_to_string(&names_data_path)
                .unwrap_or_else(|_| {
                    handle_rw_error();
                    "".to_string()
                })
                .lines()
                .filter(|x| !x.is_empty())
                .map(|x| x.to_string()),
        )
    } else {
        HashSet::new()
    };

    info!("tried reading local names: local_names is {local_names:#?}");

    let importer = if readwrite_to_filesystem.get() {
        new_github_importer()
            .with_local_data_dir(local_data_dir)
            .unwrap_or_else(|_| {
                handle_rw_error();
                new_github_importer()
            })
    } else {
        new_github_importer()
    };

    let api_names = importer.name_strings_from_api().await.unwrap();
    let api_names_set = HashSet::from_iter(api_names.iter().map(|x| x.to_string()));
    let missing_names = api_names_set.difference(&local_names);
    let mut updated_local_names: HashSet<String> = local_names.clone();

    let mut users = if readwrite_to_filesystem.get() {
        UserCollection::create_from_db(&users_db_path).unwrap_or_default()
    } else {
        UserCollection::default()
    };

    for name in missing_names {
        let body = importer.body_from_name(name).await.unwrap();
        let lines = JsonLinesReader::new(body.as_bytes());
        for line in lines.read_all::<UnprocessedUserLine>().flatten() {
            users.push_unprocessed_user_line(line).unwrap_or(());
        }
        updated_local_names.insert(name.clone());
    }

    if readwrite_to_filesystem.get() {
        users
            .save_to_db(&users_db_path)
            .unwrap_or_else(|_| handle_rw_error());
    };

    let local_names_file: Option<File> = if readwrite_to_filesystem.get() {
        std::fs::File::create(&names_data_path)
            .inspect_err(|_| {
                handle_rw_error();
            })
            .ok()
    } else {
        None
    };

    info!("update local names: local names file is {updated_local_names:?}");

    let local_names_output = updated_local_names
        .iter()
        .fold("".to_string(), |acc, x| format!("{acc}\n{x}"))
        .lines()
        .filter(|x| !x.is_empty())
        .collect::<Vec<&str>>()
        .join("\n");
    info!("writing names: {local_names_output:#?}");

    if let Some(mut local_names_file) = local_names_file {
        local_names_file
            .write_all(local_names_output.as_bytes())
            .unwrap_or(());
    }

    let env_var = std::env::var("FARMAP_ALLOWED_URLS").unwrap_or_default();

    let allowed_urls: Vec<HeaderValue> = env_var
        .split(',')
        .map(|x| HeaderValue::from_str(x).unwrap())
        .collect::<Vec<_>>();
    info!("allowed urls are : {allowed_urls:#?}");

    let cors_layer = CorsLayer::new()
        .allow_origin(allowed_urls)
        .allow_methods([Method::GET, Method::POST]);

    println!("number of users are {}", users.user_count());
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
