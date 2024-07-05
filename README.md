# simplified btc in rust

a POW+UTXO blockchain demo, just for learning 
* align with btc's address derivation & signature algorithms
* only support P2PKH now

concurrency by multi-threading, while node communication by TCP protocol

## steps

- Create wallet:
  ```sh
  cargo run createwallet
  ```
- Create blockchain(*address* will get reward from coinbase tx of genesis block):
  ```
  cargo run create <address>
  ```
- send coins (if `-m` is specified, the block will be mined immediately in the same node):
  ```
  cargo run send <from> <to> <amount> -m 
  ```

