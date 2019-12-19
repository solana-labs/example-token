use crate::state::State;
use solana_sdk::{account_info::AccountInfo, entrypoint, entrypoint::SUCCESS, pubkey::Pubkey};

entrypoint!(process_instruction);
fn process_instruction<'a>(
    program_id: &Pubkey,
    accounts: &'a mut [AccountInfo<'a>],
    input: &[u8],
) -> u32 {
    const FAILURE: u32 = 1;

    match State::process(program_id, accounts, input) {
        Ok(_) => SUCCESS,
        Err(e) => {
            e.print();
            FAILURE
        }
    }
}
