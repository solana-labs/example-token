// @flow

import {Account, Connection} from '@solana/web3.js';

export async function newAccountWithLamports(
  connection: Connection,
  lamports: number = 1000000,
): Promise<Account> {
  const account = new Account();

  await connection.requestAirdrop(account.publicKey, lamports);
  return account;
}
