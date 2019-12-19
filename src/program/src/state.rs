use crate::error::{Result, TokenError};
use crate::simple_serde::SimpleSerde;
use serde_derive::{Deserialize, Serialize};
use solana_sdk::{account_info::AccountInfo, info, pubkey::Pubkey};

/// Return the next KeyedAccount or a NotEnoughAccountKeys instruction error
pub fn next_account_info<'a, I: Iterator>(iter: &'a mut I) -> Result<I::Item> {
    iter.next().ok_or(TokenError::NotEnoughAccountKeys)
}

#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq)]
pub struct TokenInfo {
    /// Total supply of tokens
    pub supply: u64,

    /// Number of base 10 digits to the right of the decimal place in the total supply
    pub decimals: u8,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq)]
pub struct TokenAccountDelegateInfo {
    /// The source account for the tokens
    pub source: Pubkey,

    /// The original amount that this delegate account was authorized to spend up to
    pub original_amount: u64,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq)]
pub struct TokenAccountInfo {
    /// The kind of token this account holds
    pub token: Pubkey,

    /// Owner of this account
    pub owner: Pubkey,

    /// Amount of tokens this account holds
    pub amount: u64,

    /// If `delegate` None, `amount` belongs to this account.
    /// If `delegate` is Option<_>, `amount` represents the remaining allowance
    /// of tokens that may be transferred from the `source` account.
    pub delegate: Option<TokenAccountDelegateInfo>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub enum TokenInstruction {
    NewToken(TokenInfo),
    // key 0 - Destination new token account
    // key 1 - Owner of the account
    // key 2 - Token this account is associated with
    // key 3 - Source account that this account is a delegate for (optional)
    NewTokenAccount,
    Transfer(u64),
    Approve(u64),
    SetOwner,
}
impl SimpleSerde for TokenInstruction {}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub enum TokenState {
    Unallocated,
    Token(TokenInfo),
    Account(TokenAccountInfo),
    Invalid,
}
impl SimpleSerde for TokenState {}
impl Default for TokenState {
    fn default() -> TokenState {
        TokenState::Unallocated
    }
}

impl<'a> TokenState {
    pub fn process_newtoken<I: Iterator<Item = &'a mut AccountInfo<'a>>>(
        account_info_iter: &mut I,
        token_info: TokenInfo,
    ) -> Result<()> {
        let new_account = next_account_info(account_info_iter)?;
        let dest_account = next_account_info(account_info_iter)?;

        if let TokenState::Account(mut dest_token_account_info) =
            TokenState::deserialize(dest_account.data)?
        {
            if new_account.key != &dest_token_account_info.token || !new_account.is_signer {
                info!("Error: token mismatch");
                return Err(TokenError::InvalidArgument);
            }

            if dest_token_account_info.delegate.is_some() {
                info!("Error: Destination account is a delegate and cannot accept tokens");
                return Err(TokenError::InvalidArgument);
            }

            dest_token_account_info.amount = token_info.supply;
            TokenState::Account(dest_token_account_info).serialize(dest_account.data)?;
        } else {
            info!("Error: Destination account is not an Account");
            return Err(TokenError::InvalidArgument);
        }

        if TokenState::Unallocated != TokenState::deserialize(new_account.data)? {
            info!("Error: new account is already allocated");
            return Err(TokenError::InvalidArgument);
        }
        TokenState::Token(token_info).serialize(new_account.data)
    }

    pub fn process_newaccount<I: Iterator<Item = &'a mut AccountInfo<'a>>>(
        account_info_iter: &mut I,
    ) -> Result<()> {
        let new_account = next_account_info(account_info_iter)?;
        let owner_account = next_account_info(account_info_iter)?;
        let token_account = next_account_info(account_info_iter)?;

        if TokenState::Unallocated != TokenState::deserialize(new_account.data)? {
            info!("Error: account is already allocated");
            return Err(TokenError::InvalidArgument);
        }
        let mut token_account_info = TokenAccountInfo {
            token: *token_account.key,
            owner: *owner_account.key,
            amount: 0,
            delegate: None,
        };
        if let Ok(delegate_account) = next_account_info(account_info_iter) {
            token_account_info.delegate = Some(TokenAccountDelegateInfo {
                source: *delegate_account.key,
                original_amount: 0,
            });
        }
        TokenState::Account(token_account_info).serialize(new_account.data)
    }

    pub fn process_transfer<I: Iterator<Item = &'a mut AccountInfo<'a>>>(
        account_info_iter: &mut I,
        amount: u64,
    ) -> Result<()> {
        let owner_account = next_account_info(account_info_iter)?;
        let source_account = next_account_info(account_info_iter)?;
        let dest_account = next_account_info(account_info_iter)?;

        if let (
            TokenState::Account(mut source_account_info),
            TokenState::Account(mut dest_account_info),
        ) = (
            TokenState::deserialize(source_account.data)?,
            TokenState::deserialize(dest_account.data)?,
        ) {
            if source_account_info.token != dest_account_info.token {
                info!("Error: token mismatch");
                return Err(TokenError::InvalidArgument);
            }

            if dest_account_info.delegate.is_some() {
                info!("Error: destination account is a delegate and cannot accept tokens");
                return Err(TokenError::InvalidArgument);
            }

            if owner_account.key != &source_account_info.owner || !owner_account.is_signer {
                info!("Error: source account owner not present");
                return Err(TokenError::InvalidArgument);
            }

            if source_account_info.amount < amount {
                return Err(TokenError::InsufficientFunds);
            }

            source_account_info.amount -= amount;
            TokenState::Account(source_account_info.clone()).serialize(source_account.data)?;

            if let Some(ref delegate_info) = source_account_info.delegate {
                let delegate_account_info = source_account_info.clone();
                let source_account = next_account_info(account_info_iter)?;

                if let TokenState::Account(mut source_account_info) =
                    TokenState::deserialize(source_account.data)?
                {
                    if source_account_info.token != delegate_account_info.token {
                        info!("Error: token mismatch");
                        return Err(TokenError::InvalidArgument);
                    }
                    if source_account.key != &delegate_info.source {
                        info!("Error: Source account is not a delegate payee");
                        return Err(TokenError::InvalidArgument);
                    }

                    if source_account_info.amount < amount {
                        return Err(TokenError::InsufficientFunds);
                    }

                    source_account_info.amount -= amount;
                    TokenState::Account(source_account_info).serialize(source_account.data)?;
                } else {
                    info!("Error: payee is an invalid account");
                    return Err(TokenError::InvalidArgument);
                }
            }

            dest_account_info.amount -= amount;
            TokenState::Account(dest_account_info).serialize(dest_account.data)?;
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
        let owner_account = next_account_info(account_info_iter)?;
        let source_account = next_account_info(account_info_iter)?;
        let delegate_account = next_account_info(account_info_iter)?;

        if let (
            TokenState::Account(source_account_info),
            TokenState::Account(mut delegate_account_info),
        ) = (
            TokenState::deserialize(source_account.data)?,
            TokenState::deserialize(delegate_account.data)?,
        ) {
            if source_account_info.token != delegate_account_info.token {
                info!("Error: token mismatch");
                return Err(TokenError::InvalidArgument);
            }

            if owner_account.key != &source_account_info.owner || !owner_account.is_signer {
                info!("Error: source account owner is not present");
                return Err(TokenError::InvalidArgument);
            }

            if source_account_info.delegate.is_some() {
                info!("Error: source account is a delegate");
                return Err(TokenError::InvalidArgument);
            }

            match &delegate_account_info.delegate {
                None => {
                    info!("Error: delegate account is not a delegate");
                    return Err(TokenError::InvalidArgument);
                }
                Some(delegate_info) => {
                    if source_account.key != &delegate_info.source {
                        info!("Error: delegate account is not a delegate of the source account");
                        return Err(TokenError::InvalidArgument);
                    }

                    delegate_account_info.amount = amount;
                    delegate_account_info.delegate = Some(TokenAccountDelegateInfo {
                        source: delegate_info.source,
                        original_amount: amount,
                    });
                    TokenState::Account(delegate_account_info).serialize(delegate_account.data)?;
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
        let owner_account = next_account_info(account_info_iter)?;
        let dest_account = next_account_info(account_info_iter)?;
        let new_owner_account = next_account_info(account_info_iter)?;

        if let TokenState::Account(mut dest_account_info) =
            TokenState::deserialize(dest_account.data)?
        {
            if owner_account.key != &dest_account_info.owner || !owner_account.is_signer {
                info!("Error: destination account owner is not present");
                return Err(TokenError::InvalidArgument);
            }

            dest_account_info.owner = *new_owner_account.key;
            TokenState::Account(dest_account_info).serialize(dest_account.data)?;
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
        let command = TokenInstruction::deserialize(input)?;
        let account_info_iter = &mut accounts.iter_mut();

        match command {
            TokenInstruction::NewToken(token_info) => {
                info!("TokenInstruction: NewToken");
                Self::process_newtoken(account_info_iter, token_info)
            }
            TokenInstruction::NewTokenAccount => {
                info!("TokenInstruction: NewTokenAccount");
                Self::process_newaccount(account_info_iter)
            }

            TokenInstruction::Transfer(amount) => {
                info!("TokenInstruction: Transfer");
                Self::process_transfer(account_info_iter, amount)
            }

            TokenInstruction::Approve(amount) => {
                info!("TokenInstruction: Approve");
                Self::process_approve(account_info_iter, amount)
            }

            TokenInstruction::SetOwner => {
                info!("TokenInstruction: SetOwner");
                Self::process_setowner(account_info_iter)
            }
        }
    }
}

// Pulls in the stubs required for `info!()`
#[cfg(not(target_arch = "bpf"))]
solana_sdk_bpf_test::stubs!();

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    pub fn serde() {
        assert_eq!(TokenState::deserialize(&[0]), Ok(TokenState::default()));

        let mut data = vec![0; 256];

        let account = TokenState::Account(TokenAccountInfo {
            token: Pubkey::new(&[1; 32]),
            owner: Pubkey::new(&[2; 32]),
            amount: 123,
            delegate: None,
        });
        account.serialize(&mut data).unwrap();
        assert_eq!(TokenState::deserialize(&data), Ok(account));

        let account = TokenState::Token(TokenInfo {
            supply: 12345,
            decimals: 2,
        });
        account.serialize(&mut data).unwrap();
        assert_eq!(TokenState::deserialize(&data), Ok(account));
    }

    #[test]
    pub fn serde_expect_fail() {
        let mut data = vec![0; 256];

        // Certain TokenState's may not be serialized
        let account = TokenState::default();
        assert_eq!(account, TokenState::Unallocated);
        assert!(account.serialize(&mut data).is_err());
        assert!(account.serialize(&mut data).is_err());
        let account = TokenState::Invalid;
        assert!(account.serialize(&mut data).is_err());

        // Bad deserialize data
        assert!(TokenState::deserialize(&[]).is_err());
        assert!(TokenState::deserialize(&[1]).is_err());
        assert!(TokenState::deserialize(&[1, 2]).is_err());
        assert!(TokenState::deserialize(&[2, 2]).is_err());
        assert!(TokenState::deserialize(&[3]).is_err());
    }
}
