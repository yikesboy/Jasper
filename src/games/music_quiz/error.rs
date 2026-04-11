use thiserror::Error;

#[derive(Error, Debug)]
pub enum MusicQuizError {
    #[error("There is no round in progress.")]
    NoRoundInProgress,
}
