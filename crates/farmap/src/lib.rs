//! # Farmap
//! A library to parse and analyze [farcaster](https://www.farcaster.xyz/) data. The ambition is to
//! be the simplest gateway to farcaster data for your rust application. For now, the
//! library is focused on [Warpcast Spam Labels](https://github.com/warpcast/labels) and farcaster
//! protocol data via Pinata, but other types of data may be added in the future.
//! # Quickstart
//! In order to get started, you likely want to fetch data using the fetch module. Choose one or
//! several Fetchers to get the data you want. You can use and analyze the data in various ways
//! using the User, UserCollection and Subset struct.
pub mod cast_meta;
mod dated;
pub mod fetch;
pub mod fid_score_shift;
pub mod spam_score;
pub mod subset;
mod unprocessed_user_line;
pub mod user;
pub mod user_collection;
mod user_value;
mod utils;
#[doc(inline)]
pub use fid_score_shift::FidScoreShift;
pub use spam_score::DatedSpamScoreCount;
pub use spam_score::SpamRecord;
pub use spam_score::SpamScore;
pub use spam_score::SpamScoreDistribution;
#[doc(inline)]
pub use subset::UsersSubset;
pub use unprocessed_user_line::UnprocessedUserLine;
pub use user::InvalidInputError;
pub use user::User;
pub use user::UserError;
#[doc(inline)]
pub use user_collection::UserCollection;
pub use user_value::UserValue;
