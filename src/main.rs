mod deltas;
mod inventory;
mod prices;
mod symbols;
mod year;
use chrono::{TimeZone, Utc};
use std::collections::HashMap;


fn main() {

    // let words_97 = {
    //     let data = std::fs::read_to_string("./2021/scrap/grt_0.97.csv").unwrap();
    //     let lines: Vec<&str> = data.split("\n").collect();
    //     let mut words = Vec::new();
    //     for line in &lines {
    //         let ws: Vec<String> = line.split(",").map(|x| x.to_string()).collect();
    //         words.push(ws)
    //     }
    //     words
    // };
    // let words_100 = {
    //     let data = std::fs::read_to_string("./2021/scrap/grt_1.00.csv").unwrap();
    //     let lines: Vec<&str> = data.trim().split("\n").collect();
    //     let mut words = Vec::new();
    //     for line in &lines {
    //         let ws: Vec<String> = line.split(",").map(|x| x.to_string()).collect();
    //         words.push(ws)
    //     }
    //     words
    // };

    // for index in 1..100 {
    //     
    //     let proceeds_97: f64 = words_97[index][4].parse().unwrap();
    //     let cost_basis_97: f64 = words_97[index][5].parse().unwrap();
    //     let gain_97: f64 = words_97[index][6].parse().unwrap();
    //     let proceeds_100: f64 = words_100[index][4].parse().unwrap();
    //     let cost_basis_100: f64 = words_100[index][5].parse().unwrap();
    //     let gain_100: f64 = words_100[index][6].parse().unwrap();

    //     println!("{} {} {} {}", words_97[index][2], proceeds_97 / proceeds_100, cost_basis_97 / cost_basis_100, gain_97 / gain_100);
    // }

    // let data2 = std::fs::read_to_string("./scrap/grt_1.00.csv").unwrap();
    
    // let prices = prices::Prices::load_dir("/home/dwc/code/coingecko/2021/day_open/USD", &vec!["ETH".to_string(), "ENS".to_string()]).unwrap();

    // let price = prices.price_at_rfc3339("ENS", "2021-01-02T00:00:00Z");
    // println!("{}", price);



    // year::twenty::save_USD_prices();
    // year::twenty::save_initial_inventory_us();
    // year::twenty::save_linked_deltas();
    // year::twenty::check_linked_deltas();
    // year::twenty::calculate_us(inventory::InventoryMethod::Lifo);
    // year::twenty::save_CAD_prices();
    // year::twenty::save_initial_inventory_canada();
    // println!("wef");
    // year::twenty::calculate_canada();



    // year::twenty_one::load_initial_inventory_us();
    // year::twenty_one::save_USD_prices();
    // year::twenty_one::check_linked_deltas();
    // year::twenty_one::save_linked_deltas();
    // year::twenty_one::calculate(inventory::InventoryMethod::Lifo);


    // year::twenty_two::load_initial_inventory_us();
    // year::twenty_two::save_USD_prices();
    // year::twenty_two::save_linked_deltas();
    // year::twenty_two::check_linked_deltas();
    // year::twenty_two::calculate(inventory::InventoryMethod::Fifo);
    // year::twenty_two::check_end_inventory();
    

    // let inventory = year::twenty_three::load_initial_inventory_us();

    // for (key, val) in inventory.0.iter() {
    //     dbg!(key, val.len());
    // }
    // dbg!(inventory);

    // year::twenty_three::save_USD_prices();
    // year::twenty_three::save_linked_deltas();
    // year::twenty_three::check_linked_deltas();
    // year::twenty_three::calculate(inventory::InventoryMethod::Lifo);
    // year::twenty_three::check_end_inventory();
    //

    // year::twenty_four::save_USD_prices();
    // year::twenty_four::save_linked_deltas();
    // year::twenty_four::check_linked_deltas();
    year::twenty_four::calculate(inventory::InventoryMethod::Lifo);
    // year::twenty_four::check_end_inventory();


    // let deltas = deltas::Deltas::load("./data/2023/linked_deltas.json").unwrap();
    // let prices = prices::Prices::load("./data/2023/prices_USD.json").unwrap();

    // let mut sum1 = 0.0;
    // let mut sum2 = 0.0;
    // let mut smallest = u64::MAX as f64;

    // let mut smallet_by_pair: HashMap<String, f64> = HashMap::new();
    // for (i, d) in deltas.0.iter().enumerate() {
    //     // if d.asset == "UNI-V3-LIQUIDITY:494643_WETH_ARB_500_73280_73340" {
    //     //     if d.direction == deltas::Direction::In {
    //     //         let cost = deltas.index_cost(i, "USD", &prices);
    //     //         dbg!(cost);
    //     //         
    //     //     }
    //     //     dbg!(d);
    //     // }
    //     // if d.ilk == deltas::Ilk::WithdrawalToBank {
    //     //     withdrawn += d.qty;
    //     // }
    //     // if d.ilk == deltas::Ilk::Airdrop {
    //     //     sum1 += deltas.index_cost(i, "USD", &prices);
    //     //     sum2 += d.value("USD", &prices);
    //     // }
    //     if d.asset.starts_with("UNI-V3-LIQUIDITY") {

    //         let sym = {
    //             let split: Vec<&str> = d.asset.split("_").collect();
    //             let sym = format!("{}-{}", split[1], split[2]);
    //             // dbg!(sym);
    //             sym

    //         };

    //         if smallet_by_pair.contains_key(&sym) {
    //             if d.qty < smallet_by_pair[&sym] {
    //                 *smallet_by_pair.get_mut(&sym).unwrap() = d.qty;
    //             }
    //         } else {
    //             smallet_by_pair.insert(sym, d.qty);
    //         };

    //         // if d.qty < smallest {
    //         //     dbg!(d);
    //         //     smallest = d.qty;
    //         // }
    //     }
    // }
    // dbg!(sum1);
    // dbg!(sum2);
    // dbg!(smallet_by_pair);

    // for d in &deltas.0 {
    //     if d.asset.starts_with("UNI-V1") || d.asset == "GTC" {
    //         continue
    //     }
    //     let v = d.value("USD", &prices);
    //     if v == 0.0 && d.ilk != deltas::Ilk::TradeFee && d.ilk != deltas::Ilk::SwapGas{
    //         println!("{:#?}", d);
    //     }
    //     let v = d.value("USD", &prices);
    //     if v == 0.0 && d.ilk != deltas::Ilk::TradeFee && d.ilk != deltas::Ilk::SwapGas{
    //         println!("{:#?}", d);
    //     }
    // }


    // let deltas = deltas::Deltas::load("./data/2024/unlinked_deltas.json").unwrap();
    // let used_assets = deltas.used_assets();
    // dbg!("{:?}", used_assets);

    // let ii = year::twenty_one::load_initial_inventory_us();

    // year::twenty_one::save_linked_deltas();
    // year::twenty_one::check_linked_deltas();
 

    // year::twenty::save_CAD_prices();
    // year::twenty::save_initial_inventory_canada();
    //



    // let deltas = deltas::Deltas::load("./2021/linked_deltas.json").unwrap();
    // let mut counter = 0;
    // let mut first = 0;
    // let mut last = 0;
    // for d in &deltas.0 {

    //     if d.timestamp > 1612962030657 - 360000 && d.timestamp < 1612962030657 + 360000 {
    //         println!("{:#?}", d);
    //     }

    //     // if counter % 100 == 0 && d.linked_to.len() == 0 {

    //     //     println!("{:#?}", d);
    //     // }
    //     // if d.direction == deltas::Direction::In && d.linked_to.len() == 1 && d.host == deltas::Host::CoinbasePro && d.ilk == deltas::Ilk::Match{
    //     //     counter += 1; 

    //     //     if first == 0 {
    //     //         first = d.timestamp;
    //     //     }

    //     //     last = d.timestamp
    //     // }
    //     // if d.direction == deltas::Direction::Out && d.linked_to.len() == 0 {
    //     //     break
    //     // }
    // }
    // println!("first: {}, last: {}, number: {}", first, last, counter);

}

