//! transaction implement

use crate::blockchain::UTXOSet;
use crate::common::Result;
use crate::txn::tx_in_out::{TXInput, TXOutput};
use crate::wallet::{hash_pub_key, Wallet};
use bincode::serialize;
use bitcoincash_addr::Address;
use crypto::digest::Digest;
use crypto::ed25519;
use crypto::sha2::Sha256;
use failure::format_err;
use log::{debug, error, info};
use rand::rngs::OsRng;
use rand::{Rng, RngCore};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

const SUBSIDY: i32 = 10;

/// Transaction represents a Bitcoin transaction
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Transaction {
    pub id: String,
    pub vin: Vec<TXInput>,
    pub vout: Vec<TXOutput>,
}

impl Transaction {
        /// NewUTXOTransaction creates a new transaction
    pub fn new_UTXO(wallet: &Wallet, to: &str, amount: i32, utxo: &UTXOSet) -> Result<Transaction> {
        info!(
            "new UTXO Transaction from: {} to: {}",
            wallet.get_address(),
            to
        );
        let mut vin = Vec::new();

        let mut pub_key_hash = wallet.public_key.clone();
        hash_pub_key(&mut pub_key_hash);

        let acc_v = utxo.find_spendable_outputs(&pub_key_hash, amount)?;

        if acc_v.0 < amount {
            error!("Not Enough balance");
            return Err(format_err!(
                "Not Enough balance: current balance {}",
                acc_v.0
            ));
        }

        for tx in acc_v.1 {
            for out in tx.1 {
                let input = TXInput {
                    txid: tx.0.clone(),
                    vout: out,
                    signature: Vec::new(),
                    pub_key: wallet.public_key.clone(),
                };
                vin.push(input);
            }
        }

        let mut vout = vec![TXOutput::new(amount, to.to_string())?];
        if acc_v.0 > amount {
            vout.push(TXOutput::new(acc_v.0 - amount, wallet.get_address())?) // give change(找零)
        }

        let mut tx = Transaction {
            id: String::new(),
            vin,
            vout,
        };
        tx.id = tx.hash()?;
        utxo.blockchain
            .sign_transacton(&mut tx, &wallet.secret_key)?;
        Ok(tx)
    }

    /// NewCoinbaseTX creates a new coinbase transaction
    pub fn new_coinbase(to: String, mut data: String) -> Result<Transaction> {
        info!("new coinbase Transaction to: {}", to);
        let mut key: [u8; 32] = [0; 32];
        if data.is_empty() {
            let mut rand = OsRng::default();
            rand.fill_bytes(&mut key);
            data = format!("Reward to '{}'", to);
        }
        let mut pub_key = Vec::from(data.as_bytes());
        pub_key.append(&mut Vec::from(key));

        let mut tx = Transaction {
            id: String::new(),
            vin: vec![TXInput {
                txid: String::new(),
                vout: -1,
                signature: Vec::new(),
                pub_key,
            }],
            vout: vec![TXOutput::new(SUBSIDY, to)?],
        };
        tx.id = tx.hash()?;
        Ok(tx)
    }

    /// IsCoinbase checks whether the transaction is coinbase
    pub fn is_coinbase(&self) -> bool {
        self.vin.len() == 1 && self.vin[0].txid.is_empty() && self.vin[0].vout == -1
    }

    /// Hash returns the hash of the Transaction
    pub fn hash(&self) -> Result<String> {
        let mut copy = self.clone();
        copy.id = String::new();
        let data = serialize(&copy)?;
        let mut hasher = Sha256::new();
        hasher.input(&data[..]);
        Ok(hasher.result_str())
    }

    /// TrimmedCopy creates a trimmed copy of Transaction to be used in signing
    fn trim_copy(&self) -> Transaction {
        let mut vin = Vec::new();
        let mut vout = Vec::new();

        for v in &self.vin {
            vin.push(TXInput {
                txid: v.txid.clone(),
                vout: v.vout.clone(),
                signature: Vec::new(),
                pub_key: Vec::new(),
            })
        }

        for v in &self.vout {
            vout.push(TXOutput {
                value: v.value,
                pub_key_hash: v.pub_key_hash.clone(),
            })
        }

        Transaction {
            id: self.id.clone(),
            vin,
            vout,
        }
    }

    /// Sign signs each input of a Transaction
    pub fn sign(
        &mut self,
        private_key: &[u8],
        prev_TXs: HashMap<String, Transaction>,
    ) -> Result<()> {
        if self.is_coinbase() {
            return Ok(());
        }

        for vin in &self.vin {
            if prev_TXs.get(&vin.txid).unwrap().id.is_empty() {
                return Err(format_err!("ERROR: Previous transaction is not correct"));
            }
        }

        let mut tx_copy = self.trim_copy();

        for in_id in 0..tx_copy.vin.len() {
            let prev_Tx = prev_TXs.get(&tx_copy.vin[in_id].txid).unwrap();
            tx_copy.vin[in_id].signature.clear();
            tx_copy.vin[in_id].pub_key = prev_Tx.vout[tx_copy.vin[in_id].vout as usize]
                .pub_key_hash
                .clone();
            tx_copy.id = tx_copy.hash()?;
            tx_copy.vin[in_id].pub_key = Vec::new();
            let signature = ed25519::signature(tx_copy.id.as_bytes(), private_key);
            self.vin[in_id].signature = signature.to_vec();
        }

        Ok(())
    }

    /// Verify verifies signatures of Transaction inputs
    pub fn verify(&self, prev_TXs: HashMap<String, Transaction>) -> Result<bool> {
        if self.is_coinbase() {
            return Ok(true);
        }

        for vin in &self.vin {
            if prev_TXs.get(&vin.txid).unwrap().id.is_empty() {
                return Err(format_err!("ERROR: Previous transaction is not correct"));
            }
        }

        let mut tx_copy = self.trim_copy();

        for in_id in 0..self.vin.len() {
            let prev_Tx = prev_TXs.get(&self.vin[in_id].txid).unwrap();
            tx_copy.vin[in_id].signature.clear();
            tx_copy.vin[in_id].pub_key = prev_Tx.vout[self.vin[in_id].vout as usize]
                .pub_key_hash
                .clone();
            tx_copy.id = tx_copy.hash()?;
            tx_copy.vin[in_id].pub_key = Vec::new();

            if !ed25519::verify(
                &tx_copy.id.as_bytes(),
                &self.vin[in_id].pub_key,
                &self.vin[in_id].signature,
            ) {
                return Ok(false);
            }
        }

        Ok(true)
    }
}

#[cfg(test)]
mod test {
    use crate::wallet::Wallets;

    use super::*;

    #[test]
    fn test_signature() {
        let mut ws = Wallets::new().unwrap();
        let wa1 = ws.create_wallet();
        let w = ws.get_wallet(&wa1).unwrap().clone();
        ws.save_all().unwrap();
        drop(ws);

        let data = String::from("test");
        let tx = Transaction::new_coinbase(wa1, data).unwrap();
        assert!(tx.is_coinbase());

        let signature = ed25519::signature(tx.id.as_bytes(), &w.secret_key);
        assert!(ed25519::verify(tx.id.as_bytes(), &w.public_key, &signature));
    }
}
