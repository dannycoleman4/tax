
use crate::deltas;
use crate::prices;
use crate::inventory;
use crate::symbols;
use std::collections::HashMap;
use chrono::{Utc, TimeZone};



pub fn calculate(method: inventory::InventoryMethod) {
    let mut inventory = load_initial_inventory_us();
    inventory.add_asset("DYDX");
    inventory.add_asset("OP");
    inventory.add_asset("ETHW");
    // let mut inventory = inventory::Inventory::load("./2022/initial_inventory_us.json").unwrap();
    let prices = prices::Prices::load("./data/2022/prices_USD.json").unwrap();
    let mut deltas = deltas::Deltas::load("./data/2022/linked_deltas.json").unwrap();
    deltas.reassign_quote_fee_links("USD");

    let (summary, dispositions) = inventory.apply_deltas(&deltas, "USD", &prices, method);
    
    // summary.save("./2022/summary_us.json");
    inventory.save("./data/2022/end_inventory_us.json");

    check_end_inventory();


    let mut report = String::new();
    report += "\n";
    report += "all values in USD\n";
    report += "day average (hourly vwap) prices from cryptocompare.com used to determine fair market value\n"; 
    report += "\n";

    report += "2022 cryptocurrency income (\"airdrops\"):\n";
    report += &format!(" income: {:.8}\n", summary.income);
    report += "\n";
    report += "2022 cryptocurrency capital_gains:\n";
    report += &format!(" inventory method: {:.8}\n", summary.inventory_method);
    report += &format!(" short term capital gains: {:.8}\n", summary.short_term_capital_gains);
    report += &format!(" long term capital gains: {:.8}\n", summary.long_term_capital_gains);
    report += "\n";
    println!("{}", report);

    let fp = "./data/2022/all_dispositions_us.csv";
    std::fs::write(fp, dispositions);

    let fp = "./data/2022/capital_gains_report_us.txt";
    std::fs::write(fp, report);

}


pub fn load_initial_inventory_us() -> inventory::Inventory {

    let initial_balances = {
        let data = std::fs::read_to_string("./data/2022/initial_balances.json").unwrap();
        let ib: HashMap<String, f64> = serde_json::from_str(&data).unwrap();
        ib
    };

    let mut initial_inventory = inventory::Inventory::load("./data/2021/end_inventory_us.json").unwrap();


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

pub fn check_end_inventory() {

    let initial_balances = {
        let data = std::fs::read_to_string("./data/2022/end_balances.json").unwrap();
        let ib: HashMap<String, f64> = serde_json::from_str(&data).unwrap();
        ib
    };

    let mut end_inventory_us = inventory::Inventory::load("./data/2022/end_inventory_us.json").unwrap();


    for (asset_id, acq_vec) in &end_inventory_us.0 {
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
        } else {
            initial_balances[asset_id]
        };
        let surplus = tot_inv - exp_bal;
        println!("{}, {}", asset_id, surplus);
        if surplus > 0.000000001 {
            println!("{}: tot_inv: {}, exp_bal: {}", asset_id, tot_inv, exp_bal);
            panic!("");
        }
    }
    
}

pub fn save_USD_prices() {
    let deltas = deltas::Deltas::load("./data/2022/unlinked_deltas.json").unwrap();


    let used_assets = deltas.used_assets();
    println!("used_assets: {:?}", used_assets);
    // panic!("");
    // let prices = prices::Prices::load_dir("/home/dwc/code/coingecko/2022/day_open/USD", &used_assets).unwrap();
    let mut prices = prices::Prices::load_dir("/home/dwc/code/crypto_compare/2022/day_hourvwap/USD", &used_assets).unwrap();
    // let mut prices = prices::Prices::load_dir_candles("/home/dwc/code/coinbase/candles/2022/900", "USD", &used_assets).unwrap();
    let other_prices = prices::Prices::load_dir("/home/dwc/code/coingecko/2022/day_close/USD", &used_assets).unwrap();
    prices.patch(&other_prices, Utc.ymd(2022,1,1).and_hms(0,0,0), Utc.ymd(2022,11,13).and_hms(0,0,0));
    prices.save("./data/2022/prices_USD.json");
}


pub fn save_linked_deltas() {
    let mut deltas = deltas::Deltas::load("./data/2022/unlinked_deltas.json").unwrap();
    deltas.link_airdrop_components(); 
    deltas.link_swap_components(); 
    // deltas.link_miner_direct_payment();
    deltas.link_trade_components();
    deltas.link_conversion_components();
    //ndeltas.link_remove_liquidity_components();
    // deltas.link_dydx_deposits_and_withdraws();
    deltas.link_swap_fail_gas(std::time::Duration::from_secs(7*24*3600));
    // deltas.link_tx_cancel(std::time::Duration::from_secs(7*24*3600));
    deltas.save("./data/2022/linked_deltas.json").unwrap();
    check_linked_deltas();
}

pub fn check_linked_deltas() {
    let deltas = deltas::Deltas::load("./data/2022/linked_deltas.json").unwrap();

    for delta in &deltas.0 {
        if delta.ilk == deltas::Ilk::TradeFee || delta.ilk == deltas::Ilk::SwapGas || delta.ilk == deltas::Ilk::SwapFailGas {
            assert!(delta.linked_to.len() < 2);
        }
    }
    acquisitions_that_need_link(&deltas);
    deltas.disposition_links();
}


fn is_trade_fee_rebate(delta: &deltas::Delta) -> bool {
    delta.ilk == deltas::Ilk::TradeFee 
        && delta.direction == deltas::Direction::In 
        && delta.host == deltas::Host::FtxUs
}

fn is_aquisition_that_needs_link(delta: &deltas::Delta) -> bool {
    if (
        delta.direction == deltas::Direction::In 
        && delta.ilk != deltas::Ilk::WrapEth 
        && delta.ilk != deltas::Ilk::UnwrapEth 
        // && delta.ilk != deltas::Ilk::TokenMigration 
        && delta.ilk != deltas::Ilk::ChangeMakerVault 
        && delta.ilk != deltas::Ilk::DepositDiscrepancy 
        && delta.ilk != deltas::Ilk::BridgeFeeRefund 
        && !is_trade_fee_rebate(delta)
        && &delta.identifier != "Ftxus_2022Q3_inferred_credit_1"
        ) {
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
