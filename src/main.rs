pub mod spam_score;
pub mod subset;
pub mod user;
pub mod utils;
use chrono::NaiveDate;
use clap::Parser;
use std::path::PathBuf;
use subset::UsersSubset;
use user::User;
use user::Users;

/// Returns the spam score distribution of warpcast label data at a certain date.
#[derive(Parser, Debug)]
struct Args {
    /// Date of analysis in format YYYY-MM-DD.
    /// If no date is provided the program assumes the most recent date.
    #[arg(short, long, default_value = None)]
    date: Option<String>,

    /// Path to data directory. If no path is provided the program checks $HOME/.local/share/farmap. It is necessary to
    /// either populate that directory with farcaster label data in .jsonl files or provide a path
    /// to a directory with such data
    #[arg(short, long, default_value = None)]
    path: Option<PathBuf>,

    /// Only include users created at or after this date.
    #[arg(short, long, default_value = None)]
    created_after_date: Option<String>,
}

fn main() {
    let args = Args::parse();
    let path = if let Some(p) = args.path {
        p.to_str().unwrap().to_owned()
    } else {
        let home_dir = std::env::var("HOME").unwrap();
        home_dir + "/.local/share/farmap"
    };

    let date = if let Some(d) = &args.date {
        NaiveDate::parse_from_str(d, "%Y-%m-%d").unwrap()
    } else {
        chrono::Local::now().naive_local().date()
    };

    let users = Users::create_from_dir(&path);

    // If filter_date is some, create a subset by that filter.
    let subset = args.created_after_date.map(|filter_date| {
        UsersSubset::from_filter(&users, |user: &User| {
            user.created_at_or_after_date(
                NaiveDate::parse_from_str(&filter_date, "%Y-%m-%d").unwrap(),
            )
        })
    });

    if let Some(subset) = subset {
        println!(
            "The spam score distribution at date {:?} is {:?}. User count in subset is {}",
            date,
            subset.spam_score_distribution_at_date(date),
            subset.user_count(),
        );
    } else {
        println!(
        "The spam score distribution at date {:?} is {:?}. The number of users included is {:?} (total {})",
        date,
        users.spam_score_distribution_at_date(date),
        users.user_count_at_date(date),
        users.user_count(),
    );
    }
}
