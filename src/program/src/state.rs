use crate::error::{Result, TokenError};
use crate::simple_serde::SimpleSerde;
use serde_derive::{Deserialize, Serialize};
use solana_sdk::{account_info::AccountInfo, info, pubkey::Pubkey};

#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq)]
pub struct TokenInfo {
    /// Total supply of tokens
    pub supply: u64,

    /// Number of base 10 digits to the right of the decimal place in the total supply
    pub decimals: u8,

    /// Descriptive name of this token
    pub name: [u8; 32],

    /// Symbol for this token
    pub symbol: [u8; 32],
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

impl TokenState {
    pub fn process_newtoken(
        accounts: &mut [AccountInfo],
        token_info: TokenInfo,
        input_accounts: &[TokenState],
        output_accounts: &mut Vec<(usize, TokenState)>,
    ) -> Result<()> {
        if input_accounts.len() != 2 {
            info!("Error: Expected 2 accounts");
            return Err(TokenError::InvalidArgument);
        }

        if let TokenState::Account(dest_account) = &input_accounts[1] {
            if accounts[0].key != &dest_account.token || !accounts[0].is_signer {
                info!("Error: account 1 token mismatch");
                return Err(TokenError::InvalidArgument);
            }

            if dest_account.delegate.is_some() {
                info!("Error: account 1 is a delegate and cannot accept tokens");
                return Err(TokenError::InvalidArgument);
            }

            let mut output_dest_account = dest_account.clone();
            output_dest_account.amount = token_info.supply;
            output_accounts.push((1, TokenState::Account(output_dest_account)));
        } else {
            info!("Error: account 1 invalid");
            return Err(TokenError::InvalidArgument);
        }

        if input_accounts[0] != TokenState::Unallocated {
            info!("Error: account 0 not available");
            return Err(TokenError::InvalidArgument);
        }
        output_accounts.push((0, TokenState::Token(token_info)));
        Ok(())
    }

    pub fn process_newaccount(
        accounts: &mut [AccountInfo],
        input_accounts: &[TokenState],
        output_accounts: &mut Vec<(usize, TokenState)>,
    ) -> Result<()> {
        if input_accounts.len() < 3 {
            info!("Error: Expected 3 accounts");
            return Err(TokenError::InvalidArgument);
        }
        if input_accounts[0] != TokenState::Unallocated {
            info!("Error: account 0 is already allocated");
            return Err(TokenError::InvalidArgument);
        }
        let mut token_account_info = TokenAccountInfo {
            token: *accounts[2].key,
            owner: *accounts[1].key,
            amount: 0,
            delegate: None,
        };
        if input_accounts.len() >= 4 {
            token_account_info.delegate = Some(TokenAccountDelegateInfo {
                source: *accounts[3].key,
                original_amount: 0,
            });
        }
        output_accounts.push((0, TokenState::Account(token_account_info)));
        Ok(())
    }

    pub fn process_transfer(
        accounts: &mut [AccountInfo],
        amount: u64,
        input_accounts: &[TokenState],
        output_accounts: &mut Vec<(usize, TokenState)>,
    ) -> Result<()> {
        if input_accounts.len() < 3 {
            info!("Error: Expected 3 accounts");
            return Err(TokenError::InvalidArgument);
        }

        if let (TokenState::Account(source_account), TokenState::Account(dest_account)) =
            (&input_accounts[1], &input_accounts[2])
        {
            if source_account.token != dest_account.token {
                info!("Error: account 1/2 token mismatch");
                return Err(TokenError::InvalidArgument);
            }

            if dest_account.delegate.is_some() {
                info!("Error: account 2 is a delegate and cannot accept tokens");
                return Err(TokenError::InvalidArgument);
            }

            if accounts[0].key != &source_account.owner || !accounts[0].is_signer {
                info!("Error: owner of account 1 not present");
                return Err(TokenError::InvalidArgument);
            }

            if source_account.amount < amount {
                return Err(TokenError::InsufficentFunds);
            }

            let mut output_source_account = source_account.clone();
            output_source_account.amount -= amount;
            output_accounts.push((1, TokenState::Account(output_source_account)));

            if let Some(ref delegate_info) = source_account.delegate {
                if input_accounts.len() != 4 {
                    info!("Error: Expected 4 accounts");
                    return Err(TokenError::InvalidArgument);
                }

                let delegate_account = source_account;
                if let TokenState::Account(source_account) = &input_accounts[3] {
                    if source_account.token != delegate_account.token {
                        info!("Error: account 1/3 token mismatch");
                        return Err(TokenError::InvalidArgument);
                    }
                    if accounts[3].key != &delegate_info.source {
                        info!("Error: Account 1 is not a delegate of account 3");
                        return Err(TokenError::InvalidArgument);
                    }

                    if source_account.amount < amount {
                        return Err(TokenError::InsufficentFunds);
                    }

                    let mut output_source_account = source_account.clone();
                    output_source_account.amount -= amount;
                    output_accounts.push((3, TokenState::Account(output_source_account)));
                } else {
                    info!("Error: account 3 is an invalid account");
                    return Err(TokenError::InvalidArgument);
                }
            }

            let mut output_dest_account = dest_account.clone();
            output_dest_account.amount += amount;
            output_accounts.push((2, TokenState::Account(output_dest_account)));
        } else {
            info!("Error: account 1 and/or 2 are invalid accounts");
            return Err(TokenError::InvalidArgument);
        }
        Ok(())
    }

    pub fn process_approve(
        accounts: &mut [AccountInfo],
        amount: u64,
        input_accounts: &[TokenState],
        output_accounts: &mut Vec<(usize, TokenState)>,
    ) -> Result<()> {
        if input_accounts.len() != 3 {
            info!("Error: Expected 3 accounts");
            return Err(TokenError::InvalidArgument);
        }

        if let (TokenState::Account(source_account), TokenState::Account(delegate_account)) =
            (&input_accounts[1], &input_accounts[2])
        {
            if source_account.token != delegate_account.token {
                info!("Error: account 1/2 token mismatch");
                return Err(TokenError::InvalidArgument);
            }

            if accounts[0].key != &source_account.owner || !accounts[0].is_signer {
                info!("Error: owner of account 1 not present");
                return Err(TokenError::InvalidArgument);
            }

            if source_account.delegate.is_some() {
                info!("Error: account 1 is a delegate");
                return Err(TokenError::InvalidArgument);
            }

            match &delegate_account.delegate {
                None => {
                    info!("Error: account 2 is not a delegate");
                    return Err(TokenError::InvalidArgument);
                }
                Some(delegate_info) => {
                    if accounts[1].key != &delegate_info.source {
                        info!("Error: account 2 is not a delegate of account 1");
                        return Err(TokenError::InvalidArgument);
                    }

                    let mut output_delegate_account = delegate_account.clone();
                    output_delegate_account.amount = amount;
                    output_delegate_account.delegate = Some(TokenAccountDelegateInfo {
                        source: delegate_info.source,
                        original_amount: amount,
                    });
                    output_accounts.push((2, TokenState::Account(output_delegate_account)));
                }
            }
        } else {
            info!("Error: account 1 and/or 2 are invalid accounts");
            return Err(TokenError::InvalidArgument);
        }
        Ok(())
    }

    pub fn process_setowner(
        accounts: &mut [AccountInfo],
        input_accounts: &[TokenState],
        output_accounts: &mut Vec<(usize, TokenState)>,
    ) -> Result<()> {
        if input_accounts.len() < 3 {
            info!("Error: Expected 3 accounts");
            return Err(TokenError::InvalidArgument);
        }

        if let TokenState::Account(source_account) = &input_accounts[1] {
            if accounts[0].key != &source_account.owner || !accounts[0].is_signer {
                info!("Error: owner of account 1 not present");
                return Err(TokenError::InvalidArgument);
            }

            let mut output_source_account = source_account.clone();
            output_source_account.owner = *accounts[2].key;
            output_accounts.push((1, TokenState::Account(output_source_account)));
        } else {
            info!("Error: account 1 is invalid");
            return Err(TokenError::InvalidArgument);
        }
        Ok(())
    }

    pub fn process(program_id: &Pubkey, accounts: &mut [AccountInfo], input: &[u8]) -> Result<()> {
        let command = TokenInstruction::deserialize(input)?;

        let input_accounts: Vec<TokenState> = accounts
            .iter()
            .map(|account_info| {
                if account_info.owner == program_id {
                    match Self::deserialize(&account_info.data) {
                        Ok(token_state) => token_state,
                        Err(_) => {
                            info!("Error: deserialize failed");
                            TokenState::Invalid
                        }
                    }
                } else {
                    TokenState::Invalid
                }
            })
            .collect();

        let mut output_accounts: Vec<(_, _)> = vec![];

        info!(0, 0, 0, 0, line!());
        match command {
            TokenInstruction::NewToken(token_info) => {
                info!("TokenInstruction: NewToken");
                Self::process_newtoken(accounts, token_info, &input_accounts, &mut output_accounts)?
            }
            TokenInstruction::NewTokenAccount => {
                info!("TokenInstruction: NewTokenAccount");
                Self::process_newaccount(accounts, &input_accounts, &mut output_accounts)?
            }

            TokenInstruction::Transfer(amount) => {
                info!("TokenInstruction: Transfer");
                Self::process_transfer(accounts, amount, &input_accounts, &mut output_accounts)?
            }

            TokenInstruction::Approve(amount) => {
                info!("TokenInstruction: Approve");
                Self::process_approve(accounts, amount, &input_accounts, &mut output_accounts)?
            }

            TokenInstruction::SetOwner => {
                info!("TokenInstruction: SetOwner");
                Self::process_setowner(accounts, &input_accounts, &mut output_accounts)?
            }
        }

        info!(0, 0, 0, 0, line!());
        for (index, account) in &output_accounts {
            Self::serialize(account, &mut accounts[*index].data)?;
            info!(accounts[*index].data.len(), 0, 0, 0, 0);
        }
        info!(0, 0, 0, 0, line!());
        Ok(())
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
            // name: "A test token".to_string(),
            // symbol: "TEST".to_string(),
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
