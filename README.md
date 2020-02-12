[![Build status][travis-image]][travis-url]

[travis-image]: https://api.travis-ci.org/solana-labs/example-token.svg?branch=master
[travis-url]: https://travis-ci.org/solana-labs/example-token

# Token Example on Solana

This project demonstrates how to use the [Solana Javascript API](https://github.com/solana-labs/solana-web3.js)
to build, deploy, and interact with an ERC20-like Token example program on the Solana blockchain.

The project comprises of:

* A library to interact with the on-chain program
* Test client that exercises the program

## Getting Started

First fetch the npm dependencies, including `@solana/web3.js`, by running:
```sh
$ npm install
```

### Select a Network

This example connects to a local Solana cluster by default.

To enable on-chain program logs, set the `RUST_LOG` environment variable:

```bash
$ export RUST_LOG=solana_runtime::native_loader=trace,solana_runtime::system_instruction_processor=trace,solana_runtime::bank=debug,solana_bpf_loader=debug,solana_rbpf=debug
```

To start a local Solana cluster run:
```bash
$ npm run localnet:update
$ npm run localnet:up
```

Solana cluster logs are available with:
```bash
$ npm run localnet:logs
```

For more details on working with a local cluster, see the [full instructions](https://github.com/solana-labs/solana-web3.js#local-network).

By default the program will connect to the
beta testnet.  To use the edge testnet instead, define `export CHANNEL=edge' in
your environment (see [url.js](https://github.com/solana-labs/solana/tree/master/urj.js) for more)

### Run the test client

```sh
$ npm run start
```

## Customizing the Program

To customize the example, make changes to the files under `/src`

Now when you run `npm run start`, you should see the results of your changes.
