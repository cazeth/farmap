//! # Farmap
//! A library to parse and analyze [farcaster](https://www.farcaster.xyz/) data. The ambition is to
//! be the simplest gateway to farcaster data for your rust application. For now, the
//! library is focused on [Warpcast Spam Labels](https://github.com/warpcast/labels) and farcaster
//! protocol data via Pinata, but other types of data may be added in the future.
//! # Quickstart
//! In order to get started, you likely want to fetch data using the fetch module. Choose one or
//! several Fetchers to get the data you want. You can use and analyze the data in various ways
//! using the User, UserCollection and Subset struct.
mod analyze_spam_entry;
pub use analyze_spam_entry::SetWithSpamEntries;
mod cast_type;
mod collection_error;
mod collidable;
mod dated;
pub mod fetch;
mod fid;
pub mod fid_score_shift;
mod fidded;
mod follow_count;
mod has_tag;
mod is_user;
mod set_with_cast_data;
pub mod spam_score;
pub mod subset;
mod time_utils;
mod try_from_user;
mod try_from_user_set;
mod unprocessed_user_line;
pub mod user;
pub mod user_collection;
mod user_collection_serde;
mod user_error;
mod user_serde;
mod user_set;
mod user_value;
mod user_with_cast_data;
mod user_with_spam_data;
mod utils;
pub use crate::has_tag::HasTag;
pub use crate::unprocessed_user_line::SpamDataParseError;
pub use cast_type::CastType;
pub use cast_type::InvalidCastInputError;
pub use collection_error::CollectionError;
pub use fid::Fid;
#[doc(inline)]
pub use fid_score_shift::FidScoreShift;
pub use fidded::Fidded;
pub use follow_count::FollowCount;
pub use is_user::IsUser;
pub use set_with_cast_data::SetWithCastData;
pub use spam_score::DatedSpamScoreCount;
pub use spam_score::SpamRecord;
pub use spam_score::SpamScore;
pub use spam_score::SpamScoreCount;
pub use spam_score::SpamScoreDistribution;
#[doc(inline)]
pub use subset::UsersSubset;
pub use try_from_user::TryFromUser;
pub use try_from_user_set::TryFromUserSet;
pub use unprocessed_user_line::UnprocessedUserLine;
pub use user::User;
#[doc(inline)]
pub use user_collection::UserCollection;
pub use user_error::UserError;
pub use user_set::UserSet;
pub use user_value::UserValue;
pub use user_with_cast_data::UserWithCastData;
pub use user_with_spam_data::UserWithSpamData;
