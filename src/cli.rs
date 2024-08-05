use crate::blockchain::Blockchain;
use crate::common::Result;
use crate::server::Server;
//use crate::txn::;
use crate::blockchain::UTXOSet;
use crate::wallet::Wallets;
use crate::txn::{Transaction};
use crate::CONFIG;
use bitcoincash_addr::Address;
use clap::{arg, Command};
use std::process::exit;

pub struct Cli {}

impl Cli {
    pub fn new() -> Result<Cli> {
        Ok(Cli {})
    }
    pub fn run(&mut self) -> Result<()> {
        let matches = Command::new("simplified-btc-rs")
            .version("0.1")
            .author("luozhixiao1993@gmail.com")
            .about("blockchain in rust: a simple blockchain for learning")
            .subcommand(Command::new("printchain").about("print all the chain blocks"))
            .subcommand(Command::new("createwallet").about("create a wallet"))
            .subcommand(Command::new("listaddresses").about("list all addresses"))
            .subcommand(Command::new("reindex").about("reindex UTXO"))
            .subcommand(
                Command::new("getbalance")
                    .about("get balance in the blochain")
                    .arg(arg!(<ADDRESS>"'The Address it get balance for'")),
            )
            .subcommand(
                Command::new("startnode")
                    .about("start the node server")
                    .arg(arg!(<PORT>"'the port server bind to locally'")),
            )
            .subcommand(
                Command::new("create")
                    .about("Create new blochain")
                    .arg(arg!(<ADDRESS>"'The address to send gensis block reqward to' ")),
            )
            .subcommand(
                Command::new("send")
                    .about("send  in the blockchain")
                    .arg(arg!(<FROM>" 'Source wallet address'"))
                    .arg(arg!(<TO>" 'Destination wallet address'"))
                    .arg(arg!(<AMOUNT>" 'Destination wallet address'"))
                    .arg(arg!(-m --mine " 'the from address mine immediately'")),
            )
            .subcommand(
                Command::new("startminer")
                    .about("start the minner server")
                    .arg(arg!(<PORT>" 'the port server bind to locally'"))
                    .arg(arg!(<ADDRESS>" 'wallet address'")),
            )
            .get_matches();

        Ok(())
    }
}
