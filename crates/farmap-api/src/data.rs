use axum::http::{HeaderMap, HeaderValue};
use chrono::prelude::*;
use chrono::Days;
use farmap::fetch::github_parser::parse_commit_hash_body;
use farmap::fetch::pinata_parser::cast_meta_from_pinata_response;
use farmap::fetch::GithubFetcher;
use farmap::fetch::ImporterError;
use farmap::fetch::PinataFetcher;
use farmap::spam_score::DatedSpamUpdate;
use farmap::Fidded;
use farmap::SetWithSpamEntries;
use farmap::SpamScore;
use farmap::UserCollection;
use futures::stream::{self, StreamExt};
use futures::TryStreamExt;
use itertools::iproduct;
use itertools::Itertools;
use log::trace;
use log::{error, info};
use std::cell::Cell;
use std::fs::File;
use std::path::Path;
use std::path::PathBuf;
use std::{collections::HashSet, io::Write};

pub async fn get_data() -> UserCollection {
    let local_data_dir = PathBuf::from("./data/auto-import/".to_string());
    let users_db_path = PathBuf::from("./data/auto-import/user-db.json".to_string());
    let names_data_path = PathBuf::from("./data/app_data/names".to_string());
    let names_data_dir = names_data_path.parent().unwrap();
    let readwrite_to_filesystem: Cell<bool> = Cell::new(true);

    info!("reacreating user data from local database...");
    let mut users = if readwrite_to_filesystem.get() {
        UserCollection::create_from_db(&users_db_path).unwrap_or_default()
    } else {
        UserCollection::default()
    };
    info!("finished...");

    if readwrite_to_filesystem.get()
        && !std::fs::exists(names_data_dir).unwrap_or_else(|_| {
            handle_rw_error(&readwrite_to_filesystem);
            false
        })
    {
        std::fs::create_dir_all(names_data_dir)
            .unwrap_or_else(|_| handle_rw_error(&readwrite_to_filesystem));
    };

    if readwrite_to_filesystem.get()
        && !std::fs::exists(&local_data_dir).unwrap_or_else(|_| {
            handle_rw_error(&readwrite_to_filesystem);
            false
        })
    {
        std::fs::create_dir_all(&local_data_dir)
            .unwrap_or_else(|_| handle_rw_error(&readwrite_to_filesystem));
    };

    info!("starting to fetch github data");

    let _ = import_github_data(&names_data_path, &readwrite_to_filesystem, &mut users).await;

    info!("finished with github data");
    info!("number of users are {:?}", users.user_count());

    import_pinata_data(&mut users).await;

    if readwrite_to_filesystem.get() {
        users
            .save_to_db(&users_db_path)
            .unwrap_or_else(|_| handle_rw_error(&readwrite_to_filesystem));
    };

    users
}

pub async fn import_pinata_data(users: &mut UserCollection) {
    let fetch_list = pinata_fetch_list(&*users);

    let pinata_fetcher = PinataFetcher::default();
    info!("fetching cast data for {} fids", fetch_list.len());

    let fres = fetch_list
        .iter()
        .map(|x| async {
            if let Ok(response) = pinata_fetcher.casts_by_fid(*x).await {
                cast_meta_from_pinata_response(response).await.ok()
            } else {
                None
            }
        })
        .collect::<Vec<_>>();

    let results: Vec<_> = futures::future::join_all(fres)
        .await
        .into_iter()
        .flatten()
        .collect();

    for fidded_cast_metas in results {
        if fidded_cast_metas.is_empty() {
            continue;
        };

        let fid = fidded_cast_metas
            .first()
            .expect("cannot be empty this point")
            .fid();
        let cast_metas = fidded_cast_metas
            .into_iter()
            .map(|x| x.unfid())
            .collect_vec();

        if let Some(user) = users.user_mut(fid) {
            for value in cast_metas {
                let _ = user.add_user_value(value);
            }
            trace!("adding cast records to fid {fid}");
        } else {
            continue;
        };
    }
}

pub async fn import_github_data(
    names_data_path: &Path,
    readwrite_to_filesystem: &Cell<bool>,
    users: &mut UserCollection,
) -> Result<(), ImporterError> {
    let importer = GithubFetcher::default();

    let importer = if let Ok(gh_auth_token) = std::env::var("GH_AUTH_TOKEN") {
        let header_name = "authorization";
        let mut header_value: HeaderValue =
            HeaderValue::from_str(format!("Bearer {gh_auth_token}").as_str())
                .expect("invalid auth token");

        header_value.set_sensitive(true);
        let mut map = HeaderMap::new();
        map.insert(header_name, header_value);
        importer.with_api_header(map)
    } else {
        importer
    };

    let local_names: HashSet<String> = if readwrite_to_filesystem.get()
        && std::fs::exists(names_data_path).unwrap_or_else(|_| {
            handle_rw_error(readwrite_to_filesystem);
            false
        }) {
        HashSet::from_iter(
            std::fs::read_to_string(names_data_path)
                .unwrap_or_else(|_| {
                    handle_rw_error(readwrite_to_filesystem);
                    "".to_string()
                })
                .lines()
                .filter(|x| !x.is_empty())
                .map(|x| x.to_string()),
        )
    } else {
        HashSet::new()
    };

    trace!("tried reading local names: local_names is {local_names:#?}");

    let api_names = importer
        .fetch_all_commit_hashes()
        .await
        .inspect_err(|err| {
            error!("could not fetch api statuses. Aborting github data fetch");
            error!("full error message : {err}");
        })?;

    let api_names_set = HashSet::from_iter(api_names.iter().map(|x| x.to_string()));
    let missing_names = api_names_set.difference(&local_names);
    let missing_names_count = missing_names.clone().count();
    trace!("There are {missing_names_count} missing names");

    let new_bodies = stream::iter(missing_names)
        .then(|name| importer.fetch_commit_hash_body(name))
        .try_collect::<Vec<_>>()
        .await?;

    for body in new_bodies {
        let user_lines = parse_commit_hash_body(&body);
        let dated_spam_updates = user_lines
            .0
            .into_iter()
            .flat_map(Fidded::<DatedSpamUpdate>::try_from)
            .collect_vec();
        users.add_user_value_iter(dated_spam_updates);
    }

    let updated_local_names = api_names;

    trace!("updated local names: {updated_local_names:?}");

    let local_names_file: Option<File> = if readwrite_to_filesystem.get() {
        std::fs::File::create(names_data_path)
            .inspect_err(|_| {
                info!("error on filecreate names_data path");
                handle_rw_error(readwrite_to_filesystem);
            })
            .ok()
    } else {
        None
    };

    let local_names_output = updated_local_names
        .iter()
        .fold("".to_string(), |acc, x| format!("{acc}\n{x}"))
        .lines()
        .filter(|x| !x.is_empty())
        .collect::<Vec<&str>>()
        .join("\n");

    if let Some(mut local_names_file) = local_names_file {
        local_names_file
            .write_all(local_names_output.as_bytes())
            .unwrap_or(());
    }

    Ok(())
}

fn handle_rw_error(readwrite_to_filesystem: &Cell<bool>) {
    error! {"read-write error - using application without reading or writing to local filesystem"};
    readwrite_to_filesystem.set(false);
}

fn pinata_fetch_list(users: &UserCollection) -> HashSet<u64> {
    let spam_scores = [SpamScore::Zero, SpamScore::One, SpamScore::Two];
    let current_time = Local::now().date_naive();
    let previous_date = current_time.checked_sub_days(Days::new(14)).unwrap();
    let threshold: f32 = 0.01;
    let maximum_amount_of_calls = 1_000;
    let mut result_fids: HashSet<u64> = HashSet::new();

    let fill_rates = iproduct!(spam_scores, spam_scores)
        .map(|(from, to)| {
            let mut subset = SetWithSpamEntries::new(users).expect("no users with spam data");
            subset.filter(|user| {
                user.spam_score_at_date(previous_date)
                    .map(|x| x == from)
                    .unwrap_or(false)
            });
            subset.filter(|user| {
                user.spam_score_at_date(previous_date)
                    .map(|x| x == to)
                    .unwrap_or(false)
            });
            (
                subset.user_count() as f32 / users.user_count() as f32,
                from,
                to,
            )
        })
        .collect::<Vec<_>>();

    let lowest_fill_rate = fill_rates
        .iter()
        .min_by(|(rate_x, _, _), (rate_y, _, _)| rate_x.total_cmp(rate_y))
        .unwrap();

    let fill_rates_below_threshold = fill_rates
        .iter()
        .filter(|(rate, _, _)| *rate <= threshold)
        .collect::<Vec<_>>();

    let add_fids_rate_for_from_two_pair =
        |result_fids: &mut HashSet<u64>, from: SpamScore, to: SpamScore| {
            if let Some(mut subset) = SetWithSpamEntries::new(users) {
                subset.filter(|user| {
                    user.spam_score_at_date(previous_date)
                        .map(|x| x == from)
                        .unwrap_or(false)
                });
                subset.filter(|user| {
                    user.spam_score_at_date(previous_date)
                        .map(|x| x == to)
                        .unwrap_or(false)
                });

                subset.into_iter().map(|x| x.fid()).for_each(|x| {
                    result_fids.insert(x as u64);
                });
            };
        };

    add_fids_rate_for_from_two_pair(&mut result_fids, lowest_fill_rate.1, lowest_fill_rate.2);
    fill_rates_below_threshold.iter().for_each(|(_, from, to)| {
        add_fids_rate_for_from_two_pair(&mut result_fids, *from, *to);
    });

    trace!("result_fids list length is {}", result_fids.len());

    let result_fids = result_fids
        .iter()
        .copied()
        .take(maximum_amount_of_calls)
        .collect::<HashSet<u64>>();

    trace!(
        "result_fids list length is {} after shortening",
        result_fids.len()
    );

    result_fids
}
