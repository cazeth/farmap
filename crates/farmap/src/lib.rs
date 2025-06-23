//! # Farmap
//! A library to parse and analyze [farcaster](https://www.farcaster.xyz/) data. The ambition is to
//! be the simplest gateway to farcaster data for your rust application. For now, the
//! library is focused on [Warpcast Spam Labels](https://github.com/warpcast/labels) and farcaster
//! protocol data via Pinata, but may other types of data may be added in the future.
//! # Quickstart
//! In order to get started, you likely want to store Warpcast Spam Labels data locally. Then you
//! want to use one of the available create methods in UserCollections to import this data. Then
//! you likely want to create a subset of this data with various filters applied and do analysis on
//! it and use it in other applications.
pub mod cast_meta;
pub mod fetch;
pub mod fid_score_shift;
pub mod spam_score;
pub mod subset;
mod unprocessed_user_line;
pub mod user;
pub mod user_collection;
mod utils;
#[doc(inline)]
pub use fid_score_shift::FidScoreShift;
pub use spam_score::SpamRecord;
pub use spam_score::SpamScore;
pub use spam_score::SpamScoreCount;
pub use spam_score::SpamScoreDistribution;
#[doc(inline)]
pub use subset::UsersSubset;
pub use unprocessed_user_line::UnprocessedUserLine;
pub use user::InvalidInputError;
pub use user::User;
pub use user::UserError;
#[doc(inline)]
pub use user_collection::UserCollection;
