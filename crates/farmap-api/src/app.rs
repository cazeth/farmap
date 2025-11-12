use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    routing::get,
    Json, Router,
};
use chrono::prelude::*;
use chrono::{Days, Months, NaiveDate};
use farmap::SetWithCastData;
use farmap::SetWithSpamEntries;
use farmap::TryFromUserSet;
use farmap::UserCollection;
use farmap::UserWithSpamData;
use log::info;
use log::trace;
use serde::Deserialize;
use serde_json::{json, Value};
use std::sync::Arc;
use tower_http::trace::{self, TraceLayer};
use tower_http::validate_request::ValidateRequestHeaderLayer;
use tracing::Level;

pub fn build_app(users: Arc<UserCollection>) -> Router {
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
        .route(
            "/casts_for_moved/{from}/{to}/{timespan}",
            get(casts_for_moved),
        )
        .layer(
            TraceLayer::new_for_http()
                .make_span_with(trace::DefaultMakeSpan::new().level(Level::INFO))
                .on_response(trace::DefaultOnResponse::new().level(Level::INFO)),
        )
        .with_state(users);

    let allow_token = std::env::var("ALLOW_TOKEN").ok();
    if let Some(allow_token) = allow_token {
        app.layer(ValidateRequestHeaderLayer::bearer(&allow_token))
    } else {
        app
    }
}

async fn root() -> &'static str {
    "This is a server for farmap data."
}

async fn current_spam_score_distribution(State(users): State<Arc<UserCollection>>) -> Json<Value> {
    let users_ref: &UserCollection = &users;
    let set = SetWithSpamEntries::new(users_ref).unwrap();
    let spam_score_distribution = set.current_spam_score_distribution();
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

    if let Some(mut set) = SetWithSpamEntries::new(users_ref) {
        if let Some(to_fid) = filters.to_fid {
            set.filter(|user: &UserWithSpamData| user.fid() as u64 <= to_fid);
        };
        if let Some(from_fid) = filters.from_fid {
            set.filter(|user: &UserWithSpamData| user.fid() as u64 >= from_fid);
        };

        let result = set.spam_changes_with_fid_score_shift(comparison_time, Days::new(21));
        Json(json!(result))
    } else {
        Json(serde_json::Value::Null)
    }
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

async fn spam_score_distributions_for_cohort(
    Path((year, month)): Path<(u64, u64)>,
    State(users): State<Arc<UserCollection>>,
) -> Result<Json<Value>, StatusCode> {
    let users_ref: &UserCollection = &users;
    let set = SetWithSpamEntries::new(users_ref);

    let mut set = if let Some(set) = set {
        set
    } else {
        return Err(StatusCode::NO_CONTENT);
    };

    let cohort_start_date =
        if let Some(date) = NaiveDate::from_ymd_opt(year as i32, month as u32, 1) {
            date
        } else {
            return Err(StatusCode::BAD_REQUEST);
        };

    trace!(
        "checking spam score distributions for cohort created at of before {cohort_start_date:?}"
    );

    let cohort_end_date = cohort_start_date
        .checked_add_months(Months::new(1))
        .unwrap();

    set.filter(|user: &UserWithSpamData| {
        let earliest_spam_date = user.earliest_spam_update().date();
        earliest_spam_date <= cohort_end_date && earliest_spam_date >= cohort_start_date
    });

    let result = set.monthly_spam_score_distributions();
    let result = result
        .iter()
        .map(|dated_distribution| {
            (
                dated_distribution.date().to_string(),
                (*dated_distribution.as_inner()).into(),
            )
        })
        .collect::<Vec<(String, [f32; 3])>>();

    Ok(Json(json!(result)))
}

async fn monthly_spam_score_distributions(
    State(users): State<Arc<UserCollection>>,
) -> Result<Json<Value>, StatusCode> {
    let users_ref: &UserCollection = &users;
    let set = SetWithSpamEntries::new(users_ref).ok_or(StatusCode::NO_CONTENT)?;
    let result = set.monthly_spam_score_distributions();
    let result = result
        .iter()
        .map(|distribution| {
            (
                distribution.date().to_string(),
                (*distribution.as_inner()).into(),
            )
        })
        .collect::<Vec<(String, [f32; 3])>>();
    Ok(Json(json!(result)))
}

async fn weekly_spam_score_distributions(
    Query(filters): Query<Filters>,
    State(users): State<Arc<UserCollection>>,
) -> Result<Json<Value>, StatusCode> {
    let users_ref: &UserCollection = &users;
    let mut set = SetWithSpamEntries::new(users_ref).ok_or(StatusCode::NO_CONTENT)?;
    if let Some(to_fid) = filters.to_fid {
        set.filter(|user: &UserWithSpamData| user.fid() as u64 <= to_fid);
    };
    if let Some(from_fid) = filters.from_fid {
        set.filter(|user: &UserWithSpamData| user.fid() as u64 >= from_fid);
    };

    let result = set.weekly_spam_score_distributions();

    Ok(Json(json!(result)))
}

async fn weekly_spam_score_counts(
    Query(filters): Query<Filters>,
    State(users): State<Arc<UserCollection>>,
) -> Json<Value> {
    let users_ref: &UserCollection = &users;
    if let Some(mut set) = SetWithSpamEntries::new(users_ref) {
        if let Some(to_fid) = filters.to_fid {
            set.filter(|user: &UserWithSpamData| user.fid() as u64 <= to_fid);
        };
        if let Some(from_fid) = filters.from_fid {
            set.filter(|user: &UserWithSpamData| user.fid() as u64 >= from_fid);
        };

        let counts = set.weekly_spam_score_counts();
        Json(json!(counts))
    } else {
        Json(serde_json::Value::Null)
    }
}

async fn casts_for_moved(
    State(users): State<Arc<UserCollection>>,
    Path((from, to, timespan)): Path<(u64, u64, u64)>,
) -> Result<Json<Value>, StatusCode> {
    if from > 2 || to > 2 || timespan > 100 {
        return Err(StatusCode::BAD_REQUEST);
    };

    let current_time = Local::now().date_naive();
    let begin_date: NaiveDate = current_time.checked_sub_days(Days::new(timespan)).unwrap();

    let users_ref: &UserCollection = &users;
    let mut set = SetWithSpamEntries::new(users_ref).ok_or(StatusCode::NO_CONTENT)?;

    info!("checking with begin date {begin_date}");
    info!("checking with current time {current_time}");

    set.filter(|user: &UserWithSpamData| {
        user.spam_score_at_date(begin_date).is_some_and(|u| {
            u == (from as usize)
                .try_into()
                .expect("already checked spam_score range")
        })
    });

    let set_size = set.user_count();

    info!("set size after begin_date filtering is {set_size}");

    set.filter(|user: &UserWithSpamData| {
        user.latest_spam_update().score()
            == (to as usize)
                .try_into()
                .expect("already checked spam score range")
    });

    info!("set size is {set_size}");

    let average_total_casts = if let Ok(cast_users) = SetWithCastData::try_from_set(set) {
        cast_users.average_total_casts()
    } else {
        0.0
    };

    Ok(Json(json!([set_size, average_total_casts])))
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
