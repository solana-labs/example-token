use num_derive::FromPrimitive;
use solana_sdk::{info, instruction_processor_utils::DecodeError};

pub type Result<T> = std::result::Result<T, TokenError>;

#[derive(Serialize, Debug, PartialEq, FromPrimitive)]
pub enum TokenError {
    InvalidArgument,
    InvalidUserdata,
    InsufficientFunds,
    NotEnoughAccountKeys,
    NotOwner,
}

impl TokenError {
    pub fn print(&self) {
        match self {
            TokenError::InvalidArgument => info!("Error: InvalidArgument"),
            TokenError::InvalidUserdata => info!("Error: InvalidUserData"),
            TokenError::InsufficientFunds => info!("Error: InsufficientFunds"),
            TokenError::NotEnoughAccountKeys => info!("Error: NotEnoughAccountKeys"),
            TokenError::NotOwner => info!("Error: NotOwner"),
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
