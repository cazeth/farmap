//! Fetch data from external sources.
//!
//! All the functionality to fetch data from external sources should go in this module. Each source
//! can be used by the corresponding <SOURCE_NAME>Fetcher struct.
mod errors;
pub mod github_importer;
mod github_parser;
mod import;
mod importer_utils;
pub mod local_spam_label_importer;
mod pinata_importer;
/// pinata parser will eventually deprecated as a public interface. The user should only need to
/// use PinataFetcher.
pub mod pinata_parser;
mod wield_importer;
mod wield_parser;
pub use errors::DataReadError;
pub use errors::InvalidJsonlError;
pub use import::ConversionError;
pub use import::GithubFetcher;
pub use import::ImporterError;
pub use pinata_importer::PinataFetcher;
pub use wield_importer::WieldFetcher;
