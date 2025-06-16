//! Fetch data from external sources.
//!
//! All the functionality to fetch data from external sources should go in this module. Each source
//! can be used by the corresponding <SOURCE_NAME>Fetcher struct.
pub mod github_importer;
mod github_parser;
mod import;
mod importer_utils;
mod pinata_importer;
/// pinata parser will eventually deprecated as a public interface. The user should only need to
/// use PinataFetcher.
pub mod pinata_parser;
mod wield_importer;
mod wield_parser;
pub use import::ConversionError;
pub use import::GithubFetcher;
pub use import::ImporterError;
pub use pinata_importer::PinataFetcher;
pub use wield_importer::WieldFetcher;
