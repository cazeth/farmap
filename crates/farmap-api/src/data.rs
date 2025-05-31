use axum::http::{HeaderMap, HeaderValue};
use chrono::prelude::*;
use chrono::Days;
use farmap::import::ImporterError;
use farmap::pinata_parser::cast_meta_from_pinata_response;
use farmap::{new_github_importer, user::UnprocessedUserLine};
use farmap::{pinata_importer::PinataFetcher, SpamScore};
use farmap::{UserCollection, UsersSubset};
use futures::stream::{self, StreamExt};
use futures::TryStreamExt;
use log::trace;
use log::{error, info};
use serde_jsonlines::JsonLinesReader;
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

    let mut users = if readwrite_to_filesystem.get() {
        UserCollection::create_from_db(&users_db_path).unwrap_or_default()
    } else {
        UserCollection::default()
    };

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

    let _ = import_github_data(
        &local_data_dir,
        &names_data_path,
        &readwrite_to_filesystem,
        &mut users,
    )
    .await;

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
    let current_time = Local::now().date_naive();
    let two_weeks_ago = current_time.checked_sub_days(Days::new(14)).unwrap();

    let move_subset = UsersSubset::from(&*users).filtered(|x| {
        x.latest_spam_record().0 == SpamScore::Two
            && x.spam_score_at_date(&two_weeks_ago)
                .map(|x| *x == SpamScore::One || *x == SpamScore::Zero)
                .unwrap_or(false)
    });

    let pinata_fetcher = PinataFetcher::default();

    let fres = move_subset
        .iter()
        .filter(|x| {
            if x.latest_cast_record_check_date()
                .is_some_and(|x| x > two_weeks_ago)
            {
                if let Some(val) = x.cast_count() {
                    val == 0
                } else {
                    false
                }
            } else {
                false
            }
        })
        .inspect(|x| info!("making call for {x:?})"))
        .map(|x| async {
            if let Ok(response) = pinata_fetcher.api_request_for_id(x.fid() as u64).await {
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

    for cast_metas in results {
        let cast_meta = cast_metas.first();

        let fid = if let Some(cast_meta) = cast_meta {
            cast_meta.fid()
        } else {
            continue;
        };

        if let Some(user) = users.user_mut(fid as usize) {
            user.add_cast_records(cast_metas, current_time);
            info!("adding cast records to fid {fid}");
        } else {
            continue;
        };
    }
}

pub async fn import_github_data(
    local_data_dir: &Path,
    names_data_path: &Path,
    readwrite_to_filesystem: &Cell<bool>,
    users: &mut UserCollection,
) -> Result<(), ImporterError> {
    let importer = if readwrite_to_filesystem.get() {
        new_github_importer()
            .with_local_data_dir(local_data_dir.to_path_buf())
            .unwrap_or_else(|_| {
                handle_rw_error(readwrite_to_filesystem);
                new_github_importer()
            })
    } else {
        new_github_importer()
    };

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

    let api_names = importer.name_strings_from_api().await.unwrap();
    let api_names_set = HashSet::from_iter(api_names.iter().map(|x| x.to_string()));
    let missing_names = api_names_set.difference(&local_names);
    let missing_names_count = missing_names.clone().count();
    trace!("There are {} missing names", missing_names_count);

    let new_bodies = stream::iter(missing_names)
        .then(|name| importer.body_from_name(name))
        .try_collect::<Vec<_>>()
        .await?;

    for (i, body) in new_bodies.iter().enumerate() {
        let lines = JsonLinesReader::new(body.as_bytes());
        for line in lines.read_all::<UnprocessedUserLine>().flatten() {
            users.push_unprocessed_user_line(line).unwrap_or(());
        }
        trace!("finished with {i}...");
    }

    let updated_local_names = api_names;

    trace!("updated local names: {:?}", updated_local_names);

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
