use crate::deltas;
use crate::prices;
use crate::inventory;
use crate::symbols;
use std::collections::HashMap;
use chrono::{Utc, TimeZone};

const MILLIS_YEAR: u64 = 31557600000;




pub fn calculate(method: inventory::InventoryMethod) {
    let mut inventory = load_initial_inventory_us();
    // let mut inventory = inventory::Inventory::load("./2021/initial_inventory_us.json").unwrap();
    let prices = prices::Prices::load("./2021/prices_USD.json").unwrap();
    let mut linked = deltas::LinkedDeltas::load("./2021/linked_deltas.json").unwrap();
    linked.reassign_quote_fee_links("USD");

    let (summary, dispositions) = inventory.apply_deltas(&linked, "USD", &prices, method);

    // summary.save("./2021/summary_us.json");
    inventory.save("./2021/end_inventory_us.json");


    let mut report = String::new();
    report += "\n";
    report += "all values in USD\n";
    report += "day average (hourly vwap) prices from cryptocompare.com used to determine fair market value\n";
    report += "\n";

    report += "2021 cryptocurrency income (\"airdrops\"):\n";
    report += &format!(" income: {:.8}\n", summary.income);
    report += "\n";
    report += "2021 cryptocurrency capital_gains:\n";
    report += &format!(" inventory method: {:.8}\n", summary.inventory_method);
    report += &format!(" short term capital gains: {:.8}\n", summary.short_term_capital_gains);
    report += &format!(" long term capital gains: {:.8}\n", summary.long_term_capital_gains);
    report += "\n";
    println!("{}", report);

    let fp = "./2021/all_dispositions_us.csv";
    std::fs::write(fp, dispositions);

    let fp = "./2021/capital_gains_report_us.txt";
    std::fs::write(fp, report);

}

pub fn load_initial_inventory_us() -> inventory::Inventory {

    let initial_balances = {
        let data = std::fs::read_to_string("./2021/initial_balances.json").unwrap();
        let ib: HashMap<String, f64> = serde_json::from_str(&data).unwrap();
        ib
    };

    let mut initial_inventory = inventory::Inventory::load("./2020/end_inventory_us.json").unwrap();


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


pub fn save_linked_deltas() {
    let deltas = deltas::Deltas::load("./2021/unlinked_deltas.json").unwrap();
    let linked = deltas.link();
    linked.save("./2021/linked_deltas.json").unwrap();
    check_linked_deltas();
}

pub fn check_linked_deltas() {
    let linked = deltas::LinkedDeltas::load("./2021/linked_deltas.json").unwrap();
    acquisitions_that_need_link(&linked);
    linked.disposition_links();
}


fn is_aquisition_that_needs_link(delta: &deltas::Delta) -> bool {
    if (
        delta.direction == deltas::Direction::In &&
        delta.ilk != deltas::Ilk::WrapEth &&
        delta.ilk != deltas::Ilk::UnwrapEth &&
        delta.ilk != deltas::Ilk::TokenMigration &&
        delta.ilk != deltas::Ilk::ChangeMakerVault &&
        delta.ilk != deltas::Ilk::DepositDiscrepancy
        ) {
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
                    // println!("needs link: {:#?}", delta);
                    unlinked += 1;
                }
            }
        }
    }
    println!("{} unlinked of {} total", unlinked, total);
}

pub fn save_USD_prices() {
    let deltas = deltas::Deltas::load("./2021/unlinked_deltas.json").unwrap();
    let used_assets = deltas.used_assets();
    // let prices = prices::Prices::load_dir("/home/dwc/code/coingecko/2021/day_open/USD", &used_assets).unwrap();
    let prices = prices::Prices::load_dir("/home/dwc/code/crypto_compare/2021/day_hourvwap/USD", &used_assets).unwrap();
    // let mut prices = prices::Prices::load_dir_candles("/home/dwc/code/coinbase/candles/2021/900", "USD", &used_assets).unwrap();
    // let other_prices = prices::Prices::load_dir("/home/dwc/code/coingecko/2021/day_open/USD", &used_assets).unwrap();
    // prices.patch(&other_prices, Utc.ymd(2021,1,1).and_hms(0,0,0), Utc.ymd(2022,1,1).and_hms(0,0,0));
    prices.save("./2021/prices_USD.json");
}
