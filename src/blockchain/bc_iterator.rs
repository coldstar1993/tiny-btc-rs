//! Blockchain

use super::*;
use crate::block::{block::*};
use crate::txn::*;
use bincode::{deserialize, serialize};
use failure::format_err;
use log::{debug, info};
use sled;
use std::collections::HashMap;

/// BlockchainIterator is used to iterate over blockchain blocks
pub struct BlockchainIterator<'a> {
    pub current_hash: String,
    pub bc: &'a Blockchain,
}
impl<'a> Iterator for BlockchainIterator<'a> {
    type Item = Block;

    fn next(&mut self) -> Option<Self::Item> {
        if let Ok(encoded_block) = self.bc.db.get(&self.current_hash) {
            return match encoded_block {
                Some(b) => {
                    if let Ok(block) = deserialize::<Block>(&b) {
                        self.current_hash = block.get_prev_hash();
                        Some(block)
                    } else {
                        None
                    }
                }
                None => None,
            };
        }
        None
    }
}
