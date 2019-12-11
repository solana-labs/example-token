// use solana_runtime::bank::Bank;
// use solana_runtime::bank_client::BankClient;
// use solana_runtime::genesis_utils::{create_genesis_config, GenesisConfigInfo};
// use solana_runtime::loader_utils::create_invoke_instruction;
// use solana_runtime::loader_utils::load_program;
// use solana_sdk::bpf_loader;
// use solana_sdk::client::SyncClient;
// use solana_sdk::signature::KeypairUtil;
// use std::env;
// use std::fs::File;
// use std::io::Read;
// use std::path::PathBuf;

// // Pulls in the stubs required for `info!()`
// solana_sdk_bpf_test::stubs!();

// /// BPF program file extension
// const PLATFORM_FILE_EXTENSION_BPF: &str = "so";

// /// Create a BPF program file name
// fn create_bpf_path(name: &str) -> PathBuf {
//     let mut pathbuf = {
//         let current_exe = env::current_exe().unwrap();
//         PathBuf::from(current_exe.parent().unwrap().parent().unwrap())
//     };
//     pathbuf.push("bpf/");
//     pathbuf.push(name);
//     pathbuf.set_extension(PLATFORM_FILE_EXTENSION_BPF);
//     pathbuf
// }

// Pulls in the stubs required for `info!()`
solana_sdk_bpf_test::stubs!();

#[test]
pub fn bench_transfer() {
    println!("integration test");
}
