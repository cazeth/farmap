use thiserror::Error;

#[derive(Error, Debug)]
#[error("Error while retrieving data")]
pub enum RetrieveError {
    InvalidFetchedData,
    CouldNotFetchData,
}
