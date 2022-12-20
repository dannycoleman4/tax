use crate::deltas;
use crate::prices;
use crate::inventory;
use crate::symbols;
use std::collections::HashMap;
use chrono::{Utc, TimeZone};
// use std::io::Write;


pub fn save_linked_deltas() {
    let mut deltas = deltas::Deltas::load("./2020/unlinked_deltas.json").unwrap();
    deltas.link_airdrop_components(); 
    deltas.link_swap_components(); 
    deltas.link_trade_components();
    deltas.link_unused_kucoin_fees_within(1);
    deltas.link_unused_kucoin_fees_within(5);
    deltas.link_unused_kucoin_fees_within(10);
    deltas.link_unused_kucoin_fees_within(60);
    deltas.link_conversion_components();
    deltas.link_remove_liquidity_components();
    deltas.link_swap_fail_gas(std::time::Duration::from_secs(24*3600));
    deltas.save("./2020/linked_deltas.json").unwrap();
    check_linked_deltas();
}

pub fn check_linked_deltas() {
    let mut deltas = deltas::Deltas::load("./2020/linked_deltas.json").unwrap();
    acquisitions_that_need_link(&deltas);
    for delta in &deltas.0 {
        if delta.ilk == deltas::Ilk::TradeFee || delta.ilk == deltas::Ilk::SwapGas || delta.ilk == deltas::Ilk::SwapFailGas {
            assert!(delta.linked_to.len() < 2);
        }
    }
    deltas.disposition_links();
}


pub fn save_USD_prices() {
    let mut deltas = deltas::Deltas::load("./2020/unlinked_deltas.json").unwrap();
    let used_assets = deltas.used_assets();
    // let mut prices = prices::Prices::load_dir("/home/dwc/code/coingecko/2020/day_open/USD", &used_assets).unwrap();
    // let other_prices = prices::Prices::load_dir("/home/dwc/code/crypto_compare/2020/day_hourvwap/USD", &used_assets).unwrap();
    // prices.patch(&other_prices, Utc.ymd(2020,01,01).and_hms(0,0,0), Utc.ymd(2021,01,01).and_hms(0,0,0));
    let prices = prices::Prices::load_dir("/home/dwc/code/crypto_compare/2020/day_hourvwap/USD", &used_assets).unwrap();
    prices.save("./2020/prices_USD.json");

}

pub fn save_initial_inventory_us() {
    let initial_balances = {
        let data = std::fs::read_to_string("./2020/initial_balances.json").unwrap();
        let ib: HashMap<String, f64> = serde_json::from_str(&data).unwrap();
        ib
    };

    let ts = Utc.ymd(2020, 01, 01).and_hms(0,0,0).timestamp_millis();
    let mut acquisitions = inventory::Inventory::initiate_zero_cost(&initial_balances, ts as u64); 
    acquisitions.consolidate_alias("BTC", "WBTC");
    acquisitions.consolidate_alias("ETH", "WETH");
    acquisitions.consolidate_alias("REP", "REPv2");
    acquisitions.save("./2020/initial_inventory_us.json").unwrap();
}


pub fn calculate_us(method: inventory::InventoryMethod) {
    let mut inventory = inventory::Inventory::load("./2020/initial_inventory_us.json").unwrap();
    let prices = prices::Prices::load("./2020/prices_USD.json").unwrap();
    let mut deltas = deltas::Deltas::load("./2020/linked_deltas.json").unwrap();
    deltas.reassign_quote_fee_links("USD");

    let (summary, dispositions) = inventory.apply_deltas(&deltas, "USD", &prices, method);
    
    // summary.save("./2020/summary_us.json");
    inventory.save("./2020/end_inventory_us.json");


    let mut report = String::new();
    report += "\n";
    report += "all values in USD\n";
    report += "day average prices from cryptocompare.com used to determine fair market value\n"; 
    report += "\n";

    report += "2020 cryptocurrency income (\"airdrops\"):\n";
    report += &format!(" income: {:.8}\n", summary.income);
    report += "\n";
    report += "2020 cryptocurrency capital_gains:\n";
    report += &format!(" inventory method: {:.8}\n", summary.inventory_method);
    report += &format!(" short term capital gains: {:.8}\n", summary.short_term_capital_gains);
    report += &format!(" long term capital gains: {:.8}\n", summary.long_term_capital_gains);
    report += "\n";
    println!("{}", report);

    let fp = "./2020/all_dispositions_us.csv";
    std::fs::write(fp, dispositions);

    let fp = "./2020/capital_gains_report_us.txt";
    std::fs::write(fp, report);

}


fn is_aquisition_that_needs_link(delta: &deltas::Delta) -> bool {
    if (
        delta.direction == deltas::Direction::In &&
        delta.ilk != deltas::Ilk::WrapEth &&
        delta.ilk != deltas::Ilk::UnwrapEth &&
        delta.ilk != deltas::Ilk::TokenMigration &&
        &delta.identifier != "0x9d003bf5bb78764523db802d1ced8863dc9962825dee08440a60cedeb5b99902" &&
        &delta.identifier != "0xa728a2c42874f59671722dfbcae33e499ec213f1f516aef86fd3f7e2f965e1b6" &&
        delta.ilk != deltas::Ilk::DepositDiscrepancy
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
                println!("{:#?}", delta); 
                unlinked += 1;
            }
        }
    }
    println!("{} unlinked of {} total", unlinked, total);
}



pub fn save_CAD_prices() {
    let mut deltas = deltas::Deltas::load("./2020/unlinked_deltas.json").unwrap();
    let used_assets = deltas.used_assets();
    let prices = prices::Prices::load_dir("/home/dwc/code/crypto_compare/2020/day_hourvwap/CAD", &used_assets).unwrap();
    prices.save("./2020/prices_CAD.json");
}

pub fn save_initial_inventory_canada() {
    let initial_balances = {
        let data = std::fs::read_to_string("./2020/initial_balances.json").unwrap();
        let ib: HashMap<String, f64> = serde_json::from_str(&data).unwrap();
        ib
    };

    let mut holdings = inventory::ConsolidatedInventory::initiate_zero_cost(&initial_balances); 
    holdings.consolidate_alias("BTC", "WBTC");
    holdings.consolidate_alias("ETH", "WETH");
    holdings.consolidate_alias("REP", "REPv2");
    holdings.save("./2020/initial_inventory_canada.json").unwrap();
}

pub fn calculate_canada() {

    let ts = Utc.ymd(2020,11,01).and_hms(0,0,0).timestamp_millis() as u64;
    
    let mut holdings = inventory::ConsolidatedInventory::load("./2020/initial_inventory_canada.json").unwrap();
    let prices = prices::Prices::load("./2020/prices_CAD.json").unwrap();
    let day_close_prices = prices::Prices::load("/home/dwc/code/crypto_compare/2020/day_close/CAD/2020-10-31UTC.json").unwrap();

    let deltas = {
        let mut filtered = Vec::new();
        let all = deltas::Deltas::load("./2020/linked_deltas.json").unwrap();

        println!("all: {}", all.0.len());

        for d in &all.0 {
            if d.timestamp < ts {
                filtered.push(d.clone())
            }
        }
        println!("filtered: {}", filtered.len());
        deltas::Deltas ( filtered )
    };


    let (summary, disps) = holdings.apply_deltas(&deltas, "CAD", &prices);
    println!("");

    let mut report = String::new();

    report += "\n";
    report += "all values in CAD\n";
    report += "day average prices from cryptocompare.com used for income\n"; 
    report += "day average prices from cryptocompare.com used for capital gains prior to deemed dispositions\n";
    report += "day close prices for 2020-10-31 from cryptocompare.com used for value at deemed dispositions\n";
    report += "\n";



    report += "2020-01-01 to 2020-10-31 cryptocurrency income (\"airdrops\"):\n";
    report += &format!(" income: {:.8}\n", summary.income);
    report += "\n";

    report += "2020-01-01 to 2020-10-31 capital gains (not including deemed dispositions):\n";
    report += &format!(" capital gains: {:.8}\n", summary.capital_gains);
    report += "\n";

    report += &format!("holdings on 2021-10-31 EOD:\n");

    let mut total_cost = 0.0;
    let mut total_value = 0.0;
    for (asset, holding) in &holdings.0 {
        if asset.starts_with("UNI-V1:") || holding.qty < 0.00000001 || asset == "USD"{ 
            // println!("skipped: {}", asset);
            continue
        }

        report += &format!( " {}:\n", asset);
        report += &format!( "  balance: {:.8}\n", holding.qty);
        report += &format!( "  cost basis: {:.8}\n", holding.cost);
        if day_close_prices.map.contains_key(asset) {
            let v = holding.qty * day_close_prices.price_at_millis(asset, ts-1);
            report += &format!( "  market value: {:.8}\n", v);
            total_cost += holding.cost;
            total_value += v;
        } else {
            println!("no key: {}", asset);
            panic!("")
        }
    }

    report += "\n";
    report += " ALL:\n";
    report += &format!("  cost basis: {:.8}\n", total_cost);
    report += &format!("  market value: {:.8}\n", total_value);
    report += &format!("  capital gains: {:.8}\n", total_value - total_cost);

    report += "\n";
    report += "summary:\n";
    report += &format!(" cryptocurrency income: {:.8}\n", summary.income);
    report += &format!(" capital gains (including deemed dispositions): {:.8}\n", summary.capital_gains + (total_value - total_cost));

    let fp = "./2020/all_dispositions_canada.csv";
    std::fs::write(fp, disps);

    std::fs::write("./2020/capital_gains_report_canada.txt", &report).unwrap();
    println!("{}", report)
   // }

}
