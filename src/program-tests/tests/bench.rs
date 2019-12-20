use solana_bpf_loader_program::{create_vm, deserialize_parameters, serialize_parameters};
use solana_bpf_token::state::{Command, State, Token};
use solana_sdk::{
    account::{Account, KeyedAccount},
    pubkey::Pubkey,
};
use std::{fs::File, io::Error, io::Read, mem::size_of, path::PathBuf};

const BASELINE_NEWTOKENACCOUNT_COUNT: u64 = 814;
const BASELINE_NEWTOKEN_COUNT: u64 = 955;
const BASELINE_TRANSFER_COUNT: u64 = 1656;

const PLATFORM_FILE_EXTENSION_BPF: &str = "so";

fn load_program(name: &str) -> Vec<u8> {
    let mut path = PathBuf::new();
    path.push("../program/target/bpfel-unknown-unknown/release");
    path.push(name);
    path.set_extension(PLATFORM_FILE_EXTENSION_BPF);

    let mut file = File::open(path).unwrap();
    let mut program = Vec::new();
    file.read_to_end(&mut program).unwrap();
    program
}

fn run_program(
    program_id: &Pubkey,
    parameter_accounts: &mut [KeyedAccount],
    instruction_data: &[u8],
) -> Result<(u64, u64), Error> {
    let mut program_account = Account::default();
    program_account.data = load_program("solana_bpf_token");
    let (mut vm, heap_region) = create_vm(&program_account.data).unwrap();

    let mut parameter_bytes =
        serialize_parameters(program_id, parameter_accounts, &instruction_data);
    let result = vm
        .execute_program(parameter_bytes.as_mut_slice(), &[], &[heap_region.clone()])
        .unwrap();
    deserialize_parameters(parameter_accounts, &parameter_bytes);
    let instruction_count = vm.get_last_instruction_count();
    Ok((result, instruction_count))
}

#[test]
fn bench_transfer() {
    solana_logger::setup();

    let program_id = Pubkey::new_rand();
    let mut instruction_data = vec![0u8; size_of::<Command>()];
    let mint_key = Pubkey::new_rand();
    let mut mint_account = Account::new(0, size_of::<State>(), &program_id);
    let owner_key = Pubkey::new_rand();
    let mut owner_account = Account::default();
    let token_key = Pubkey::new_rand();
    let mut token_account = Account::new(0, size_of::<State>(), &program_id);

    // Create mint account
    let instruction = Command::NewTokenAccount;
    instruction.serialize(&mut instruction_data).unwrap();
    let mut parameter_accounts = vec![
        KeyedAccount::new(&mint_key, true, &mut mint_account),
        KeyedAccount::new(&owner_key, false, &mut owner_account),
        KeyedAccount::new(&token_key, false, &mut token_account),
    ];
    let (result, newtokenaccount_count) =
        run_program(&program_id, &mut parameter_accounts[..], &instruction_data).unwrap();
    assert!(result == 0);

    // Create new account
    let instruction = Command::NewTokenAccount;
    instruction.serialize(&mut instruction_data).unwrap();
    let payee_key = Pubkey::new_rand();
    let mut payee_account = Account::new(0, size_of::<State>(), &program_id);
    let mut parameter_accounts = vec![
        KeyedAccount::new(&payee_key, true, &mut payee_account),
        KeyedAccount::new(&owner_key, false, &mut owner_account),
        KeyedAccount::new(&token_key, false, &mut token_account),
    ];
    let (result, _) =
        run_program(&program_id, &mut parameter_accounts[..], &instruction_data).unwrap();
    assert!(result == 0);

    // Create new token
    let instruction = Command::NewToken(Token {
        supply: 1000,
        decimals: 2,
    });
    instruction.serialize(&mut instruction_data).unwrap();
    let mut parameter_accounts = vec![
        KeyedAccount::new(&token_key, true, &mut token_account),
        KeyedAccount::new(&mint_key, false, &mut mint_account),
    ];
    let (result, newtoken_count) =
        run_program(&program_id, &mut parameter_accounts[..], &instruction_data).unwrap();
    assert!(result == 0);

    // Transfer
    let instruction = Command::Transfer(100);
    instruction.serialize(&mut instruction_data).unwrap();
    let mut parameter_accounts = vec![
        KeyedAccount::new(&owner_key, true, &mut owner_account),
        KeyedAccount::new(&mint_key, false, &mut mint_account),
        KeyedAccount::new(&payee_key, false, &mut payee_account),
    ];
    let (result, transfer_count) =
        run_program(&program_id, &mut parameter_accounts[..], &instruction_data).unwrap();
    assert!(result == 0);

    println!("BPF instructions executed");
    println!(
        "  NewTokenAccount: {:?} ({:?})",
        newtokenaccount_count, BASELINE_NEWTOKENACCOUNT_COUNT
    );
    println!(
        "  NewToken       : {:?} ({:?})",
        newtoken_count, BASELINE_NEWTOKEN_COUNT
    );
    println!(
        "  Transfer       : {:?} ({:?})",
        transfer_count, BASELINE_TRANSFER_COUNT
    );

    assert!(newtokenaccount_count <= BASELINE_NEWTOKENACCOUNT_COUNT);
    assert!(newtoken_count <= BASELINE_NEWTOKEN_COUNT);
    assert!(transfer_count <= BASELINE_TRANSFER_COUNT);
}
