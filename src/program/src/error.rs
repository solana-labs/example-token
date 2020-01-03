use num_derive::FromPrimitive;
use solana_sdk::{info, instruction_processor_utils::DecodeError};

pub type Result<T> = std::result::Result<T, TokenError>;

#[derive(Debug, PartialEq, FromPrimitive)]
pub enum TokenError {
    MissingSigner,
    InvalidArgument,
    InvalidUserdata,
    InsufficientFunds,
    NotEnoughAccountKeys,
    TokenMismatch,
    NotDelegate,
    NoOwner,
}

impl TokenError {
    pub fn print(&self) {
        match self {
            TokenError::MissingSigner => info!("Error: MissingSigner"),
            TokenError::InvalidArgument => info!("Error: InvalidArgument"),
            TokenError::InvalidUserdata => info!("Error: InvalidUserdata"),
            TokenError::InsufficientFunds => info!("Error: InsufficientFunds"),
            TokenError::NotEnoughAccountKeys => info!("Error: NotEnoughAccountKeys"),
            TokenError::TokenMismatch => info!("Error: TokenMismatch"),
            TokenError::NotDelegate => info!("Error: NotDelegate"),
            TokenError::NoOwner => info!("Error: NoOwner"),
        }
    }
}

impl<T> DecodeError<T> for TokenError {
    fn type_of() -> &'static str {
        "TokenError"
    }
}

impl std::fmt::Display for TokenError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "error")
    }
}
impl std::error::Error for TokenError {}
