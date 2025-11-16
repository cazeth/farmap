use chrono::Days;
use chrono::NaiveDate;
use clap::Parser;
use clap::Subcommand;
use farmap::fetch::local_spam_label_importer;
use farmap::fid_score_shift::ShiftSource;
use farmap::fid_score_shift::ShiftTarget;
use farmap::spam_score::DatedSpamUpdate;
use farmap::Fidded;
use farmap::SetWithSpamEntries;
use farmap::SpamScore;
use farmap::User;
use farmap::UserCollection;
use farmap::UserWithSpamData;
use farmap::UsersSubset;
use itertools::Itertools;
use simple_log::log::info;
use simple_log::log::warn;
use simple_log::LogConfigBuilder;
use std::path::PathBuf;

/// Returns the spam score distribution of warpcast label data at a certain date.
#[derive(Parser, Debug)]
struct Args {
    /// Path to data directory or file. If no path is provided the program checks $HOME/.local/share/farmap. It is necessary to
    /// either populate that directory with farcaster label data in .jsonl files or provide a path
    /// to a directory with such data
    #[arg(short, long, default_value = None)]
    path: Option<PathBuf>,

    /// Only include users with earliest spam score at or after this date.
    #[arg(short, long, default_value = None)]
    after_date: Option<String>,

    /// Only include users with earliest spam score at or before this date.
    #[arg(short,long, default_value = None)]
    before_date: Option<String>,

    /// Only include users with a particular most recent spam score.
    #[arg(short,long, default_value = None)]
    current_spam_score: Option<usize>,

    /// Only include users with a particular spam score at a particular date. Can be run multiple
    /// times to apply multiple filters
    #[arg(short,long,default_value = None , number_of_values=2, value_names = &["STRING", "NUMBER"])]
    spam_score_at_date: Option<Vec<String>>,

    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand, Debug)]
enum Commands {
    /// Generate a change matrix from the data. The change matrix tracks the spam label of each user
    /// between two dates. Prints the changes in a matrix where the rows
    /// represent the spam label at the from date and the columns represent the spam label at the to
    /// date.
    ChangeMatrix {
        #[arg(short, long)]
        from_date: String,

        #[arg(short, long)]
        to_date: String,
    },

    SpamDistribution {
        /// Date of analysis in format YYYY-MM-DD.
        /// If no date is provided the program assumes today's date.
        #[arg(short, long, default_value = None)]
        date: Option<String>,
    },

    /// Print the spam score and their dates set for a given FID.
    Fid {
        #[arg(short, long)]
        fid: usize,
    },

    /// Print all fids that are not filtered out.
    AllFids,
}

fn main() {
    let args = Args::parse();
    let (dir_path, file_path) = if let Some(p) = args.path {
        let dir_path = if p.is_file() {
            p.parent().unwrap().to_str().unwrap().to_owned()
        } else {
            p.to_str().unwrap().to_owned()
        };

        if p.is_file() {
            (dir_path, Some(p.to_str().unwrap().to_owned()))
        } else {
            (dir_path, None)
        }
    } else {
        let home_dir = std::env::var("HOME").unwrap();
        (home_dir + "/.local/share/farmap/", None)
    };

    let log_path = format!("{}/log/farmap.log", &dir_path);

    let config = LogConfigBuilder::builder()
        .path(&log_path)
        .size(100)
        .roll_count(10)
        .time_format("%Y-%m-%d %H:%M:%S") //E.g:%H:%M:%S.%f
        .level("debug")
        .unwrap()
        .output_file()
        .build();

    simple_log::new(config).unwrap();

    if let Some(p) = &file_path {
        info!("using data from file {p}");
    } else {
        info!("using data from dir {dir_path:?}")
    }

    let users = if let Some(p) = file_path {
        import_data_from_file(&p)
    } else {
        import_data_from_dir(&dir_path)
    };

    // If after_date is some, create a subset by that filter.
    // If after_date is none, create a set with all users
    let mut set = args.after_date.map_or_else(
        || UsersSubset::from(&users),
        |after_date| {
            UsersSubset::from_filter(&users, |user: &User| {
                UserWithSpamData::try_from(user)
                    .map(|user| {
                        user.earliest_spam_update().date()
                            >= NaiveDate::parse_from_str(&after_date, "%Y-%m-%d").unwrap()
                    })
                    .unwrap_or(false)
            })
        },
    );

    // Filter on before_date if that input was provided.
    if let Some(unparsed_before_date) = args.before_date {
        if let Ok(parsed_before_date) = NaiveDate::parse_from_str(&unparsed_before_date, "%Y-%m-%d")
        {
            set.filter(|user| {
                UserWithSpamData::try_from(user)
                    .map(|user| user.earliest_spam_update().date() <= parsed_before_date)
                    .unwrap_or(false)
            })
        }
    }

    if let Some(score) = args.current_spam_score {
        let expected_spam_score = SpamScore::try_from(score).unwrap();
        set.filter(|user: &User| {
            UserWithSpamData::try_from(user)
                .map(|user| user.latest_spam_update().score() == expected_spam_score)
                .unwrap_or(false)
        })
    }

    if let Some(spam_score_at_date) = args.spam_score_at_date {
        let date = spam_score_at_date
            .first()
            .and_then(|date_str| NaiveDate::parse_from_str(date_str, "%Y-%m-%d").ok());
        let score = spam_score_at_date
            .get(1)
            .and_then(|score| score.parse::<usize>().ok())
            .and_then(|score| SpamScore::try_from(score).ok());
        if let Some(date) = date {
            if let Some(score) = score {
                set.filter(|user: &User| {
                    UserWithSpamData::try_from(user)
                        .map(|user| {
                            user.spam_score_at_date(date)
                                .map(|sc| sc == score)
                                .unwrap_or(false)
                        })
                        .unwrap_or(false)
                })
            }
        }
    }

    match args.command {
        Some(Commands::ChangeMatrix { from_date, to_date }) => {
            let from_date = NaiveDate::parse_from_str(&from_date, "%Y-%m-%d").unwrap();
            let to_date = NaiveDate::parse_from_str(&to_date, "%Y-%m-%d").unwrap();
            let days = to_date.signed_duration_since(from_date).num_days();
            if days <= 0 {
                println!("The days between to_date and from_date must be greater than zero.");
                panic!();
            };

            print_change_matrix(&set, from_date, Days::new(days as u64));
        }

        Some(Commands::SpamDistribution { date }) => {
            let analysis_date = if let Some(d) = &date {
                NaiveDate::parse_from_str(d, "%Y-%m-%d").unwrap()
            } else {
                chrono::Local::now().naive_local().date()
            };
            set.filter(|user: &User| {
                UserWithSpamData::try_from(user)
                    .map(|user| user.earliest_spam_update().date() <= analysis_date)
                    .unwrap_or(false)
            });
            print_spam_score_distribution(set.clone(), analysis_date);
        }

        None => {
            // The program returns the spam distribution today if no option is provided
            let analysis_date = chrono::Local::now().naive_local().date();
            print_spam_score_distribution(set.clone(), analysis_date);
        }

        Some(Commands::Fid { fid }) => {
            if let Ok(spam_set) = SetWithSpamEntries::try_from(&set) {
                print_fid_history(&spam_set, &fid);
            } else {
                println!("no spam data in data")
            };
        }
        Some(Commands::AllFids) => {
            print_all(&set);
        }
    }
}

fn print_all(set: &UsersSubset) {
    for user in set.iter() {
        println!("{:?}", user.fid())
    }
}

fn print_fid_history(set: &SetWithSpamEntries, fid: &usize) {
    if let Some(user) = set.fid(*fid) {
        println!("Spam record history for {fid}");
        println!("------");
        for record in user.dated_spam_updates() {
            println!("{}: {}", record.date(), record.score() as usize)
        }
    } else {
        println!("no spam data for user in data set")
    }
}

fn print_change_matrix(subset: &UsersSubset, from_date: NaiveDate, days: Days) {
    if let Ok(spam_set) = SetWithSpamEntries::try_from(subset) {
        let matrix = spam_set.spam_changes_with_fid_score_shift(from_date, days);
        for row in matrix {
            let source_disp = match row.source() {
                ShiftSource::Zero => "spam",
                ShiftSource::One => "maybe",
                ShiftSource::Two => "nonspam",
                ShiftSource::New => "new",
            };

            let target_disp = match row.target() {
                ShiftTarget::Zero => "spam",
                ShiftTarget::One => "maybe",
                ShiftTarget::Two => "nonspam",
                ShiftTarget::Removed => "removed",
            };

            println!("from {} to {}: {}", source_disp, target_disp, row.count());
        }
    } else {
        println!("no spam data in set");
    }
}

fn print_spam_score_distribution(set: UsersSubset, date: NaiveDate) {
    if let Ok(set) = SetWithSpamEntries::try_from(set) {
        let count = set.user_count();
        println!(
         "Spam score distribution at date {:?}: \n 0: {:.2}% \n 1: {:.2}% \n 2: {:.2}% \n User count in set is {}",
         date,
        (set.spam_score_count_at_date(date).unwrap().spam() as f64)/ (count as f64)*100.0,
        (set.spam_score_count_at_date(date).unwrap().maybe_spam() as f64)/ (count as f64)*100.0,
        (set.spam_score_count_at_date(date).unwrap().non_spam() as f64)/ (count as f64)*100.0,
        set.user_count(),
      );
    } else {
        println!("no spam score in set");
    }
}

fn import_data_from_dir(data_dir: &str) -> UserCollection {
    let results =
        local_spam_label_importer::import_data_from_dir_with_collected_res(data_dir).unwrap();
    let (oks, errors): (Vec<_>, Vec<_>) = results.into_iter().partition_result();
    for error in errors {
        warn!("non-fatal error on import: {error:?}")
    }

    let user_lines: Vec<Fidded<DatedSpamUpdate>> =
        oks.into_iter().map(|x| x.try_into().unwrap()).collect_vec();
    let mut collection = UserCollection::default();
    collection.add_user_value_iter(user_lines);

    collection
}

fn import_data_from_file(data_path: &str) -> UserCollection {
    let results =
        local_spam_label_importer::import_data_from_file_with_collected_res(data_path).unwrap();
    let (oks, errors): (Vec<_>, Vec<_>) = results.into_iter().partition_result();
    for error in errors {
        warn!("non-fatal error on import: {error:?}")
    }
    let user_lines: Vec<Fidded<DatedSpamUpdate>> =
        oks.into_iter().map(|x| x.try_into().unwrap()).collect_vec();
    let mut collection = UserCollection::default();
    collection.add_user_value_iter(user_lines);

    collection
}

#[cfg(test)]
pub mod tests {
    use std::env;

    use assert_cmd::Command;

    #[test]
    fn test_read_from_file_with_all_fids_on_dummy_data() {
        let current_dir = env::current_dir().unwrap();
        let path_arg = format!(
            "-p{}{}",
            current_dir.to_str().unwrap(),
            "/data/dummy-data/spam_2.jsonl"
        );

        Command::new("cargo")
            .arg("run")
            .arg("--")
            .arg(path_arg.clone())
            .arg("all-fids")
            .assert()
            .stdout("1\n");
    }

    #[test]
    fn test_distribution_on_dummy_data() {
        let current_dir = env::current_dir().unwrap();
        let path_arg = format!("-p{}{}", current_dir.to_str().unwrap(), "/data/dummy-data/");
        Command::new("cargo")
            .arg("run")
            .arg("--")
            .arg(path_arg.clone())
            .arg("spam-distribution")
            .arg("-d 2025-01-01")
            .assert()
            .stdout(
                "Spam score distribution at date 2025-01-01: \n 0: 0.00% \n 1: 100.00% \n 2: 0.00% \n User count in set is 1\n",
            );

        Command::new("cargo")
            .arg("run")
            .arg("--")
            .arg(path_arg)
            .arg("spam-distribution")
            .arg("-d 2025-01-23")
            .assert()
            .stdout(
                "Spam score distribution at date 2025-01-23: \n 0: 50.00% \n 1: 0.00% \n 2: 50.00% \n User count in set is 2\n",
            );
    }

    #[test]
    fn test_spam_score_at_filter_on_dummy_data() {
        let current_dir = env::current_dir().unwrap();
        let path_arg = format!("-p{}{}", current_dir.to_str().unwrap(), "/data/dummy-data/");
        Command::new("cargo")
            .arg("run")
            .arg("--")
            .arg(path_arg.clone())
            .arg("-s")
            .arg("2024-01-01")
            .arg("1")
            .arg("all-fids")
            .assert()
            .stdout("1\n");

        Command::new("cargo")
            .arg("run")
            .arg("--")
            .arg(path_arg.clone())
            .arg("-s")
            .arg("2024-01-01")
            .arg("1")
            .arg("-s")
            .arg("2025-01-23")
            .arg("0")
            .arg("all-fids")
            .assert()
            .stdout("1\n");

        Command::new("cargo")
            .arg("run")
            .arg("--")
            .arg(path_arg.clone())
            .arg("-s")
            .arg("2025-01-20")
            .arg("2")
            .arg("all-fids")
            .assert()
            .stdout("");

        Command::new("cargo")
            .arg("run")
            .arg("--")
            .arg(path_arg.clone())
            .arg("-s")
            .arg("2025-01-23")
            .arg("2")
            .arg("all-fids")
            .assert()
            .stdout("2\n");
    }
}
