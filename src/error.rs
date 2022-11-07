use thiserror::Error;

#[derive(Error, Debug)]
pub enum Error {
    #[error(transparent)]
    DecodingError(#[from] bincode::Error),

    #[error(transparent)]
    DatabaseError(#[from] sled::Error),

    #[error(transparent)]
    TelegramError(#[from] teloxide::RequestError), // source and Display delegate to anyhow::Error
}
