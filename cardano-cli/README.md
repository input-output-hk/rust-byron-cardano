# cardano-cli

This CLI super-seed deprecates [wallet-cli](../wallet-cli).


# Commands

## Quick start

```sh
$ cardano-cli blockchain new mainnet
$ cardano-cli blockchain pull mainnet
$ cardano-cli wallet create "My Wallet"
$ cardano-cli wallet attach "My Wallet" mainnet
$ cardano-cli wallet sync "My Wallet" mainnet
```

## Global flags

* `    --quiet`      run the command quietly, do not print anything to the command line output
* `-h, --help`       Prints help information
* `-V, --version`    Prints version information

## Global options

* `--color <COLOR>`       enable output colors or not [default: auto]  [possible values: auto, always, never]
* `--root-dir <ROOT_DIR>` the project root direction [env: CARDANO_CLI_ROOT_DIR=]  [default: _os specific_]

## `blockchain`: blockchain related operations

### `new`: creating a new local blockchain

```
cardano-cli blockchain new [FLAGS] [OPTIONS] <BLOCKCHAIN_NAME>
```

* FLAGS:
* OPTIONS:
    * `--template <TEMPLATE>`: the template for the new blockchain ; default: `mainnet`.
      Possible values: `mainnet`, `testnet`.
* ARGS:
    * `<BLOCKCHAIN_NAME>`: the blockchain name.

This command creates a local blockchain. A new blockchain is configured with
IOHK's gateways as peer remote nodes by default. You will need to synced the
blocks from time to time to get the latest details about the blockchain.

### `remote-add`: Adding new peer nodes

```
cardano-cli blockchain remote-add [FLAGS] <BLOCKCHAIN_NAME> <BLOCKCHAIN_REMOTE_ALIAS> <BLOCKCHAIN_REMOTE_ENDPOINT>
```

* ARGS:
    * `<BLOCKCHAIN_NAME>`               the blockchain name
    * `<BLOCKCHAIN_REMOTE_ALIAS>`       Alias given to a remote node.
    * `<BLOCKCHAIN_REMOTE_ENDPOINT>`    Remote end point (IPv4 or IPv6 address or domain name. May include a port
                                        number. And a sub-route point in case of an http endpoint.

Add remote node, a peer, to the local blockchain. It will be used by the `fetch`
command to download blocks.

### `remote-rm`: Removing a remote node

```
cardano-cli blockchain remote-rm <BLOCKCHAIN_NAME> <BLOCKCHAIN_REMOTE_ALIAS>
```

* ARGS:
    * `<BLOCKCHAIN_NAME>`            the blockchain name
    * `<BLOCKCHAIN_REMOTE_ALIAS>`    Alias given to a remote node.

Remove the remote node, peer, details from the local blockchain. It won't be
contacted again by the `fetch` command to download blocks.

### `remote-fetch`: Fetching remote nodes’ blockchain

```
cardano-cli blockchain remote-fetch [FLAGS] <BLOCKCHAIN_NAME> <BLOCKCHAIN_REMOTE_ALIAS>...
```

* ARGS:
    * `<BLOCKCHAIN_NAME>`               the blockchain name
    * `<BLOCKCHAIN_REMOTE_ALIAS>...`    Alias given to a remote node.

This function downloads the blocks of the specified remote source names (by
default, it will download blocks from every remote nodes).

### `forward`: Update blockchain local tip

```
cardano-cli blockchain forward [FLAGS] <BLOCKCHAIN_NAME> [HASH]
```

* ARGS:
    * `<BLOCKCHAIN_NAME>`    the blockchain name
    * `<HASH>`               Set the new local tip to the given blockhash, do not try to figure out consensus between
                         the remote nodes.

This function will _forward_ the local blockchain tip to what seems to be the
consensus between the remote nodes (based on what has been synced (see
`remote-fetch`)).

### `pull`: `remote-fetch` + `forward`

```
cardano-cli blockchain pull <BLOCKCHAIN_NAME>
```

* ARGS:
    * `<BLOCKCHAIN_NAME>`    the blockchain name

This command is equivalent to:

```bash
cardano-cli blockchain remote-fetch ${BLOCKCHAIN_NAME} && \
cardano-cli blockchain forward      ${BLOCKCHAIN_NAME}
```

### `gc`: Garbage collecting the local blockchain's lose blocks

```
cardano-cli blockchain gc <BLOCKCHAIN_NAME>
```

* ARGS:
    * `<BLOCKCHAIN_NAME>`    the blockchain name

This command will delete lose blocks. These are blocks that have been
discarded by the blockchain.

### `cat`: Pretty print content of a block

```
cardano-cli blockchain cat <BLOCKCHAIN_NAME> <HASH>
```

* ARGS:
    * `<BLOCKCHAIN_NAME>`    the blockchain name
    * `<HASH>`               The block hash to open.

### `status`: Blockchain summary

```
cardano-cli blockchain status <BLOCKCHAIN_NAME>
```

* ARGS:
    * `<BLOCKCHAIN_NAME>`    the blockchain name

This command will print some status information regarding the local
blockchain.

### `log`: Blockchain blocks

```
cardano-cli blockchain log <BLOCKCHAIN_NAME> [HASH]
```

* ARGS:
    * `<BLOCKCHAIN_NAME>`    the blockchain name
    * `<HASH>`               The hash to start from (instead of the local blockchain's tip).

Prints the blockchain logs from the tip (or the specified hash, in reverse order).

## Wallet

### `create`: Wallet cryptographic creation:

```
cardano-cli wallet create <WALLET_NAME>
```

* OPTIONS:
    * `--color <COLOR>` enable output colors or not [default: auto]  [possible values: auto, always, never]
    * `--derivation-scheme <DERIVATION_SCHEME>`     derivation scheme [default: v2]  [possible values: v1, v2]
    * `--mnemonics-language <MNEMONIC_LANGUAGE>`
            the list of languages to display the mnemonic words of the wallet in. You can set multiple values using
            comma delimiter (example: `--mnemonics-languages=english,french,italian'). [default: english]  [aliases:
            mnemonics-languages]  [possible values: chinese-simplified, chinese-traditional, english, french, italian,
            japanese, korean, spanish]
    * `--mnemonics-length <MNEMONIC_SIZE>`
            The number of words to use for the wallet mnemonic (the more the more secure). [default: 24]  [possible
            values: 12, 15, 18, 21, 24]
    * `--wallet-scheme <WALLET_SCHEME>`
            the scheme to organize accounts and addresses in a Wallet. [default: bip44]  [possible values: bip44,
            random_index_2levels]

* ARGS:
    * `<WALLET_NAME>`    the wallet name

Create a new wallet. This wallet not attached to any blockchain. This command
requires user interaction for setting mnemonic passwords, recording the
wallet's mnemonics ...

### `destroy`: Deleting a Wallet

```
cardano-cli wallet destroy <WALLET_NAME>
```

* ARGS:
    * `<WALLET_NAME>`    the wallet name

Require user's confirmation, will delete all data associated
to the given wallet.

### `attach`: Attach the wallet to a blockchain:

```
cardano-cli wallet attach <WALLET_NAME> <BLOCKCHAIN_NAME>
```

* ARGS:
    * `<WALLET_NAME>`        the wallet name
    * `<BLOCKCHAIN_NAME>`    the blockchain name

Set wallet state tag to the genesis of this blockchain.

### `detach`: Detach the wallet from the blockchain

```
cardano-cli wallet attach <WALLET_NAME>
```

Clear wallet's state associated to this blockchain (the wallet log).

### `sync`: Updating wallet state

```
cardano-cli wallet sync [OPTIONS] <WALLET_NAME>
```

* FLAGS:
    * `--dry-run`    perform the sync without storing the updated states.
    * `--quiet`      run the command quietly, do not print anything to the command line output
* OPTIONS:
    `--to <HASH>`    sync the wallet up to the given hash (otherwise, sync up to local blockchain's tip).
* ARGS:
    * `<WALLET_NAME>`    the wallet name

Require a wallet that is attached to a blockchain, the state of the wallet
is updated against the block of the blockchain, collecting UTxOs, spent
addresses...

### `status`: Wallet's accounts, transactions history

Print the wallet summary:

```
cardano-cli wallet status <WALLET_NAME>
```

## Debug

```
cardano-cli debug address <ADDRESS>
```

```
cardano-cli debug log-dump <FILE>
```
