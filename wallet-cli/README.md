>                             ***STILL IN DEVELOPMENT***

# Ariadne summary

`ariadne` is a command line interface to perform actions on a `Cardano`
compatible wallet.

`ariadne` provides the following features:

- [x] create a new address;
- [x] retrieve an address from BIP39 mnemonic words;
- [x] BIP44 addresses (and shorter addresses);
- [x] see transaction;
- [ ] retrieve wallet balance;

# Usage

Make a wallet and retrieve its addresses.

```sh-session
$ ariadne wallet generate
# creates ~/.ariadne/wallet.yml

$ ariadne wallet address "Main" 0 1 2
jYTLseJK1m2GQZeYxYKkiea3Phfqt9TUEUCjTDezYSQbd6sY5VaNFr3SKRhD
jYTLseJK1m2RmsXD44EZVe2EqGtBnQyFnPwjS2Q36c3iLwWCPGmAVWKR9ufE
jYTLseJK1m2eYuy16w7tjPNqtVdAaQ5bnb4Cyya2YH9AqJoNPQ1VLgE4MaXL
```

Make a blockchain network configuration (from a template).

```sh-session
$ ariadne blockchain new foo --template testnet
# creates a few files under ~/.ariadne/networks/foo, using the testnet template
# see `ariadne network new --help` for more info.
```

Download a blockchain specified by network configuration name (takes awhile).

```sh-session
$ ariadne blockchain sync foo
HANDSHAKE OK
Configured genesis   : b36..
Configured genesis-1 : c6a..
Network TIP is       : fbb..
Network TIP slotid   : 47.14233
latest known epoch 0 hash=Some(HeaderHash(b36...))
downloading epoch 0 b36...
...
 
```

Show a block from a given blockchain:

```sh-sesssion
$ ariadne blockchain cat foo "c9942ae..."
# display the content of the block of hash "c9942ae.." from the blockchain foo
### Header
protocol magic : 633343913
previous hash  : 59ccabad5a13660842c3633a99facfe4458e689acc5f830f32a3fde030b8c4e7
body proof
...
```
