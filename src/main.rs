pub mod spam_score;
pub mod user;
pub mod utils;
use chrono::NaiveDate;
use clap::Parser;
use std::path::PathBuf;
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

    println!(
        "The spam score distribution at date {:?} is {:?}. The number of users included is {:?} (total {})",
        date,
        users.spam_score_distribution_at_date(date),
        users.user_count_at_date(date),
        users.user_count(),
    );
}
