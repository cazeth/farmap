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
use user::User;
use user::Users;

/// Returns the spam score distribution of warpcast label data at a certain date.
#[derive(Parser, Debug)]
struct Args {
    /// Date of analysis in format YYYY-MM-DD.
    /// If no date is provided the program assumes today's date.
    #[arg(short, long, default_value = None)]
    date: Option<String>,

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

    if args.date.is_some() {
        set.filter(|user: &User| {
            !user.created_at_or_after_date(date.checked_add_days(Days::new(1)).unwrap())
            // created
            // at or
            // before
            // date
        })
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
        None => {
            print_spam_score_distribution(&set, date);
        }
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
