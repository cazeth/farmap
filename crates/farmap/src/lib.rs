//! # Farmap
//! A library to parse and analyze [farcaster](https://www.farcaster.xyz/) data. For now, the
//! library is focused on [Warpcast Spam Labels](https://github.com/warpcast/labels) but may add
//! other type of data in the future.
//! # Quickstart
//! In order to get started, you likely want to store Warpcast Spam Labels data locally. Then you
//! want to use one of the available create methods in UserCollections to import this data. Then
//! you likely want to create a subset of this data with various filters applied and do analysis on
//! it and use it in other applications.
pub mod fid_score_shift;
pub mod github_importer;
pub mod import;
pub mod spam_score;
pub mod standard_importer;
pub mod subset;
pub mod user;
pub mod user_collection;
mod utils;
#[doc(inline)]
pub use fid_score_shift::FidScoreShift;
pub use github_importer::new_github_importer;
pub use import::Importer;
pub use spam_score::SpamRecord;
pub use spam_score::SpamScore;
pub use spam_score::SpamScoreCount;
#[doc(inline)]
pub use subset::UsersSubset;
pub use user::InvalidInputError;
pub use user::User;
pub use user::UserError;
#[doc(inline)]
pub use user_collection::UserCollection;
