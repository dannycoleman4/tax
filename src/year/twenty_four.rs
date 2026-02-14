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
    let mut linked = deltas::LinkedDeltas::load("./data/2024/linked_deltas.json").unwrap();
    linked.reassign_quote_fee_links("USD");

    let (summary, dispositions) = inventory.apply_deltas(&linked, "USD", &prices, method);

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
    // dbg!(&used_assets);
    // used_assets.push("REP".to_string());
    // // let prices = prices::Prices::load_dir("/home/dwc/code/coingecko/2024/day_open/USD", &used_assets).unwrap();
    let mut prices = prices::Prices::load_dir("/home/dwc/code/crypto_compare/2024/day_hourvwap/USD", &used_assets).unwrap();
    // // let mut prices = prices::Prices::load_dir_candles("/home/dwc/code/coinbase/candles/2024/900", "USD", &used_assets).unwrap();
    // dbg!(&prices.map.keys());
    // panic!();
    let other_prices = prices::Prices::load_dir("/home/dwc/code/coingecko/2024/day_close/USD", &used_assets).unwrap();
    prices.patch(&other_prices, Utc.ymd(2024,1,1).and_hms(0,0,0), Utc.ymd(2025,1,1).and_hms(0,0,0));
    prices.save("./data/2024/prices_USD.json");
}

pub fn save_linked_deltas() {
    let deltas = deltas::Deltas::load("./data/2024/unlinked_deltas.json").unwrap();
    let linked = deltas.link();
    linked.save("./data/2024/linked_deltas.json").unwrap();
    check_linked_deltas();
}

pub fn check_linked_deltas() {
    let linked = deltas::LinkedDeltas::load("./data/2024/linked_deltas.json").unwrap();
    acquisitions_that_need_link(&linked);
    linked.disposition_links();
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
        && !(delta.ilk == deltas::Ilk::Airdrop && &delta.asset == "ARB")
        && delta.ilk != deltas::Ilk::WalletDiscovery
        && delta.ilk != deltas::Ilk::CoinbaseInterest
        && delta.ilk != deltas::Ilk::Loan
        && delta.ilk != deltas::Ilk::PhishingAttempt
        && delta.ilk != deltas::Ilk::StakingYield
        && delta.ilk != deltas::Ilk::CoinbaseDiscovery
        ) {
        // dbg!(&delta);
        true
    } else {
        false
    }
}


fn acquisitions_that_need_link(linked: &deltas::LinkedDeltas) {

    let mut total = 0;
    let mut unlinked = 0;
    for group in &linked.0 {
        for delta in &group.ins {
            if is_aquisition_that_needs_link(delta) {
                total += 1;
                if group.outs.is_empty() && group.ins.len() == 1 {
                    println!("needs link: {:#?}", delta);
                    unlinked += 1;
                }
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
        } else {
            // dbg!(asset_id);
            end_balances[asset_id]

        };
        let surplus = tot_inv - exp_bal;
        println!("{}, {}", asset_id, surplus);

        if deltas::is_uni_cl_position(asset_id) {

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
