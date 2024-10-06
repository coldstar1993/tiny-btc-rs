use crate::blockchain::Blockchain;
use crate::common::Result;
use crate::server::Server;
//use crate::txn::;
use crate::blockchain::UTXOSet;
use crate::txn::Transaction;
use crate::wallet::Wallets;
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

            
        if let Some(_) = matches.subcommand_matches("printchain") {
            cmd_print_chain()?;
        }

        if let Some(_) = matches.subcommand_matches("createwallet") {
            println!("address: {}", cmd_create_wallet()?);
        }

        if let Some(_) = matches.subcommand_matches("listaddresses") {
            cmd_list_address()?;
        }

        if let Some(ref matches) = matches.subcommand_matches("startminer") {
            let port = if let Some(port) = matches.get_one::<String>("PORT") {
                port
            } else {
                println!("PORT not supply!: usage");
                exit(1)
            };

            let address = if let Some(address) = matches.get_one::<String>("ADDRESS") {
                address
            } else {
                println!("ADDRESS not supply!: usage");
                exit(1)
            };

            {
                CONFIG.write().unwrap().port = port.parse().unwrap();
            }

            let bc = Blockchain::new()?;
            let utxo_set = UTXOSet { blockchain: bc };
            let server = Server::new(port, address, utxo_set)?;
            server.start_server()?;
        }

        Ok(())
    }
}

fn cmd_create_wallet() -> Result<String> {
    let mut ws = Wallets::new()?;
    let address = ws.create_wallet();
    ws.save_all()?;
    Ok(address)
}

fn cmd_print_chain() -> Result<()> {
    let bc = Blockchain::new()?;
    for b in bc.iter() {
        println!("{:#?}", b);
    }
    Ok(())
}

fn cmd_list_address() -> Result<()> {
    let ws = Wallets::new()?;
    let addresses = ws.get_all_addresses();
    println!("addresses: ");
    for ad in addresses {
        println!("{}", ad);
    }
    Ok(())
}
