use crate::state::TokenState;
use solana_sdk::{
    account_info::AccountInfo, entrypoint, entrypoint::SUCCESS,
    pubkey::Pubkey,
};

entrypoint!(process_instruction);
fn process_instruction(program_id: &Pubkey, accounts: &mut [AccountInfo], input: &[u8]) -> u32 {
    const FAILURE: u32 = 1;

    match TokenState::process(program_id, accounts, input) {
        Ok(_) => SUCCESS,
        Err(e) => {
            e.print();
            FAILURE
        }
    }
}
