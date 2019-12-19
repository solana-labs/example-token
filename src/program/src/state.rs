use crate::error::{Result, TokenError};
use solana_sdk::{account_info::AccountInfo, info, pubkey::Pubkey};
use std::mem::size_of;

/// Represents a unique token type that all like token accounts must be
/// associated with
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Token {
    /// Total supply of tokens
    pub supply: u64,
    /// Number of base 10 digits to the right of the decimal place in the total supply
    pub decimals: u64,
}

/// Delegation details
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct TokenAccountDelegate {
    /// The source account for the tokens
    pub source: Pubkey,
    /// The original amount that this delegate account was authorized to spend up to
    pub original_amount: u64,
}

/// Account that holds or may delegate tokens
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct TokenAccount {
    /// The kind of token this account holds
    pub token: Pubkey,
    /// Owner of this account
    pub owner: Pubkey,
    /// Amount of tokens this account holds
    pub amount: u64,
    /// If `delegate` None, `amount` belongs to this account.
    /// If `delegate` is Option<_>, `amount` represents the remaining allowance
    /// of tokens that may be transferred from the `source` account.
    pub delegate: Option<TokenAccountDelegate>,
}

/// Possible states to accounts owned by the token program
#[derive(Clone, Debug, PartialEq)]
pub enum State {
    /// Unallocated
    Unallocated,
    /// Specifies a type of token
    Token(Token),
    /// Token account
    Account(TokenAccount),
    /// Invalid state
    Invalid,
}

/// Commands supported by the token program
#[derive(Clone, Debug, PartialEq)]
pub enum Command {
    /// key 0 - New token
    /// key 1 - Token account to hold tokens
    NewToken(Token),
    /// key 0 - New token account
    /// key 1 - Owner of the account
    /// key 2 - Token this account is associated with
    /// key 3 - Source account that this account is a delegate for (optional)
    NewTokenAccount,
    /// key 0 - Owner of the source account
    /// key 1 - Source/Delegate token account
    /// key 2 - Destination account
    /// key 3 - Source account if key 1 is a delegate (optional)
    Transfer(u64),
    /// key 0 - Owner of the source account
    /// key 1 - Source token account
    /// key 2 - Delegate account
    Approve(u64),
    /// key 0 - Owner of the destination account
    /// key 1 - destination token account
    /// key 2 - Owner to assign to destination account
    SetOwner,
}

impl<'a> State {
    pub fn process_newtoken<I: Iterator<Item = &'a mut AccountInfo<'a>>>(
        account_info_iter: &mut I,
        token: Token,
    ) -> Result<()> {
        let new_account_info = next_account_info(account_info_iter)?;
        let dest_account_info = next_account_info(account_info_iter)?;

        if let State::Account(mut dest_token_account) = State::deserialize(dest_account_info.data)?
        {
            if !new_account_info.is_signer {
                info!("Error: token account not a signer");
                return Err(TokenError::InvalidArgument);
            }
            if new_account_info.key != &dest_token_account.token {
                info!("Error: token mismatch");
                return Err(TokenError::InvalidArgument);
            }

            if dest_token_account.delegate.is_some() {
                info!("Error: Destination account is a delegate and cannot accept tokens");
                return Err(TokenError::InvalidArgument);
            }

            dest_token_account.amount = token.supply;
            State::Account(dest_token_account).serialize(dest_account_info.data)?;
        } else {
            info!("Error: Destination account is not an Account");
            return Err(TokenError::InvalidArgument);
        }

        if State::Unallocated != State::deserialize(new_account_info.data)? {
            info!("Error: new account is already allocated");
            return Err(TokenError::InvalidArgument);
        }
        State::Token(token).serialize(new_account_info.data)
    }

    pub fn process_newaccount<I: Iterator<Item = &'a mut AccountInfo<'a>>>(
        account_info_iter: &mut I,
    ) -> Result<()> {
        let new_account_info = next_account_info(account_info_iter)?;
        let owner_account_info = next_account_info(account_info_iter)?;
        let token_account_info = next_account_info(account_info_iter)?;

        if State::Unallocated != State::deserialize(new_account_info.data)? {
            info!("Error: account is already allocated");
            return Err(TokenError::InvalidArgument);
        }
        let mut token_account = TokenAccount {
            token: *token_account_info.key,
            owner: *owner_account_info.key,
            amount: 0,
            delegate: None,
        };
        if let Ok(delegate_account) = next_account_info(account_info_iter) {
            token_account.delegate = Some(TokenAccountDelegate {
                source: *delegate_account.key,
                original_amount: 0,
            });
        }
        State::Account(token_account).serialize(new_account_info.data)
    }

    pub fn process_transfer<I: Iterator<Item = &'a mut AccountInfo<'a>>>(
        account_info_iter: &mut I,
        amount: u64,
    ) -> Result<()> {
        let owner_account_info = next_account_info(account_info_iter)?;
        let source_account_info = next_account_info(account_info_iter)?;
        let dest_account_info = next_account_info(account_info_iter)?;

        if let (State::Account(mut source_account), State::Account(mut dest_account)) = (
            State::deserialize(source_account_info.data)?,
            State::deserialize(dest_account_info.data)?,
        ) {
            if source_account.token != dest_account.token {
                info!("Error: token mismatch");
                return Err(TokenError::InvalidArgument);
            }

            if dest_account.delegate.is_some() {
                info!("Error: destination account is a delegate and cannot accept tokens");
                return Err(TokenError::InvalidArgument);
            }

            if !owner_account_info.is_signer || owner_account_info.key != &source_account.owner {
                info!("Error: source account owner not present");
                return Err(TokenError::InvalidArgument);
            }

            if source_account.amount < amount {
                return Err(TokenError::InsufficientFunds);
            }

            source_account.amount -= amount;
            State::Account(source_account.clone()).serialize(source_account_info.data)?;

            if let Some(ref delegate) = source_account.delegate.clone() {
                let delegate_account = source_account;
                let source_account_info = next_account_info(account_info_iter)?;

                if let State::Account(mut source_account) =
                    State::deserialize(source_account_info.data)?
                {
                    if source_account.token != delegate_account.token {
                        info!("Error: token mismatch");
                        return Err(TokenError::InvalidArgument);
                    }
                    if source_account_info.key != &delegate.source {
                        info!("Error: Source account is not a delegate payee");
                        return Err(TokenError::InvalidArgument);
                    }

                    if source_account.amount < amount {
                        return Err(TokenError::InsufficientFunds);
                    }

                    source_account.amount -= amount;
                    State::Account(source_account).serialize(source_account_info.data)?;
                } else {
                    info!("Error: payee is an invalid account");
                    return Err(TokenError::InvalidArgument);
                }
            }

            dest_account.amount -= amount;
            State::Account(dest_account).serialize(dest_account_info.data)?;
        } else {
            info!("Error: destination and/or source accounts are invalid");
            return Err(TokenError::InvalidArgument);
        }
        Ok(())
    }

    pub fn process_approve<I: Iterator<Item = &'a mut AccountInfo<'a>>>(
        account_info_iter: &mut I,
        amount: u64,
    ) -> Result<()> {
        let owner_account_info = next_account_info(account_info_iter)?;
        let source_account_info = next_account_info(account_info_iter)?;
        let delegate_account_info = next_account_info(account_info_iter)?;

        if let (State::Account(source_account), State::Account(mut delegate_account)) = (
            State::deserialize(source_account_info.data)?,
            State::deserialize(delegate_account_info.data)?,
        ) {
            if source_account.token != delegate_account.token {
                info!("Error: token mismatch");
                return Err(TokenError::InvalidArgument);
            }

            if owner_account_info.key != &source_account.owner || !owner_account_info.is_signer {
                info!("Error: source account owner is not present");
                return Err(TokenError::InvalidArgument);
            }

            if source_account.delegate.is_some() {
                info!("Error: source account is a delegate");
                return Err(TokenError::InvalidArgument);
            }

            match &delegate_account.delegate {
                None => {
                    info!("Error: delegate account is not a delegate");
                    return Err(TokenError::InvalidArgument);
                }
                Some(delegate) => {
                    if source_account_info.key != &delegate.source {
                        info!("Error: delegate account is not a delegate of the source account");
                        return Err(TokenError::InvalidArgument);
                    }

                    delegate_account.amount = amount;
                    delegate_account.delegate = Some(TokenAccountDelegate {
                        source: delegate.source,
                        original_amount: amount,
                    });
                    State::Account(delegate_account).serialize(delegate_account_info.data)?;
                }
            }
        } else {
            info!("Error: destination and/or source accounts are not Accounts");
            return Err(TokenError::InvalidArgument);
        }
        Ok(())
    }

    pub fn process_setowner<I: Iterator<Item = &'a mut AccountInfo<'a>>>(
        account_info_iter: &mut I,
    ) -> Result<()> {
        let owner_account_info = next_account_info(account_info_iter)?;
        let dest_account_info = next_account_info(account_info_iter)?;
        let new_owner_account_info = next_account_info(account_info_iter)?;

        if let State::Account(mut dest_account) = State::deserialize(dest_account_info.data)? {
            if owner_account_info.key != &dest_account.owner || !owner_account_info.is_signer {
                info!("Error: destination account owner is not present");
                return Err(TokenError::InvalidArgument);
            }

            dest_account.owner = *new_owner_account_info.key;
            State::Account(dest_account).serialize(dest_account_info.data)?;
        } else {
            info!("Error: destination account is invalid");
            return Err(TokenError::InvalidArgument);
        }
        Ok(())
    }

    pub fn process(
        _program_id: &Pubkey,
        accounts: &'a mut [AccountInfo<'a>],
        input: &[u8],
    ) -> Result<()> {
        let command = Command::deserialize(input)?;
        info!("command deserialized");
        let account_info_iter = &mut accounts.iter_mut();

        match command {
            Command::NewToken(token_info) => {
                info!("Command: NewToken");
                Self::process_newtoken(account_info_iter, token_info)
            }
            Command::NewTokenAccount => {
                info!("Command: NewTokenAccount");
                Self::process_newaccount(account_info_iter)
            }

            Command::Transfer(amount) => {
                info!("Command: Transfer");
                Self::process_transfer(account_info_iter, amount)
            }

            Command::Approve(amount) => {
                info!("Command: Approve");
                Self::process_approve(account_info_iter, amount)
            }

            Command::SetOwner => {
                info!("Command: SetOwner");
                Self::process_setowner(account_info_iter)
            }
        }
    }

    pub fn deserialize(input: &'a [u8]) -> Result<Self> {
        if input.len() < size_of::<u8>() {
            return Err(TokenError::InvalidUserdata);
        }
        Ok(match input[0] {
            0 => Self::Unallocated,
            1 => {
                if input.len() < size_of::<u8>() + size_of::<Token>() {
                    return Err(TokenError::InvalidUserdata);
                }
                let token: &Token = unsafe { &*(&input[1] as *const u8 as *const Token) };
                Self::Token(*token)
            }
            2 => {
                if input.len() < size_of::<u8>() + size_of::<TokenAccount>() {
                    return Err(TokenError::InvalidUserdata);
                }
                let account: &TokenAccount =
                    unsafe { &*(&input[1] as *const u8 as *const TokenAccount) };
                Self::Account(*account)
            }
            3 => Self::Invalid,
            _ => return Err(TokenError::InvalidUserdata),
        })
    }

    pub fn serialize(self: &Self, output: &mut [u8]) -> Result<()> {
        if output.len() < size_of::<u8>() {
            return Err(TokenError::InvalidUserdata);
        }
        Ok(match self {
            Self::Unallocated => output[0] = 0,
            Self::Token(token) => {
                if output.len() < size_of::<u8>() + size_of::<Token>() {
                    return Err(TokenError::InvalidUserdata);
                }
                output[0] = 1;
                let value = unsafe { &mut *(&mut output[1] as *mut u8 as *mut Token) };
                *value = *token;
            }
            Self::Account(account) => {
                if output.len() < size_of::<u8>() + size_of::<TokenAccount>() {
                    return Err(TokenError::InvalidUserdata);
                }
                output[0] = 2;
                let value = unsafe { &mut *(&mut output[1] as *mut u8 as *mut TokenAccount) };
                *value = *account;
            }
            Self::Invalid => output[0] = 3,
        })
    }
}

impl Command {
    pub fn deserialize<'a>(input: &'a [u8]) -> Result<Self> {
        if input.len() < size_of::<u8>() {
            return Err(TokenError::InvalidUserdata);
        }
        Ok(match input[0] {
            0 => {
                if input.len() < size_of::<u8>() + size_of::<Token>() {
                    return Err(TokenError::InvalidUserdata);
                }
                let token: &Token = unsafe { &*(&input[1] as *const u8 as *const Token) };
                Self::NewToken(*token)
            }
            1 => Self::NewTokenAccount,
            2 => {
                if input.len() < size_of::<u8>() + size_of::<u64>() {
                    return Err(TokenError::InvalidUserdata);
                }
                let amount: &u64 = unsafe { &*(&input[1] as *const u8 as *const u64) };
                Self::Transfer(*amount)
            }
            3 => {
                if input.len() < size_of::<u8>() + size_of::<u64>() {
                    return Err(TokenError::InvalidUserdata);
                }
                let amount: &u64 = unsafe { &*(&input[1] as *const u8 as *const u64) };
                Self::Approve(*amount)
            }
            4 => Self::SetOwner,
            _ => return Err(TokenError::InvalidUserdata),
        })
    }

    pub fn serialize(self: &Self, output: &mut [u8]) -> Result<()> {
        if output.len() < size_of::<u8>() {
            return Err(TokenError::InvalidUserdata);
        }
        Ok(match self {
            Self::NewToken(token) => {
                if output.len() < size_of::<u8>() + size_of::<Token>() {
                    return Err(TokenError::InvalidUserdata);
                }
                output[0] = 0;
                let value = unsafe { &mut *(&mut output[1] as *mut u8 as *mut Token) };
                *value = *token;
            }
            Self::NewTokenAccount => output[0] = 1,
            Self::Transfer(amount) => {
                if output.len() < size_of::<u8>() + size_of::<u64>() {
                    return Err(TokenError::InvalidUserdata);
                }
                output[0] = 2;
                let value = unsafe { &mut *(&mut output[1] as *mut u8 as *mut u64) };
                *value = *amount;
            }
            Self::Approve(amount) => {
                if output.len() < size_of::<u8>() + size_of::<u64>() {
                    return Err(TokenError::InvalidUserdata);
                }
                output[0] = 3;
                let value = unsafe { &mut *(&mut output[1] as *mut u8 as *mut u64) };
                *value = *amount;
            }
            Self::SetOwner => output[0] = 4,
        })
    }
}

/// Return the next AccountInfo or a NotEnoughAccountKeys error
pub fn next_account_info<I: Iterator>(iter: &mut I) -> Result<I::Item> {
    iter.next().ok_or(TokenError::NotEnoughAccountKeys)
}

// Pulls in the stubs required for `info!()`
#[cfg(not(target_arch = "bpf"))]
solana_sdk_bpf_test::stubs!();

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    pub fn serde() {
        assert_eq!(State::deserialize(&[0]), Ok(State::default()));

        let mut data = vec![0; 256];

        let account = State::Account(TokenAccount {
            token: Pubkey::new(&[1; 32]),
            owner: Pubkey::new(&[2; 32]),
            amount: 123,
            delegate: None,
        });
        account.serialize(&mut data).unwrap();
        assert_eq!(State::deserialize(&data), Ok(account));

        let account = State::Token(Token {
            supply: 12345,
            decimals: 2,
        });
        account.serialize(&mut data).unwrap();
        assert_eq!(State::deserialize(&data), Ok(account));
    }

    #[test]
    pub fn serde_expect_fail() {
        let mut data = vec![0; 256];

        // Certain State's may not be serialized
        let account = State::default();
        assert_eq!(account, State::Unallocated);
        assert!(account.serialize(&mut data).is_err());
        assert!(account.serialize(&mut data).is_err());
        let account = State::Invalid;
        assert!(account.serialize(&mut data).is_err());

        // Bad deserialize data
        assert!(State::deserialize(&[]).is_err());
        assert!(State::deserialize(&[1]).is_err());
        assert!(State::deserialize(&[1, 2]).is_err());
        assert!(State::deserialize(&[2, 2]).is_err());
        assert!(State::deserialize(&[3]).is_err());
    }
}
