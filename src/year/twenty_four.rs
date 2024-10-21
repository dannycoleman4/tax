use std::collections::HashMap;
use crate::inventory;
use crate::deltas;
use crate::prices;
use chrono::{Utc, TimeZone};
use crate::symbols;



pub fn calculate(method: inventory::InventoryMethod) {
    let mut inventory = load_initial_inventory_us();
    inventory.add_asset("USDC.OPTIMISM");
    inventory.add_asset("USDC.BASE");
    // let mut inventory = inventory::Inventory::load("./2024/initial_inventory_us.json").unwrap();
    let prices = prices::Prices::load("./data/2024/prices_USD.json").unwrap();
    let mut deltas = deltas::Deltas::load("./data/2024/linked_deltas.json").unwrap();
    deltas.reassign_quote_fee_links("USD");

    let (summary, dispositions) = inventory.apply_deltas(&deltas, "USD", &prices, method);
    
    // summary.save("./2024/summary_us.json");
    inventory.save("./data/2024/end_inventory_us.json");

    check_end_inventory();


    let mut report = String::new();
    report += "\n";
    report += "all values in USD\n";
    report += "day average (hourly vwap) prices from cryptocompare.com used to determine fair market value\n"; 
    report += "\n";

    report += "2024 cryptocurrency income (\"airdrops\"):\n";
    report += &format!(" income: {:.8}\n", summary.income);
    report += "\n";
    report += "2024 cryptocurrency capital_gains:\n";
    report += &format!(" inventory method: {:.8}\n", summary.inventory_method);
    report += &format!(" short term capital gains: {:.8}\n", summary.short_term_capital_gains);
    report += &format!(" long term capital gains: {:.8}\n", summary.long_term_capital_gains);
    report += "\n";
    println!("{}", report);

    let fp = "./data/2024/all_dispositions_us.csv";
    std::fs::write(fp, dispositions);

    let fp = "./data/2024/capital_gains_report_us.txt";
    std::fs::write(fp, report);

}

pub fn load_initial_inventory_us() -> inventory::Inventory {

    let initial_balances = {
        let data = std::fs::read_to_string("./data/2024/initial_holdings.json").unwrap();
        let ib: HashMap<String, f64> = serde_json::from_str(&data).unwrap();
        ib
    };

    // fir 2024, will need to load positions and check those too

    let mut initial_inventory = inventory::Inventory::load("./data/2023/end_inventory_us.json").unwrap();


    for (asset_id, acq_vec) in &initial_inventory.0 {
        let tot_inv = {
            let mut ti = 0.0;
            for acq in acq_vec {
                ti += acq.qty;
            }
            ti
        };
        let exp_bal = if asset_id == "ETH" {
            initial_balances["ETH"] + initial_balances["WETH"]
        } else if asset_id == "BTC" {
            initial_balances["BTC"] + initial_balances["WBTC"]
        } else if asset_id == "REP" {
            initial_balances["REP"] + initial_balances["REPv2"]
        } else if asset_id == "USDC" {
            initial_balances["USDC"] + initial_balances["USDC.ARBITRUM"] + initial_balances["USDC.BASE"] + initial_balances["USDC.OPTIMISM"]
        } else {
            initial_balances[asset_id]
        };
        let surplus = tot_inv - exp_bal;
        println!("{}, {}", asset_id, surplus);
        if surplus > 0.000000001 {
            println!("{}: {}", asset_id, tot_inv - exp_bal);
            panic!("");
        }
    }
    
    initial_inventory
}

pub fn save_USD_prices() {

    let deltas = deltas::Deltas::load("./data/2024/unlinked_deltas.json").unwrap();


    let used_assets = {
        let onchain_names = deltas.used_assets();
        let tax_names = symbols::batch_onchain_to_tax_ticker(&onchain_names);
        tax_names

    };
    dbg!(&used_assets);
    // used_assets.push("REP".to_string());
    // // let prices = prices::Prices::load_dir("/home/dwc/code/coingecko/2024/day_open/USD", &used_assets).unwrap();
    let mut prices = prices::Prices::load_dir("/home/dwc/code/crypto_compare/2024/day_hourvwap/USD", &used_assets).unwrap();
    // // let mut prices = prices::Prices::load_dir_candles("/home/dwc/code/coinbase/candles/2024/900", "USD", &used_assets).unwrap();
    // dbg!(&prices);
    let other_prices = prices::Prices::load_dir("/home/dwc/code/coingecko/2024/day_close/USD", &used_assets).unwrap();
    prices.patch(&other_prices, Utc.ymd(2024,1,1).and_hms(0,0,0), Utc.ymd(2024,4,1).and_hms(0,0,0));
    prices.save("./data/2024/prices_USD.json");
}

pub fn save_linked_deltas() {
    let mut deltas = deltas::Deltas::load("./data/2024/unlinked_deltas.json").unwrap();
    deltas.link_airdrop_components(); 
    deltas.link_add_liquidity_v3(); 
    {
        use crate::deltas::Ilk;
        use crate::deltas::Direction;
        for delta in &deltas.0 {
            if delta.ilk == Ilk::ManageLiquidity && delta.direction == Direction::In && delta.asset.starts_with("UNI-V3-LIQUIDITY") {
                if delta.linked_to.len() != 1 {
                    dbg!(delta);
                }

            }
            if delta.ilk == Ilk::ManageLiquidity && delta.direction == Direction::Out && !delta.asset.starts_with("UNI-V3-LIQUIDITY"){
                if delta.linked_to.len() != 1 {
                    dbg!(delta);
                }

            }
            // if delta.ilk == Ilk::ManageLiquidityGas && delta.direction == Direction::Out {
            //     if delta.linked_to.len() != 1 {
            //         dbg!(delta);
            //     }

            // }
        }
    }

    deltas.link_remove_liquidity_v3(); 
    {
        use crate::deltas::Ilk;
        use crate::deltas::Direction;
        for delta in &deltas.0 {
            if delta.ilk == Ilk::ManageLiquidity && delta.direction == Direction::Out && delta.asset.starts_with("UNI-V3-LIQUIDITY") {
                if delta.linked_to.len() == 0 || delta.linked_to.len() > 2 {
                    dbg!(delta);
                }

            }
            if delta.ilk == Ilk::ManageLiquidity && delta.direction == Direction::In && !delta.asset.starts_with("UNI-V3-LIQUIDITY"){
                if delta.linked_to.len() != 1 {
                    dbg!(delta);
                }

            }
            // if delta.ilk == Ilk::ManageLiquidityGas && delta.direction == Direction::Out {
            //     if delta.linked_to.len() != 1 {
            //         dbg!(delta);
            //     }

            // }
        }
    }
    deltas.link_manage_liquidity_gas_v3(); 
    {
        use crate::deltas::Ilk;
        use crate::deltas::Direction;
        for delta in &deltas.0 {
            if delta.ilk == Ilk::ManageLiquidityGas && delta.direction == Direction::Out {
                if delta.linked_to.len() != 1 {
                    dbg!(delta);
                }

            }
            // if delta.ilk == Ilk::ManageLiquidity && delta.direction == Direction::In && !delta.asset.starts_with("UNI-V3-LIQUIDITY"){
            //     if delta.linked_to.len() != 1 {
            //         dbg!(delta);
            //     }

            // }
            // if delta.ilk == Ilk::ManageLiquidityGas && delta.direction == Direction::Out {
            //     if delta.linked_to.len() != 1 {
            //         dbg!(delta);
            //     }

            // }
        }
    }

    deltas.link_swap_components(); 
    deltas.link_trade_components();
    deltas.link_conversion_components();
    deltas.link_swap_fail_gas(std::time::Duration::from_secs(7*24*3600));
    deltas.link_manage_liquidity_fail_gas(std::time::Duration::from_secs(7*24*3600));
    deltas.link_tx_cancel(std::time::Duration::from_secs(7*24*3600));
    deltas.save("./data/2024/linked_deltas.json").unwrap();
    check_linked_deltas();
}

pub fn check_linked_deltas() {
    let deltas = deltas::Deltas::load("./data/2024/linked_deltas.json").unwrap();

    for delta in &deltas.0 {
        if delta.ilk == deltas::Ilk::TradeFee {
            assert!(delta.linked_to.len() == 1);
        }
        if delta.ilk == deltas::Ilk::SwapGas {
            assert!(delta.linked_to.len() == 1);
        }
        if delta.ilk == deltas::Ilk::SwapFailGas {
            assert!(delta.linked_to.len() < 2 );
        }
        if delta.ilk == deltas::Ilk::ManageLiquidityGas {
            if delta.linked_to.len() < 1 {
                println!("{}", delta.linked_to.len());
                for index in &delta.linked_to {
                    dbg!(&deltas.0[*index]);
                }
                println!("");
            }
            // assert!(delta.linked_to.len() == 1);
        }
        if delta.ilk == deltas::Ilk::ManageLiquidityFailGas {
            
            assert!(delta.linked_to.len() < 2);
        }
    }
    acquisitions_that_need_link(&deltas);
    deltas.disposition_links();
}

fn is_aquisition_that_needs_link(delta: &deltas::Delta) -> bool {
    if (
        delta.direction == deltas::Direction::In 
        && delta.ilk != deltas::Ilk::WrapEth 
        && delta.ilk != deltas::Ilk::UnwrapEth 
        && delta.ilk != deltas::Ilk::SwapFees 
        // && delta.ilk != deltas::Ilk::ChangeMakerVault 
        && delta.ilk != deltas::Ilk::DepositDiscrepancy 
        && delta.ilk != deltas::Ilk::BridgeFeeRefund 
        && !(delta.ilk == deltas::Ilk::Airdrop && &delta.asset == "OP")
        && delta.ilk != deltas::Ilk::WalletDiscovery 
        && delta.ilk != deltas::Ilk::CoinbaseInterest 
        && delta.ilk != deltas::Ilk::Loan 
        && delta.ilk != deltas::Ilk::PhishingAttempt 
        ) {
        // dbg!(&delta);
        true
    } else {
        false
    }
}


fn acquisitions_that_need_link(deltas: &deltas::Deltas) {


    let mut total = 0;
    let mut unlinked = 0;
    for delta in &deltas.0 {
        if is_aquisition_that_needs_link(delta) {
            total += 1;
            if delta.linked_to.len() == 0 {
                println!("needs link: {:#?}", delta); 
                unlinked += 1;

            }
        }
    }
    println!("required: {} unlinked of {} needed", unlinked, total);
}


pub fn check_end_inventory() {

    let end_balances = {
        let data = std::fs::read_to_string("./data/2024/end_holdings.json").unwrap();
        let ib: HashMap<String, f64> = serde_json::from_str(&data).unwrap();
        ib
    };

    let mut end_inventory_us = inventory::Inventory::load("./data/2024/end_inventory_us.json").unwrap();


    for (asset_id, acq_vec) in &end_inventory_us.0 {
        let tot_inv = {
            let mut ti = 0.0;
            for acq in acq_vec {
                ti += acq.qty;
            }
            ti
        };
        let exp_bal = if asset_id == "ETH" {
            end_balances["ETH"] + end_balances["WETH"]
        } else if asset_id == "BTC" {
            end_balances["BTC"] + end_balances["WBTC"]
        } else if asset_id == "REP" {
            end_balances["REP"] + end_balances["REPv2"]
        } else if asset_id == "USDC" {
            end_balances["USDC"] + end_balances["USDC.ARBITRUM"]
        } else if asset_id == "SOL" {
            end_balances["WSOL"]
        // } else if asset_id.starts_with("UNI-V3-LIQUIDITY") {
        //     if acq_vec.len() > most_acqs.len() {
        //         most_acqs = acq_vec.clone();
        //     }
        //     for acq in acq_vec {
        //         if acq.qty > biggest_qty {
        //             dbg!(&acq.qty);
        //             biggest_qty = acq.qty;
        //         }
        //     }
        //     positions += 1;
        //     continue
        } else {
            dbg!(asset_id);
            end_balances[asset_id]

        };
        let surplus = tot_inv - exp_bal;
        println!("{}, {}", asset_id, surplus);

        if asset_id.starts_with("UNI-V3-LIQUIDITY") {

            if surplus > 1024.0 {
                println!("{}: tot_inv: {}, exp_bal: {}", asset_id, tot_inv, exp_bal);
                panic!("");
            }

        } else {
            if surplus > 0.000000001 {
                println!("{}: tot_inv: {}, exp_bal: {}", asset_id, tot_inv, exp_bal);
                panic!("");
            }
        }
    }
    
}
