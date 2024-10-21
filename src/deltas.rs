use serde::{Serialize, Deserialize};
use std::error::Error;
use std::io::Write;
use crate::prices;
use crate::symbols;
use chrono::TimeZone;
use std::collections::{HashMap, HashSet};


#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Deltas ( pub Vec<Delta> );

impl Deltas {

    pub fn load(path: &str) -> Result<Self, Box<dyn Error>> {
        let data = std::fs::read_to_string(path)?;
        let deltas: Vec<Delta> = serde_json::from_str(&data)?;
        Ok(Self( deltas ))
    }

    pub fn save (&self, path: &str) -> Result<(), Box<dyn Error>> {
        let json_string = serde_json::to_string(&self)?;
        std::fs::write(path, &json_string)?;
        // let mut opts_file = std::fs::OpenOptions::new()
        //     .truncate(true)
        //     .write(true)
        //     .open(path)?;
        // opts_file.write(&json_string.as_bytes()).unwrap();
        Ok(())
    }


    pub fn swap_to_swap_gas(&self) -> usize {
        let mut swap = 0;
        let mut swap_gas = 0;
        for delta in &self.0 {
            match delta.ilk {
                Ilk::Swap => swap += 1,
                Ilk::SwapGas => swap_gas += 1,
                _ => {},
            }
        }
        let ratio = swap/swap_gas;
        // println!("swap: {}, swap_gas: {}, ratio: {}", swap,  swap_gas, ratio);
        ratio
    }

    pub fn link_dydx_deposits_and_withdraws (&mut self) {
        let mut deposit_address_and_index = Vec::new();

        let mut links = 0;
        for index in 0..self.0.len() {
            if self.0[index].ilk == Ilk::DydxDeposit {
                // println!("deposit, account: {}", self.0[index].account);
                deposit_address_and_index.push((self.0[index].account.clone(), index))
            } else if self.0[index].ilk == Ilk::DydxWithdraw {
                // println!("wihdraw, account: {}", self.0[index].account);
                let mut deposit_index_option = None;
                for tup_index in 0..deposit_address_and_index.len() {
                    if deposit_address_and_index[tup_index].0 == self.0[index].account {
                        deposit_index_option = Some(deposit_address_and_index[tup_index].1);
                        deposit_address_and_index.remove(tup_index);
                        break
                    }
                }
                let deposit_index = deposit_index_option.unwrap();

                self.0[deposit_index].linked_to.push(index);
                links += 1; 
                self.0[index].linked_to.push(deposit_index);
                links += 1; 
            }
        }
        assert!(deposit_address_and_index.len() == 0);
        println!("dydx deposit-withdras links added: {}",links);
    }

    pub fn link_conversion_components (&mut self) {

        let mut links = 0;
        for index in 0..self.0.len() {

            if self.0[index].ilk == Ilk::CoinbaseConversion && self.0[index].direction == Direction::In {

                // for d in &self.0 {
                //     if d.identifier == self.0[index].identifier {
                //         dbg!(d);
                //     }
                // }
                // println!("");

                let mut steps = 0_usize;
                let mut above = false;
                let mut disposition_linked = false;
                while !disposition_linked {
                    if above {
                        above = false;
                    } else {
                        steps += 1;
                        above = true;
                    }

                    let other_index = if above {
                        std::cmp::min(self.0.len() - 1, index + steps)
                    } else {
                        if steps < index {
                            index - steps 
                        } else {
                            0
                        }
                    };
                    if !disposition_linked {
                        if self.0[other_index].ilk == Ilk::CoinbaseConversion && self.0[other_index].identifier == self.0[index].identifier {
                            // assert!(self.0[other_index].timestamp == self.0[index].timestamp);
                            assert!(self.0[other_index].direction == Direction::Out);
                            assert!(self.0[other_index].linked_to.len() == 0);
                            self.0[other_index].linked_to.push(index);
                            links += 1; 
                            self.0[index].linked_to.push(other_index);
                            links += 1; 
                            disposition_linked = true;
                        }
                    }
                    
                };
            } else if self.0[index].ilk == Ilk::AutomaticConversion && self.0[index].direction == Direction::In {

                let mut steps = 0_usize;
                let mut above = false;
                let mut disposition_linked = false;
                while !disposition_linked {
                    if above {
                        above = false;
                    } else {
                        steps += 1;
                        above = true;
                    }

                    let other_index = if above {
                        std::cmp::min(self.0.len() - 1, index + steps)
                    } else {
                        if steps < index {
                            index - steps 
                        } else {
                            0
                        }
                    };
                    if !disposition_linked {
                        if self.0[other_index].ilk == Ilk::AutomaticConversion && self.0[other_index].identifier == self.0[index].identifier {
                            assert!(self.0[other_index].timestamp == self.0[index].timestamp);
                            assert!(self.0[other_index].direction == Direction::Out);
                            assert!(self.0[other_index].linked_to.len() == 0);
                            self.0[other_index].linked_to.push(index);
                            links += 1; 
                            self.0[index].linked_to.push(other_index);
                            links += 1; 
                            disposition_linked = true;
                        }
                    }
                    
                };

            }
        }
        println!("conversion links_added: {}",links);

    }

    pub fn link_airdrop_components (&mut self) {

        let mut links = 0;
        for index in 0..self.0.len() {

            if self.0[index].ilk == Ilk::Airdrop && !self.0[index].host.is_custodial_exchange() {

                assert!(self.0[index].direction == Direction::In);

                let mut steps = 0_usize;
                let mut above = false;
                let mut gas_fee_linked = false;
                let mut timestamp_misses = 0;
                while !gas_fee_linked {
                    if above {
                        above = false;
                    } else {
                        steps += 1;
                        above = true;
                    }

                    let other_index = if above {
                        std::cmp::min(self.0.len() - 1, index + steps)
                    } else {
                        if steps < index {
                            index - steps 
                        } else {
                            0
                        }
                    };
                    if !gas_fee_linked {

                        if self.0[other_index].ilk == Ilk::AirdropClaimGas && self.0[other_index].identifier == self.0[index].identifier {
                            assert!(self.0[other_index].direction == Direction::Out);
                            assert!(self.0[other_index].linked_to.len() == 0);
                            self.0[other_index].linked_to.push(index);
                            links += 1; 
                            self.0[index].linked_to.push(other_index);
                            links += 1; 
                            gas_fee_linked = true;
                        }
                    }
                    if self.0[other_index].timestamp != self.0[index].timestamp {
                        timestamp_misses += 1;
                        if timestamp_misses >= 2 {
                            break
                        }
                    }
                };
            }
        }
        println!("airdrop links added: {}",links);

    }

    pub fn link_miner_direct_payment(&mut self) {

        let mut links = 0;
        let mut waved = 0;
        for index in 0..self.0.len() {

            if self.0[index].ilk == Ilk::PayMinerDireclty || self.0[index].ilk == Ilk::PayMinerDirecltyGas {

                assert!(self.0[index].direction == Direction::Out);
                assert!(self.0[index].linked_to.len() == 0);

                let mut steps = 0_usize;
                let mut linked_or_waved = false;

                let mut steps = 0_usize;
                let mut above = false;
                let mut linked = false;
                while !linked {
                    if above {
                        above = false;
                    } else {
                        steps += 1;
                        above = true;
                    }

                    let other_index = if above {
                        std::cmp::min(self.0.len() - 1, index + steps)
                    } else {
                        if steps < index {
                            index - steps 
                        } else {
                            0
                        }
                    };
                    if self.0[other_index].ilk == Ilk::Swap && self.0[other_index].timestamp == self.0[index].timestamp &&self.0[other_index].direction == Direction::In {
                        assert!(self.0[other_index].linked_to.len() == 1 || self.0[other_index].linked_to.len() == 2 || self.0[other_index].linked_to.len() == 3);
                        self.0[other_index].linked_to.push(index);
                        links += 1; 
                        self.0[index].linked_to.push(other_index);
                        links += 1; 
                        linked = true;
                    }
                    // if self.0[other_index].timestamp != self.0[index].timestamp {
                    //     println!("not2");
                    //     break
                    // }
                    
                };
            }
        }
        println!("miner_direct_payment: added: {}, waved: {}",links, waved);
    }

    pub fn link_tx_cancel(&mut self, window: std::time::Duration) {

        let mut links = 0;
        let mut waved = 0;
        for index in 0..self.0.len() {

            if self.0[index].ilk == Ilk::EmptyTransaction {

                assert!(self.0[index].direction == Direction::Out);
                assert!(self.0[index].linked_to.len() == 0);

                let mut steps = 0_usize;
                let mut linked_or_waved = false;
                while !linked_or_waved {

                    let other_index = index + steps;

                    if self.0[other_index].timestamp - self.0[index].timestamp > window.as_millis() as u64 {
                        linked_or_waved = true;
                        waved += 1;
                    } else if (
                        self.0[other_index].ilk == Ilk::Swap && 
                        self.0[other_index].direction == Direction::In && 
                        self.0[other_index].account == self.0[index].account
                        ){
                        self.0[other_index].linked_to.push(index);
                        links += 1; 
                        self.0[index].linked_to.push(other_index);
                        links += 1; 
                        linked_or_waved = true;
                    
                    } 
                    steps += 1;

                    if index + steps >= self.0.len() {
                        linked_or_waved = true;
                        waved += 1;
                    }

                    
                }
            }
        }
        println!("transaction_cancel: added: {}, waved: {}",links, waved);
    }

    pub fn link_swap_fail_gas(&mut self, window: std::time::Duration) {

        let mut links = 0;
        let mut waved = 0;
        for index in 0..self.0.len() {

            if self.0[index].ilk == Ilk::SwapFailGas {

                assert!(self.0[index].direction == Direction::Out);
                assert!(self.0[index].linked_to.len() == 0);

                let mut steps = 0_usize;
                let mut linked_or_waved = false;
                while !linked_or_waved {

                    let other_index = index + steps;

                    if self.0[other_index].timestamp - self.0[index].timestamp > window.as_millis() as u64 {
                        linked_or_waved = true;
                        waved += 1;
                    } else if (
                        self.0[other_index].ilk == Ilk::Swap && 
                        self.0[other_index].direction == Direction::In && 
                        self.0[other_index].account == self.0[index].account
                        ){
                        self.0[other_index].linked_to.push(index);
                        links += 1; 
                        self.0[index].linked_to.push(other_index);
                        links += 1; 
                        linked_or_waved = true;
                    
                    } 
                    steps += 1;

                    if index + steps >= self.0.len() {
                        linked_or_waved = true;
                        waved += 1;
                    }

                    
                }
            }
        }
        println!("swap_fail_gas: added: {}, waved: {}",links, waved);
    }

    pub fn link_manage_liquidity_fail_gas(&mut self, window: std::time::Duration) {

        let mut links = 0;
        let mut waved = 0;
        for index in 0..self.0.len() {

            if self.0[index].ilk == Ilk::ManageLiquidityFailGas {

                assert!(self.0[index].direction == Direction::Out);
                assert!(self.0[index].linked_to.len() == 0);

                let mut steps = 0_usize;
                let mut linked_or_waved = false;
                while !linked_or_waved {

                    let other_index = index + steps;

                    if self.0[other_index].timestamp - self.0[index].timestamp > window.as_millis() as u64 {
                        linked_or_waved = true;
                        waved += 1;
                    } else if (
                        self.0[other_index].ilk == Ilk::ManageLiquidity && 
                        self.0[other_index].direction == Direction::In && 
                        self.0[other_index].account == self.0[index].account
                        ){
                        self.0[other_index].linked_to.push(index);
                        links += 1; 
                        self.0[index].linked_to.push(other_index);
                        links += 1; 
                        linked_or_waved = true;
                    
                    } 
                    steps += 1;

                    if index + steps >= self.0.len() {
                        linked_or_waved = true;
                        waved += 1;
                    }

                    
                }
            }
        }
        println!("manage_liquidity_fail_gas: added: {}, waved: {}",links, waved);
    }

    pub fn link_swap_components (&mut self) {
        let mut links = 0;
        for index in 0..self.0.len() {

            if self.0[index].ilk == Ilk::Swap && self.0[index].direction == Direction::In {

                let mut steps = 0_usize;
                let mut above = false;
                let mut gas_fee_linked = false;
                let mut disposition_linked = false;
                while !gas_fee_linked || !disposition_linked {
                    if above {
                        above = false;
                    } else {
                        steps += 1;
                        above = true;
                    }

                    let other_index = if above {

                        // dbg!(self.0.len() - 1);
                        // dbg!(index + steps);
                        std::cmp::min(self.0.len() - 1, index + steps)
                    } else {
                        if steps < index {
                            index - steps 
                        } else {
                            0
                        }
                    };
                    if !gas_fee_linked {
                        if self.0[other_index].ilk == Ilk::SwapGas && self.0[other_index].identifier == self.0[index].identifier {
                            assert!(self.0[other_index].direction == Direction::Out);
                            assert!(self.0[other_index].linked_to.len() == 0);
                            self.0[other_index].linked_to.push(index);
                            links += 1; 
                            self.0[index].linked_to.push(other_index);
                            links += 1; 
                            gas_fee_linked = true;
                        }
                    }
                    if !disposition_linked {
                        if self.0[other_index].ilk == Ilk::Swap && self.0[other_index].identifier == self.0[index].identifier && index != other_index {
                            // if !(self.0[other_index].direction == Direction::Out) {
                            //     dbg!(index);
                            //     dbg!(other_index);
                            //     dbg!(&self.0[index]);
                            //     dbg!(&self.0[other_index]);
                            // }
                            assert!(self.0[other_index].direction == Direction::Out);
                            assert!(self.0[other_index].linked_to.len() == 0);
                            self.0[other_index].linked_to.push(index);
                            links += 1; 
                            self.0[index].linked_to.push(other_index);
                            links += 1; 
                            disposition_linked = true;
                        }
                    }
                };
            }
        }
        println!("swap links added: {}",links);
    }

    pub fn link_add_liquidity_v3 (&mut self) {
        let mut links = 0;
        for index in 0..self.0.len() {

            if self.0[index].ilk == Ilk::ManageLiquidity && self.0[index].direction == Direction::In && self.0[index].asset.starts_with("UNI-V3-LIQUIDITY") {

                let mut steps = 0_usize;
                let mut above = false;
                // let mut gas_fee_linked = false;
                // let mut disposition_linked = false;

                let mut have_incremented_time_up = false;
                let mut have_incremented_time_down = false;
                while !have_incremented_time_up || !have_incremented_time_down {

                    if above {
                        above = false;
                    } else {
                        steps += 1;
                        above = true;
                    }

                    let other_index = if above {
                        std::cmp::min(self.0.len() - 1, index + steps)

                    } else {
                        if steps < index {
                            index - steps 
                        } else {
                            0
                        }
                    };

                    if index == other_index {
                        have_incremented_time_up = true;
                    }

                    if self.0[other_index].timestamp == self.0[index].timestamp && index != other_index {

                        // if other_index == 41374 {
                        //     dbg!(steps);
                        //     dbg!(above);
                        //     dbg!(&self.0[other_index]);
                        // }

                        // if self.0[other_index].ilk == Ilk::ManageLiquidityGas && self.0[index].identifier.starts_with(&self.0[other_index].identifier) {
                        //     assert!(self.0[other_index].direction == Direction::Out);
                        //     assert!(self.0[other_index].linked_to.len() == 0);
                        //     self.0[other_index].linked_to.push(index);
                        //     links += 1; 
                        //     self.0[index].linked_to.push(other_index);
                        //     links += 1; 
                        //     // gas_fee_linked = true;
                        if self.0[other_index].ilk == Ilk::ManageLiquidity && self.0[other_index].identifier == self.0[index].identifier {
                            assert!(self.0[other_index].direction == Direction::Out);
                            assert!(self.0[other_index].linked_to.len() == 0);
                            self.0[other_index].linked_to.push(index);
                            links += 1; 
                            self.0[index].linked_to.push(other_index);
                            links += 1; 
                            // disposition_linked = true;
                        }

                    } else if self.0[other_index].timestamp > self.0[index].timestamp {
                        have_incremented_time_up = true;
                    } else if self.0[other_index].timestamp < self.0[index].timestamp {
                        have_incremented_time_down = true;
                    }
                    
                };
            }
        }
        println!("add v3 liquiduty links added: {}",links);
    }

    pub fn link_remove_liquidity_v3 (&mut self) {
        let mut links = 0;
        for index in 0..self.0.len() {

            if self.0[index].ilk == Ilk::ManageLiquidity && self.0[index].direction == Direction::Out && self.0[index].asset.starts_with("UNI-V3-LIQUIDITY") {

                let mut steps = 0_usize;
                let mut above = false;
                // let mut gas_fee_linked = false;
                // let mut disposition_linked = false;

                let mut have_incremented_time_up = false;
                let mut have_incremented_time_down = false;
                while !have_incremented_time_up || !have_incremented_time_down {

                    if above {
                        above = false;
                    } else {
                        steps += 1;
                        above = true;
                    }

                    let other_index = if above {
                        std::cmp::min(self.0.len() - 1, index + steps)
                    } else {
                        if steps < index {
                            index - steps 
                        } else {
                            0
                        }
                    };

                    if self.0[other_index].timestamp == self.0[index].timestamp && index != other_index{

                        if self.0[other_index].ilk == Ilk::ManageLiquidity && self.0[other_index].identifier == self.0[index].identifier  {
                            
                            if self.0[other_index].asset.starts_with("UNI-V3-LIQUIDITY") {
                                dbg!(&self.0[index]);
                                dbg!(&self.0[other_index]);
                                dbg!(index, other_index);
                                dbg!(self.0.len());
                                panic!();
                            }
                            if self.0[other_index].direction != Direction::In {
                                dbg!(&self.0[other_index].direction);
                                dbg!(index, other_index);
                                dbg!(self.0.len());
                                panic!();
                            };
                            assert!(self.0[other_index].linked_to.len() == 0);
                            self.0[other_index].linked_to.push(index);
                            links += 1; 
                            self.0[index].linked_to.push(other_index);
                            links += 1; 
                            // disposition_linked = true;
                        }

                    } else if self.0[other_index].timestamp > self.0[index].timestamp || other_index >= self.0.len() - 1 {
                        have_incremented_time_up = true;
                    } else if self.0[other_index].timestamp < self.0[index].timestamp {
                        have_incremented_time_down = true;
                    }
                    
                };
            }
        }
        println!("remove v3 liquiduty links added: {}",links);
    }

    pub fn link_manage_liquidity_gas_v3 (&mut self) {
        let mut links = 0;
        for index in 0..self.0.len() {

            if self.0[index].ilk == Ilk::ManageLiquidityGas && self.0[index].direction == Direction::Out {

                let mut steps = 0_usize;
                let mut above = false;
                let mut linked = false;
                // let mut disposition_linked = false;

                let mut have_incremented_time_up = false;
                let mut have_incremented_time_down = false;

                while !linked {

                    if have_incremented_time_down && have_incremented_time_up {
                        dbg!(&self.0[index].identifier);
                        linked = true;
                        // panic!("");
                    };

                    if above {
                        above = false;
                    } else {
                        steps += 1;
                        above = true;
                    }

                    let other_index = if above {
                        std::cmp::min(self.0.len() - 1, index + steps)
                    } else {
                        if steps < index {
                            index - steps 
                        } else {
                            0
                        }
                    };

                    if self.0[other_index].timestamp == self.0[index].timestamp {

                        if self.0[other_index].ilk == Ilk::ManageLiquidity && self.0[other_index].direction == Direction::In && self.0[other_index].identifier.starts_with(&self.0[index].identifier) {
                            assert!(self.0[index].linked_to.len() == 0);
                            self.0[other_index].linked_to.push(index);
                            links += 1; 
                            self.0[index].linked_to.push(other_index);
                            links += 1; 
                            linked = true;
                        }
                            // gas_fee_linked = true;
                    } else if self.0[other_index].timestamp > self.0[index].timestamp {
                        have_incremented_time_up = true;
                    } else if self.0[other_index].timestamp < self.0[index].timestamp {
                        have_incremented_time_down = true;
                    }
                    
                };
            }
        }
        println!("add v3 liquiduty links added: {}",links);
    }

    pub fn link_remove_liquidity_components (&mut self) {
        let mut links = 0;
        for index in 0..self.0.len() {

            if self.0[index].ilk == Ilk::RemoveLiquidity && self.0[index].direction == Direction::In {

                let mut steps = 0_usize;
                let mut above = false;
                let mut gas_fee_linked = false;
                let mut disposition_linked = false;
                while !gas_fee_linked || !disposition_linked {
                    if above {
                        above = false;
                    } else {
                        steps += 1;
                        above = true;
                    }

                    let other_index = if above {
                        std::cmp::min(self.0.len() - 1, index + steps)
                    } else {
                        if steps < index {
                            index - steps 
                        } else {
                            0
                        }
                    };
                    if !gas_fee_linked {
                        if self.0[other_index].ilk == Ilk::RemoveLiquidityGas && self.0[other_index].identifier == self.0[index].identifier {
                            assert!(self.0[other_index].direction == Direction::Out);
                            assert!(self.0[other_index].linked_to.len() < 2);
                            self.0[other_index].linked_to.push(index);
                            links += 1; 
                            self.0[index].linked_to.push(other_index);
                            links += 1; 
                            gas_fee_linked = true;
                        }
                    }
                    if !disposition_linked {
                        if self.0[other_index].ilk == Ilk::RemoveLiquidity && self.0[other_index].direction == Direction::Out && self.0[other_index].identifier == self.0[index].identifier {
                            assert!(self.0[other_index].linked_to.len() < 2);
                            self.0[other_index].linked_to.push(index);
                            links += 1; 
                            self.0[index].linked_to.push(other_index);
                            links += 1; 
                            disposition_linked = true;
                        }
                    }
                };
            }
        }
        println!("remove liquidity links added: {}",links);
    }

    pub fn index_link_type_count(&self, index: usize, ilk: Ilk) -> usize {
        let mut count = 0;
        for i in &self.0[index].linked_to {
            if self.0[*i].ilk == ilk {
                count += 1;
            }

        }
        count

    }

    pub fn reassign_quote_fee_links(&mut self, quote_currency: &str) {
        let mut total_reassigned = 0;
        let mut waved = 0;
        for match_in_index in 0..self.0.len() {
            if self.0[match_in_index].ilk == Ilk::Match 
                && self.0[match_in_index].direction == Direction::In 
                && self.0[match_in_index].asset == quote_currency 
                && self.index_link_type_count(match_in_index, Ilk::TradeFee) == 1 {
                assert!(self.0[match_in_index].linked_to.len() == 2);
                assert!(self.0[match_in_index].host.is_custodial_exchange());

                
                let mut removed = 0;
                let mut added = 0;

                let mut fee_index_option = None;
                for potential_fee_index in self.0[match_in_index].linked_to.clone() {
                    if self.0[potential_fee_index].ilk == Ilk::TradeFee {
                        fee_index_option = Some(potential_fee_index);
                    }
                    
                }

                let fee_index = fee_index_option.unwrap();

                {
                    let i_option = self.0[fee_index].linked_to.iter().position(|x| x == &match_in_index);
                    let i = i_option.unwrap();
                    self.0[fee_index].linked_to.remove(i);
                    removed += 1;
                }
                {
                    let i_option = self.0[match_in_index].linked_to.iter().position(|x| x == &fee_index);
                    let i = i_option.unwrap();
                    self.0[match_in_index].linked_to.remove(i);
                    removed += 1;
                }

                let mut above = false;
                let mut steps = 0; 
                while added < 2 {
                    if above {
                        above = false;
                    } else {
                        steps += 1;
                        above = true;
                    }

                    let other_index = if above {
                        std::cmp::min(self.0.len() - 1, fee_index + steps)
                    } else {
                        if steps < fee_index {
                            fee_index - steps 
                        } else {
                            0
                        }
                    };
                        
                    if self.0[other_index].ilk == Ilk::Match && self.0[other_index].direction == Direction::Out && self.0[other_index].identifier == self.0[fee_index].identifier {
                        assert!(self.0[other_index].identifier == self.0[match_in_index].identifier);
                        assert!(self.0[fee_index].ilk == Ilk::TradeFee && self.0[fee_index].direction == Direction::Out);
                        assert!(self.0[other_index].linked_to.len() == 1);
                        self.0[fee_index].linked_to.push(other_index);
                        added += 1; 
                        self.0[other_index].linked_to.push(fee_index);
                        added += 1; 
                    }
                    
                    if steps > 10 {
                        println!("{:#?}", self.0[match_in_index]);
                        waved += 1;
                        panic!("")
                    }
                        

                }
                assert!(added == removed);
                total_reassigned += removed;
            }
        }
        println!("reassined: {}", total_reassigned);
    }

    pub fn link_trade_components (&mut self) {

        // let mut same_id = 0;
        // let mut waved = 0;
        // let mut close_timestamp = 0;

        let mut links = 0;
        let mut waved = 0;
        for index in 0..self.0.len() {

            if self.0[index].ilk == Ilk::Match && self.0[index].direction == Direction::In {
                match self.0[index].host {
                    Host::Mainnet => {
                    },
                    Host::Optimism => {
                    },
                    Host::Base => {
                    },
                    Host::Optimism10 => {
                    },
                    Host::Optimism20 => {
                    },
                    Host::ArbitrumOne => {
                    },
                    Host::PolygonPos => {
                    },
                    Host::CoinbaseDotcom => {
                        panic!("");
                        // assert!(self.0[index].ilk == Ilk::WithdrawalFee);
                    },
                    Host::Coinbase => {
                        let mut steps = 0_usize;
                        let mut above = false;
                        let mut fee_linked_or_waved = false;
                        let mut disposition_linked = false;
                        while !fee_linked_or_waved || !disposition_linked {
                            if above {
                                above = false;
                            } else {
                                steps += 1;
                                above = true;
                            }

                            let other_index = if above {
                                std::cmp::min(self.0.len() - 1, index + steps)
                            } else {
                                if steps < index {
                                    index - steps 
                                } else {
                                    0
                                }
                            };
                            if !fee_linked_or_waved {
                                
                                if self.0[other_index].timestamp != self.0[index].timestamp && steps > 10 {
                                    fee_linked_or_waved = true;
                                    waved += 1;

                                }
                                if self.0[other_index].ilk == Ilk::TradeFee && self.0[other_index].identifier == self.0[index].identifier {
                     //             assert!(self.0[other_index].timestamp == self.0[index].timestamp);
                                    assert!(self.0[other_index].direction == Direction::Out);
                                    assert!(self.0[other_index].linked_to.len() == 0);
                                    self.0[other_index].linked_to.push(index);
                                    links += 1; 
                                    self.0[index].linked_to.push(other_index);
                                    links += 1; 
                                    fee_linked_or_waved = true;
                                }
                            }
                            if !disposition_linked {
                                if self.0[other_index].ilk == Ilk::Match && self.0[other_index].identifier == self.0[index].identifier {
                     //             assert!(self.0[other_index].timestamp == self.0[index].timestamp);
                                    assert!(self.0[other_index].direction == Direction::Out);
                                    assert!(self.0[other_index].linked_to.len() == 0);
                                    self.0[other_index].linked_to.push(index);
                                    links += 1; 
                                    self.0[index].linked_to.push(other_index);
                                    links += 1; 
                                    disposition_linked = true;
                                }
                            }
                        };
                    },
                    Host::CoinbasePro => {
                        let mut steps = 0_usize;
                        let mut above = false;
                        let mut fee_linked_or_waved = false;
                        let mut disposition_linked = false;
                        while !fee_linked_or_waved || !disposition_linked {
                            if above {
                                above = false;
                            } else {
                                steps += 1;
                                above = true;
                            }

                            let other_index = if above {
                                std::cmp::min(self.0.len() - 1, index + steps)
                            } else {
                                if steps < index {
                                    index - steps 
                                } else {
                                    0
                                }
                            };
                            if !fee_linked_or_waved {
                                
                                if self.0[other_index].timestamp != self.0[index].timestamp && steps > 10 {
                                    fee_linked_or_waved = true;
                                    waved += 1;

                                }
                                if self.0[other_index].ilk == Ilk::TradeFee && self.0[other_index].identifier == self.0[index].identifier {
                     //             assert!(self.0[other_index].timestamp == self.0[index].timestamp);
                                    assert!(self.0[other_index].direction == Direction::Out);
                                    assert!(self.0[other_index].linked_to.len() == 0);
                                    self.0[other_index].linked_to.push(index);
                                    links += 1; 
                                    self.0[index].linked_to.push(other_index);
                                    links += 1; 
                                    fee_linked_or_waved = true;
                                }
                            }
                            if !disposition_linked {
                                if self.0[other_index].ilk == Ilk::Match && self.0[other_index].identifier == self.0[index].identifier {
                     //             assert!(self.0[other_index].timestamp == self.0[index].timestamp);
                                    assert!(self.0[other_index].direction == Direction::Out);
                                    assert!(self.0[other_index].linked_to.len() == 0);
                                    self.0[other_index].linked_to.push(index);
                                    links += 1; 
                                    self.0[index].linked_to.push(other_index);
                                    links += 1; 
                                    disposition_linked = true;
                                }
                            }
                        };
                    },
                    Host::FtxUs => {
                        // panic!("not implemented; everything below is copy of coinbase");
                        let mut steps = 0_usize;
                        let mut above = false;
                        let mut fee_linked_or_waved = false;
                        let mut disposition_linked = false;
                        while !fee_linked_or_waved || !disposition_linked {
                            if above {
                                above = false;
                            } else {
                                steps += 1;
                                above = true;
                            }

                            let other_index = if above {
                                std::cmp::min(self.0.len() - 1, index + steps)
                            } else {
                                if steps < index {
                                    index - steps 
                                } else {
                                    0
                                }
                            };
                            if !fee_linked_or_waved {
                                
                                if self.0[other_index].timestamp != self.0[index].timestamp && steps > 10 {
                                    fee_linked_or_waved = true;
                                    waved += 1;

                                }
                                if self.0[other_index].ilk == Ilk::TradeFee && self.0[other_index].identifier == self.0[index].identifier && self.0[other_index].direction == Direction::Out {
                     //             assert!(self.0[other_index].timestamp == self.0[index].timestamp);
                                    // assert!(self.0[other_index].direction == Direction::Out);
                                    assert!(self.0[other_index].linked_to.len() == 0);
                                    self.0[other_index].linked_to.push(index);
                                    links += 1; 
                                    self.0[index].linked_to.push(other_index);
                                    links += 1; 
                                    fee_linked_or_waved = true;
                                }
                            }
                            if !disposition_linked {
                                if self.0[other_index].ilk == Ilk::Match && self.0[other_index].identifier == self.0[index].identifier {
                     //             assert!(self.0[other_index].timestamp == self.0[index].timestamp);
                                    assert!(self.0[other_index].direction == Direction::Out);
                                    assert!(self.0[other_index].linked_to.len() == 0);
                                    self.0[other_index].linked_to.push(index);
                                    links += 1; 
                                    self.0[index].linked_to.push(other_index);
                                    links += 1; 
                                    disposition_linked = true;
                                }
                            }
                        };
                    },
                    Host::Binance => {
                        let mut steps = 0_usize;
                        let mut above = false;
                        let mut fee_linked = false;
                        let mut disposition_linked = false;
                        while !fee_linked || !disposition_linked {
                            if above {
                                above = false;
                            } else {
                                steps += 1;
                                above = true;
                            }

                            let other_index = if above {
                                std::cmp::min(self.0.len() - 1, index + steps)
                            } else {
                                if steps < index {
                                    index - steps 
                                } else {
                                    0
                                }
                            };
                            if !fee_linked {
                                if self.0[other_index].ilk == Ilk::TradeFee && self.0[other_index].identifier == self.0[index].identifier {
                                    assert!(self.0[other_index].timestamp == self.0[index].timestamp);
                                    assert!(self.0[other_index].direction == Direction::Out);
                                    assert!(self.0[other_index].linked_to.len() == 0);
                                    self.0[other_index].linked_to.push(index);
                                    links += 1; 
                                    self.0[index].linked_to.push(other_index);
                                    links += 1; 
                                    fee_linked = true;
                                }
                            }
                            if !disposition_linked {
                                if self.0[other_index].ilk == Ilk::Match && self.0[other_index].identifier == self.0[index].identifier {
                                    assert!(self.0[other_index].timestamp == self.0[index].timestamp);
                                    assert!(self.0[other_index].direction == Direction::Out);
                                    assert!(self.0[other_index].linked_to.len() == 0);
                                    self.0[other_index].linked_to.push(index);
                                    links += 1; 
                                    self.0[index].linked_to.push(other_index);
                                    links += 1; 
                                    disposition_linked = true;
                                }
                            }
                        };
                    },
                    Host::BinanceUs => {
                        let mut steps = 0_usize;
                        let mut above = false;
                        let mut fee_linked = false;
                        let mut disposition_linked = false;
                        while !fee_linked || !disposition_linked {
                            if above {
                                above = false;
                            } else {
                                steps += 1;
                                above = true;
                            }

                            let other_index = if above {
                                std::cmp::min(self.0.len() - 1, index + steps)
                            } else {
                                if steps < index {
                                    index - steps 
                                } else {
                                    0
                                }
                            };
                            if self.0[index].identifier.starts_with("ETHUSD") {
                                fee_linked = true;
                            }
                            if !fee_linked {
                                
                                if self.0[other_index].ilk == Ilk::TradeFee && self.0[other_index].identifier == self.0[index].identifier {
                                    assert!(self.0[other_index].timestamp == self.0[index].timestamp);
                                    assert!(self.0[other_index].direction == Direction::Out);
                                    assert!(self.0[other_index].linked_to.len() == 0);
                                    self.0[other_index].linked_to.push(index);
                                    links += 1; 
                                    self.0[index].linked_to.push(other_index);
                                    links += 1; 
                                    fee_linked = true;
                                }
                            }
                            if !disposition_linked {
                                if self.0[other_index].ilk == Ilk::Match && self.0[other_index].identifier == self.0[index].identifier {
                                    assert!(self.0[other_index].timestamp == self.0[index].timestamp);
                                    assert!(self.0[other_index].direction == Direction::Out);
                                    assert!(self.0[other_index].linked_to.len() == 0);
                                    self.0[other_index].linked_to.push(index);
                                    links += 1; 
                                    self.0[index].linked_to.push(other_index);
                                    links += 1; 
                                    disposition_linked = true;
                                }
                            }
                        };
                    },
                    Host::Kucoin => {

                        let mut steps = 0_usize;
                        let mut above = false;
                        let mut fee_linked_or_waved = false;
                        let mut disposition_linked = false;
                        while !disposition_linked || !fee_linked_or_waved {
                            if above {
                                above = false;
                            } else {
                                steps += 1;
                                above = true;
                            }

                            let other_index = if above {
                                std::cmp::min(self.0.len() - 1, index + steps)
                            } else {
                                if steps < index {
                                    index - steps 
                                } else {
                                    0
                                }
                            };
                            if !fee_linked_or_waved {

                                
                                if above && self.0[other_index].timestamp - self.0[index].timestamp > 0 {
                                    // println!("{:?}", self.0[index]);
                                    fee_linked_or_waved = true;

                                    waved += 1;
                                } else if !above && self.0[index].timestamp - self.0[other_index].timestamp > 0 {
                                    // println!("{:?}", self.0[index]);
                                    fee_linked_or_waved = true;
                                    waved += 1;
                                } else if self.0[other_index].ilk == Ilk::TradeFee && 
                                    self.0[other_index].host == Host::Kucoin &&
                                    self.0[other_index].linked_to.len() == 0 &&
                                    self.0[other_index].timestamp == self.0[index].timestamp 
                                    {
                                    assert!(self.0[other_index].direction == Direction::Out);
                                    if self.0[other_index].identifier == self.0[index].identifier {
                                        self.0[other_index].linked_to.push(index);
                                        links += 1; 
                                        self.0[index].linked_to.push(other_index);
                                        links += 1; 
                                        fee_linked_or_waved = true;
                                        // same_id += 1;
                                    } else {
                                        self.0[other_index].linked_to.push(index);
                                        links += 1; 
                                        self.0[index].linked_to.push(other_index);
                                        links += 1; 
                                        fee_linked_or_waved = true;
                                        // close_timestamp += 1;
                                    }
                                }
                                
                            }
                            if !disposition_linked {
                                if self.0[other_index].host == Host::Kucoin && 
                                        self.0[other_index].ilk == Ilk::Match && 
                                        self.0[other_index].identifier == self.0[index].identifier && 
                                        self.0[other_index].direction == Direction::Out {
                                    assert!(self.0[other_index].timestamp == self.0[index].timestamp);
                                    // assert!(self.0[other_index].direction == Direction::Out);
                                    if self.0[other_index].direction != Direction::Out {
                                        println!("index {:?}", self.0[index]);
                                        println!("other {:?}", self.0[other_index]);
                                    };
                                    assert!(self.0[other_index].linked_to.len() == 0);
                                    self.0[other_index].linked_to.push(index);
                                    links += 1; 
                                    self.0[index].linked_to.push(other_index);
                                    links += 1; 
                                    disposition_linked = true;

                                }
                            }
                            if index == self.0.len() - 1{
                                println!("last index");



                            } else if other_index == self.0.len() - 1 {
                                println!("{:?}", self.0[index]);
                                println!("last other index");
                                waved += 1;
                                break

                            }
                        };



                    },
                    Host::DydxSoloMargin => {
                        let mut steps = 0_usize;
                        let mut above = false;
                        let mut fee_linked_or_waved = false;
                        let mut disposition_linked = false;
                        while !fee_linked_or_waved || !disposition_linked {
                            if above {
                                above = false;
                            } else {
                                steps += 1;
                                above = true;
                            }

                            let other_index = if above {
                                std::cmp::min(self.0.len() - 1, index + steps)
                            } else {
                                if steps < index {
                                    index - steps 
                                } else {
                                    0
                                }
                            };
                            if !fee_linked_or_waved {
                                if self.0[other_index].ilk == Ilk::TradeFee && self.0[other_index].identifier == self.0[index].identifier {
                                    assert!(self.0[other_index].timestamp == self.0[index].timestamp);
                                    assert!(self.0[other_index].direction == Direction::Out);
                                    assert!(self.0[other_index].linked_to.len() == 0);
                                    self.0[other_index].linked_to.push(index);
                                    links += 1; 
                                    self.0[index].linked_to.push(other_index);
                                    links += 1; 
                                    fee_linked_or_waved = true;
                                } else if self.0[other_index].timestamp != self.0[index].timestamp {
                                    fee_linked_or_waved = true;
                                    waved += 1;
                                }
                            }
                            if !disposition_linked {
                                if self.0[other_index].ilk == Ilk::Match && self.0[other_index].identifier == self.0[index].identifier {
                                    assert!(self.0[other_index].timestamp == self.0[index].timestamp);
                                    assert!(self.0[other_index].linked_to.len() == 0);
                                    self.0[other_index].linked_to.push(index);
                                    links += 1; 
                                    self.0[index].linked_to.push(other_index);
                                    links += 1; 
                                    disposition_linked = true;
                                }
                            }
                        };
                    },
                }
            }
        }
        // println!("same_id: {}, close_timestamp: {}, waved: {}", same_id, close_timestamp, waved);
        println!("trade links added: {}, waved: {}",links, waved);
    }

    pub fn used_assets(&self) -> Vec<String> {
        let mut uas = Vec::new();
        for delta in &self.0 {
            if delta.asset.starts_with("UNI-V3-LIQUIDITY") {
                continue
            };
            if !uas.contains(&delta.asset) {
                uas.push(delta.asset.clone());
            }
        }
        uas
    }

    pub fn disposition_links(&self) {
        let mut zero = 0;
        let mut one = 0;
        let mut two = 0;

        let mut unlinked = HashSet::new();
        let mut unlinked_map = HashMap::new();

        for delta in &self.0 {
            if delta.direction == Direction::Out {
                if delta.linked_to.len() == 0 {
                    unlinked.insert(delta.ilk.clone());
                    if 
                        delta.ilk != Ilk::UnwrapEth && 
                        delta.ilk != Ilk::WrapEth && 
                        delta.ilk != Ilk::WithdrawalToBank &&
                        delta.ilk != Ilk::WithdrawalFee &&
                        delta.ilk != Ilk::ChangeMakerVault 
                            {
                        if unlinked_map.contains_key(&delta.asset) {
                            *unlinked_map.get_mut(&delta.asset).unwrap() += delta.qty;
                        } else {
                            unlinked_map.insert(delta.asset.clone(), delta.qty);
                        };
                    }
                    // println!("{:?}", delta);
                    zero += 1;
                } else if delta.linked_to.len() == 1 {
                    one += 1;
                } else if delta.linked_to.len() == 2 {
                    two += 1;
                } else {
                    panic!("");
                }
            }
        }
        println!("disposition_links: zero: {}, one: {}. two: {}", zero, one, two);
        println!("unlinked types: {:?}", unlinked);
        println!("unlinked totals: {:#?}", unlinked_map);
    }


    // pub fn acquisitions_that_need_link(&self) {


    //     let mut total = 0;
    //     let mut unlinked = 0;
    //     for delta in &self.0 {
    //         if delta.is_aquisition_that_needs_link() {
    //             total += 1;
    //             if delta.linked_to.len() == 0 {
    //                 println!("{:?}", delta); 
    //                 unlinked += 1;
    //             }
    //         }
    //     }
    //     println!("{} unlinked of {} total", unlinked, total);
    // }


    pub fn index_cost (&self, index: usize, quote_currency: &str, prices: &prices::Prices) -> f64 {

        let delta = &self.0[index];
        assert!(delta.direction == Direction::In);

        let cost = if delta.asset == quote_currency {

            // delta.value(quote_currency, prices)
            0.0
        } else if delta.ilk == Ilk::RemoveLiquidity {
            delta.value(quote_currency, prices)
        } else if delta.ilk == Ilk::ManageLiquidity && !delta.asset.starts_with("UNI-V3-LIQUIDITY") {
            let mut c = delta.value(quote_currency, prices);
            for index in &delta.linked_to {
                if !self.0[*index].asset.starts_with("UNI-V3-LIQUIDITY") {
                    assert!(self.0[*index].ilk == Ilk::ManageLiquidityGas || self.0[*index].ilk == Ilk::ManageLiquidityFailGas);
                    assert!(self.0[*index].direction == Direction::Out);
                    c += self.0[*index].value(quote_currency, prices)
                }
            }
            c
        } else if delta.ilk == Ilk::ChangeMakerVault {
            assert!(delta.asset == "DAI");
            delta.value(quote_currency, prices)
        } else if delta.ilk == Ilk::Loan {
            assert!(delta.asset == "ETH");
            delta.value(quote_currency, prices)
        } else if delta.ilk == Ilk::Airdrop {

            let mut c = delta.value(quote_currency, prices);
            for index in &delta.linked_to {
                c += self.0[*index].value(quote_currency, prices)
            }
            c
        } else if delta.ilk == Ilk::SwapFees {
            0.0

            // let mut c = delta.value(quote_currency, prices);
            // for index in &delta.linked_to {
            //     c += self.0[*index].value(quote_currency, prices)
            // }
            // c

        } else {
            let mut c = 0f64;
            for index in &delta.linked_to {
                assert!(self.0[*index].direction == Direction::Out);
                c += self.0[*index].value(quote_currency, prices);
                
            }


            //////////2021TEST////////////////////
            // if index % 4 == 0 || index % 7 == 0 {
            //     c *= 1.0031
            // }
            //////////2021TEST////////////////////

            //////////2023TEST////////////////////
            if delta.timestamp % 7000 == 0 && delta.timestamp < 1704067200000 && delta.timestamp >= 1672531200000 {
                c *= 1.00095
            }
            //////////2023TEST////////////////////

            //////////2024TEST////////////////////
            // if delta.timestamp % 7000 == 0 && delta.timestamp < 1735689600000 && delta.timestamp >= 1704067200000 {
            //     c *= 1.00095
            // }
            //////////2024TEST////////////////////
            

            c

        };
        cost
    }

    pub fn index_income (&self, index: usize, quote_currency: &str, prices: &prices::Prices) -> f64 {
        let delta = &self.0[index];
        assert!(delta.direction == Direction::In);
        let income = if delta.ilk == Ilk::Airdrop {
            // dbg!(format!("airdrop: {}, {}", chrono::Utc.timestamp_millis(self.0[index].timestamp as i64), delta.value(quote_currency, prices));
            delta.value(quote_currency, prices)
        } else if delta.ilk == Ilk::TradeFee && delta.direction == Direction::In {
            delta.value(quote_currency, prices)
            // 0_f64
        // } else if delta.ilk == Ilk::SwapFees && delta.direction == Direction::In {
        //     delta.value(quote_currency, prices)
        //     // 0_f64
        } else {
            0_f64
        };
        income
    }

    pub fn index_revenue(&self, index: usize, quote_currency: &str, prices: &prices::Prices) -> f64 {
        // if &self.0[index].asset == "UNI-V3-LIQUIDITY:494643_WETH_ARB_500_73280_73340" {
        //     dbg!();
        // }
        let delta = &self.0[index];
        assert!(delta.direction == Direction::Out);

        
        
        let rev = if delta.asset == quote_currency {
            // delta.value(quote_currency, prices)
            0.0
            

        } else if delta.ilk == Ilk::RemoveLiquidity {
            let mut c = 0f64;
            for index in &delta.linked_to {
                c += self.0[*index].value(quote_currency, prices)
            }
            c
        } else if delta.ilk == Ilk::ManageLiquidity && delta.asset.starts_with("UNI-V3-LIQUIDITY") {
            // dbg!(delta.linked_to.len());
            let mut c = 0f64;
            for index in &delta.linked_to {
                assert!(self.0[*index].ilk == Ilk::ManageLiquidity);
                assert!(self.0[*index].direction == Direction::In);
                c += self.0[*index].value(quote_currency, prices)
            }
            c
        } else {
            let mut r = delta.value(quote_currency, prices);
            for i in &delta.linked_to {
                if i < &self.0.len() && self.0[*i].asset == quote_currency && self.0[*i].direction == Direction::In {
                    // panic!("");

                    r = self.0[*i].qty;
                }
            }
            for i in &delta.linked_to {
                if i < &self.0.len() && self.0[*i].asset == quote_currency && self.0[*i].direction == Direction::Out {
                    assert!(self.0[*i].ilk == Ilk::TradeFee);
                    r -= self.0[*i].qty;
                }
            }
            r
        };
        rev
    }

    pub fn link_unused_kucoin_fees_within(&mut self, tolerance: u64) {

        let mut links = 0;
        let mut waved = 0;
        for index in 0..self.0.len() {

            if self.0[index].ilk == Ilk::Match && self.0[index].host == Host::Kucoin && self.0[index].linked_to.len() == 1 && self.0[index].direction == Direction::In {

                let mut steps = 0_usize;
                let mut above = false;
                let mut fee_linked_or_waved = false;
                while !fee_linked_or_waved {
                    if above {
                        above = false;
                    } else {
                        steps += 1;
                        above = true;
                    }

                    let other_index = if above {
                        std::cmp::min(self.0.len() - 1, index + steps)
                    } else {
                        if steps < index {
                            index - steps 
                        } else {
                            0
                        }
                    };
                    if !fee_linked_or_waved {

                        
                        if above && self.0[other_index].timestamp - self.0[index].timestamp > tolerance*1000 {
                            // println!("{:?}", self.0[index]);
                            fee_linked_or_waved = true;

                            waved += 1;
                        } else if !above && self.0[index].timestamp - self.0[other_index].timestamp > tolerance*1000 {
                            // println!("{:?}", self.0[index]);
                            fee_linked_or_waved = true;
                            waved += 1;
                        } else if (
                            self.0[other_index].ilk == Ilk::TradeFee && 
                            self.0[other_index].host == Host::Kucoin &&
                            self.0[other_index].linked_to.len() == 0 
                            ){
                            self.0[other_index].linked_to.push(index);
                            links += 1; 
                            self.0[index].linked_to.push(other_index);
                            links += 1; 
                            fee_linked_or_waved = true;
                        }
                        
                    }
                    if index == self.0.len() - 1{
                        println!("last index");



                    } else if other_index == self.0.len() - 1 {
                        println!("{:?}", self.0[index]);
                        println!("last other index");
                        waved += 1;
                        break

                    }
                };


            }
        }

        println!("tolerance: {}, added: {}, waved: {}", tolerance, links, waved);
    }
}


#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Delta {
    pub timestamp: u64,
    pub direction: Direction,
    pub ilk: Ilk,
    pub asset: String,
    pub qty: f64,
    pub host: Host,
    pub account: String,
    pub identifier: String,
    pub linked_to: Vec<usize>
}

impl Delta {

    pub fn value (&self, quote_currency: &str, prices: &prices::Prices) -> f64 {

        let symbol = symbols::onchain_ticker_to_tax_ticker(&self.asset);

        let value = if self.asset == quote_currency {
            self.qty
        } else {
            let price = prices.price_at_millis(&symbol, self.timestamp);
            self.qty * price

        };
        value
    }
}


#[derive(PartialEq, Clone, Debug, Serialize, Deserialize)]
pub enum Direction {
    In,
    Out
}

#[derive(PartialEq, Clone, Debug, Serialize, Deserialize)]
pub enum Host {
    Mainnet, 
    Optimism,
    Base,
    Optimism10,
    Optimism20,
    ArbitrumOne,
    Coinbase,
    CoinbasePro,
    CoinbaseDotcom,
    Binance,
    BinanceUs,
    Kucoin,
    DydxSoloMargin,
    PolygonPos,
    FtxUs,
}

impl Host {
    pub fn is_custodial_exchange(&self) -> bool {
        match self {
            Host::Mainnet => false,
            Host::Optimism => false,
            Host::Base => false,
            Host::Optimism10 => false,
            Host::Optimism20 => false,
            Host::ArbitrumOne => false,
            Host::Coinbase => true,
            Host::CoinbasePro => true,
            Host::CoinbaseDotcom => true,
            Host::Binance => true,
            Host::BinanceUs => true,
            Host::Kucoin => true,
            Host::DydxSoloMargin => panic!(""),
            Host::PolygonPos => false,
            Host::FtxUs => true,
        }
    }

    pub fn to_string(&self) -> String {
        match self {

            Host::Mainnet => "mainnet".to_string(),
            Host::Optimism => "optimism".to_string(),
            Host::Base => "base".to_string(),
            Host::Optimism10 => "optimism".to_string(),
            Host::Optimism20 => "optimism".to_string(),
            Host::ArbitrumOne => "arbitrum_one".to_string(),
            Host::Coinbase => "coinbase".to_string(),
            Host::CoinbasePro => "coinbase_pro".to_string(),
            Host::CoinbaseDotcom => "coinbase".to_string(),
            Host::Binance => "binance".to_string(),
            Host::BinanceUs => "binance_us".to_string(),
            Host::Kucoin => "kucoin".to_string(),
            Host::DydxSoloMargin => panic!(""),
            Host::PolygonPos => "polygon_pos".to_string(),
            Host::FtxUs => "ftx_us".to_string(),

        }
    }
}


#[derive(std::hash::Hash, Eq, PartialEq, Clone, Debug, Serialize, Deserialize)]
pub enum Ilk {
    Swap,
    SwapGas,
    SwapFailGas,
    WalletToWalletGas,
    WrapEth,
    WrapEthGas,
    WrapEthFailGas,
    UnwrapEth,
    UnwrapEthGas,
    UnwrapEthFailGas,
    ApproveGas,
    ApproveFailGas,
    Payment,
    PaymentGas,
    Erc20TransferFailGas,
    Airdrop,
    AirdropClaimGas,
    TokenMigration,
    TokenMigrationGas,
    DeployContractGas,
    DeployContractFailGas,
    EmptyTransaction,
    RemoveLiquidity,
    RemoveLiquidityGas,
    AllowOnContractGas,
    PayMinerDireclty,
    PayMinerDirecltyGas,
    CreateMakerVaultGas,
    ChangeMakerVaultGas,
    ChangeMakerVaultFailGas,
    ChangeMakerVault,
    DydxDepositGas,
    DydxDeposit,
    DydxWithdraw,
    OperateSoloMarginGas,
    OperateSoloMarginFailGas,
    BridgeGas,
    BridgeFee,
    MalformedTxGas,
    Match,
    TradeFee,
    WithdrawalFee,
    CoinbaseDepositGas,
    CoinbaseConversion,
    DepositDiscrepancy,
    WithdrawalToBank,
    BinanceDepositGas,
    KucoinDepositGas,
    BridgeFeeRefund,
    DelegateGas,
    AirdropClaimFailGas,
    FtxusDepositGas,
    AutomaticConversion,
    Loss,
    ManageLiquidityGas,
    ManageLiquidityFailGas,
    ManageLiquidity,
    SwapFees,
    WalletDiscovery,
    PhishingAttempt,
    Loan,
    LoanInterestPayment,
    CoinbaseInterest,
}


