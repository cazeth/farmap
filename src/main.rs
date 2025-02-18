pub mod spam_score;
pub mod subset;
pub mod user;
pub mod utils;
use chrono::Days;
use chrono::NaiveDate;
use clap::Parser;
use clap::Subcommand;
use std::path::PathBuf;
use subset::UsersSubset;
use user::UnprocessedUserLine;
use user::User;
use user::Users;

/// Returns the spam score distribution of warpcast label data at a certain date.
#[derive(Parser, Debug)]
struct Args {
    /// Path to data directory. If no path is provided the program checks $HOME/.local/share/farmap. It is necessary to
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
}

fn main() {
    let args = Args::parse();
    let path = if let Some(p) = args.path {
        p.to_str().unwrap().to_owned()
    } else {
        let home_dir = std::env::var("HOME").unwrap();
        home_dir + "/.local/share/farmap"
    };

    let users = import_data(&path);

    // If after_date is some, create a subset by that filter.
    // If after_date is none, create a set with all users
    let mut set = args.after_date.map_or_else(
        || UsersSubset::from(&users),
        |after_date| {
            UsersSubset::from_filter(&users, |user: &User| {
                user.created_at_or_after_date(
                    NaiveDate::parse_from_str(&after_date, "%Y-%m-%d").unwrap(),
                )
            })
        },
    );

    // Filter on before_date if that input was provided.
    if let Some(before_date) = args.before_date {
        set.filter(|user: &User| {
            user.created_at_or_before_date(
                NaiveDate::parse_from_str(&before_date, "%Y-%m-%d").unwrap(),
            )
        })
    };

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
            set.filter(|user: &User| user.created_at_or_before_date(analysis_date));
            print_spam_score_distribution(&set, analysis_date);
        }

        None => {
            // The program returns the spam distribution today if no option is provided
            let analysis_date = chrono::Local::now().naive_local().date();
            print_spam_score_distribution(&set, analysis_date);
        }

        Some(Commands::Fid { fid }) => {
            print_fid_history(&set, &fid);
        }
    }
}

fn print_fid_history(set: &UsersSubset, fid: &usize) {
    println!("Spam record history for {}", fid);
    println!("------");
    for record in set.user(*fid).unwrap().all_spam_records() {
        println!("{:?}: {:?}", record.1, record.0 as usize);
    }
}

fn print_spam_score_distribution(set: &UsersSubset, date: NaiveDate) {
    println!(
        "Spam score distribution at date {:?}: \n 0: {:.2}% \n 1: {:.2}% \n 2: {:.2}% \n User count in set is {}",
        date,
        set.spam_score_distribution_at_date(date).unwrap()[0]*100.0,
        set.spam_score_distribution_at_date(date).unwrap()[1]*100.0,
        set.spam_score_distribution_at_date(date).unwrap()[2]*100.0,
        set.user_count(),
    );
}

fn print_change_matrix(subset: &UsersSubset, from_date: NaiveDate, days: Days) {
    let matrix = subset.spam_change_matrix(from_date, days);
    for row in matrix {
        for element in row {
            print!(" {} ", element)
        }
        println!();
    }
}

fn import_data(data_dir: &str) -> Users {
    // for now just panic if the path doesn't exist or is not jsonl.
    let unprocessed_user_lines =
        UnprocessedUserLine::import_data_from_dir_with_res(data_dir).unwrap();

    let mut users = Users::default();

    for line in unprocessed_user_lines {
        let user = match User::try_from(line) {
            Ok(user) => user,
            Err(err) => {
                eprintln!("got an error of type {:?}. Skipping line...", err);
                continue;
            }
        };

        if let Err(err) = users.push_with_res(user) {
            eprintln!(
                "got an error of type {:?} when trying to push user to collection.",
                err
            )
        }
    }

    users
}

#[cfg(test)]
pub mod tests {
    use std::env;

    use assert_cmd::Command;

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
}
