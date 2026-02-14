use serde::{Serialize, Deserialize};
use std::collections::HashMap;
use std::error::Error;
use crate::symbols;
use crate::deltas;
use crate::prices;
use chrono::{Utc, TimeZone};

const MILLIS_YEAR: u64 = 31557600000;

#[derive(Serialize)]
pub struct CapitalGainsSummary {
    pub inventory_method: String,
    pub income: f64,
    pub short_term_capital_gains: f64,
    pub long_term_capital_gains: f64,
}

impl CapitalGainsSummary {

    pub fn save (&self, path: &str) -> Result<(), Box<dyn Error>> {
        let json_string = serde_json::to_string(&self)?;
        std::fs::write(path, &json_string)?;
        Ok(())
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Lot {
    pub timestamp: u64,
    pub qty: f64,
    pub cost: f64,
    pub host: Option<deltas::Host>,
    pub identifier: Option<String>,
}

impl Lot {
    pub fn remove_qty(&mut self, qty: f64) -> Self {

        assert!(qty < self.qty);


        // let price = cost/self.qty;
        let removed_cost = qty * self.cost/self.qty;

        self.qty -= qty;
        self.cost -= removed_cost;

        Self {
            timestamp: self.timestamp,
            qty: qty,
            cost: removed_cost,
            host: self.host.clone(),
            identifier: self.identifier.clone(),
        }
    }
}


#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Inventory ( pub HashMap<String, Vec<Lot>> );


#[derive(Clone, Copy)]
pub enum InventoryMethod {
    Fifo,
    Lifo,
    Yipo,
}


impl Inventory {
    pub fn initiate_zero_cost(balances: &HashMap<String, f64>, timestamp: u64) -> Self {

        let mut lots_inner = HashMap::new();

        for (asset, balance) in balances {
            let acq = Lot {
                timestamp: timestamp,
                qty: *balance,
                cost: 0_f64,
                host: None,
                identifier: None,

            };
            assert!(!lots_inner.contains_key(asset));
            lots_inner.insert(asset.clone(), vec![acq]);
        }
        Self ( lots_inner )

    }
    pub fn add_asset(&mut self, asset: &str) {
        self.0.insert(asset.to_string(), Vec::new());
    }

    pub fn load(path: &str) -> Result<Self, Box<dyn Error>> {
        let data = std::fs::read_to_string(path)?;
        let inner: Self = serde_json::from_str(&data)?;


        Ok(inner)
    }

    pub fn save (&self, path: &str) -> Result<(), Box<dyn Error>> {
        let json_string = serde_json::to_string(&self)?;
        std::fs::write(path, &json_string)?;
        Ok(())
    }


    pub fn consolidate_alias(&mut self, name: &str, alias: &str) {

        let mut to_copy = self.0[alias].clone();
        self.0.get_mut(name).unwrap().append(&mut to_copy);
        self.0.get_mut(name).unwrap().sort_by(|a, b| a.timestamp.cmp(&b.timestamp));
        self.0.remove(alias);

    }

    pub fn apply_deltas(&mut self, linked_deltas: &deltas::LinkedDeltas, quote_currency: &str, prices: &prices::Prices, method: InventoryMethod) -> (CapitalGainsSummary, String) {

        let smallest_liquidity_deltas = smallet_by_pair(linked_deltas);


        let mut link_only_short_term = 0.0;
        let mut link_only_long_term = 0.0;

        let mut events = "asset,quantity,disposition_date,acquisition_date,proceeds_USD,cost_basis_USD,capital_gain_USD,term\n".to_string();
        let mut test_gain = 0_f64;
        let mut long_term_capital_gains = 0_f64;
        let mut short_term_capital_gains = 0_f64;
        let mut income = 0_f64;

        let mut lowest_gain = 0f64;

        for group in &linked_deltas.0 {
            // Process all Ins in the group
            for delta in &group.ins {
                if delta.ilk == deltas::Ilk::WrapEth || delta.ilk == deltas::Ilk::UnwrapEth || delta.ilk == deltas::Ilk::TokenMigration {
                    continue
                }

                let symbol = symbols::delta_tax_ticker(&delta);

                income += group.income_for(delta, quote_currency, &prices);

                let cost = group.cost_for(delta, quote_currency, &prices);

                if cost < 0.0 {
                    panic!("");
                }

                if !self.0.contains_key(&symbol) {
                    self.0.insert(symbol.clone(), vec![]);
                }
                if self.0[&symbol].len() == 1 && self.0[&symbol][0].qty < 0.0 {

                    assert!(self.0[&symbol][0].qty.abs() < delta.qty);
                    assert!(self.0[&symbol][0].timestamp == 0);
                    assert!(self.0[&symbol][0].cost == 0.0);

                    self.0.get_mut(&symbol).unwrap()[0] = Lot {
                        timestamp: delta.timestamp,
                        qty: self.0[&symbol][0].qty + delta.qty,
                        cost: cost,
                        host: Some(delta.host.clone()),
                        identifier: Some(delta.identifier.clone()),
                    }

                } else {
                    let acq = Lot {
                        timestamp: delta.timestamp,
                        qty: delta.qty,
                        cost: cost,
                        host: Some(delta.host.clone()),
                        identifier: Some(delta.identifier.clone()),
                    };
                    self.0.get_mut(&symbol).unwrap().push(acq);
                };
            }

            // Process all Outs in the group
            for delta in &group.outs {
                if delta.ilk == deltas::Ilk::WrapEth || delta.ilk == deltas::Ilk::UnwrapEth || delta.ilk == deltas::Ilk::TokenMigration {
                    continue
                }

                let symbol = symbols::delta_tax_ticker(&delta);
                let total_revenue = group.revenue_for(delta, quote_currency, &prices);

                let mut rem_qty = delta.qty;
                let mut removed_lots = Vec::new();

                match method {
                    InventoryMethod::Fifo => {


                        while rem_qty > 0.0 {
                            let symbol = symbols::delta_tax_ticker(&delta);
                            if self.0[&symbol].len() == 0 {
                                self.0.get_mut(&symbol).unwrap().push ( Lot {
                                    timestamp: 0,
                                    qty: -rem_qty,
                                    cost: 0.0,
                                    host: None,
                                    identifier: None,
                                });
                                rem_qty = 0.0

                            } else if rem_qty >= self.0[&symbol][0].qty {
                                let removed = self.0.get_mut(&symbol).unwrap().remove(0);
                                rem_qty -= removed.qty;
                                removed_lots.push(removed);

                            } else {
                                let removed = self.0.get_mut(&symbol).unwrap()[0].remove_qty(rem_qty);
                                rem_qty -= removed.qty;
                                assert!(rem_qty == 0.0);
                                removed_lots.push(removed);
                            }
                        }
                    },
                    InventoryMethod::Lifo => {
                        let symbol = symbols::delta_tax_ticker(&delta);
                        while rem_qty > 0.0 {

                            if self.0[&symbol].len() == 0 {

                                self.0.get_mut(&symbol).unwrap().push ( Lot {
                                    timestamp: 0,
                                    qty: -rem_qty,
                                    cost: 0.0,
                                    host: None,
                                    identifier: None,

                                });
                                rem_qty = 0.0

                            } else if rem_qty >= self.0[&symbol].last().unwrap().qty {
                                let removed = self.0.get_mut(&symbol).unwrap().pop().unwrap();
                                rem_qty -= removed.qty;
                                removed_lots.push(removed);
                            } else {
                                let last_index = self.0[&symbol].len() - 1;
                                let removed = self.0.get_mut(&symbol).unwrap()[ last_index ].remove_qty(rem_qty);
                                rem_qty -= removed.qty;
                                assert!(rem_qty == 0.0);
                                removed_lots.push(removed);
                            }
                        }
                    },
                    InventoryMethod::Yipo => {
                        let symbol = symbols::delta_tax_ticker(&delta);
                        while rem_qty > 0.0 {

                            if self.0[&symbol].len() == 0 {
                                println!("neg acq_vec: {} from delta: {:#?}", rem_qty, delta);

                                self.0.get_mut(&symbol).unwrap().push ( Lot {
                                    timestamp: 0,
                                    qty: -rem_qty,
                                    cost: 0.0,
                                    host: None,
                                    identifier: None,

                                });
                                rem_qty = 0.0
                            } else if delta.timestamp - self.0[&symbol][0].timestamp >= MILLIS_YEAR {

                                if rem_qty >= self.0[&symbol][0].qty {
                                    let removed = self.0.get_mut(&symbol).unwrap().remove(0);
                                    rem_qty -= removed.qty;
                                    removed_lots.push(removed);
                                } else {
                                    let removed = self.0.get_mut(&symbol).unwrap()[0].remove_qty(rem_qty);
                                    rem_qty -= removed.qty;
                                    assert!(rem_qty == 0.0);
                                    removed_lots.push(removed);
                                }


                            } else {

                                if rem_qty >= self.0[&symbol].last().unwrap().qty {
                                    let removed = self.0.get_mut(&symbol).unwrap().pop().unwrap();
                                    rem_qty -= removed.qty;
                                    removed_lots.push(removed);
                                } else {
                                    let last_index = self.0[&symbol].len() - 1;
                                    let removed = self.0.get_mut(&symbol).unwrap()[ last_index ].remove_qty(rem_qty);
                                    rem_qty -= removed.qty;
                                    assert!(rem_qty == 0.0);
                                    removed_lots.push(removed);
                                }
                            }
                        }
                    }
                }


                self.remove_empty_positions(&symbols::delta_tax_ticker(&delta), &smallest_liquidity_deltas);


                let mut rev = 0_f64;

                for rem_acq in &removed_lots {

                    let revenue = total_revenue * (rem_acq.qty / delta.qty);
                    rev += revenue;

                    if self.0["USDC"][0].qty < 0.0 {
                    }


                    let gain = revenue - rem_acq.cost;

                    let term = if delta.timestamp - rem_acq.timestamp > MILLIS_YEAR {
                        long_term_capital_gains += gain;
                        if delta.asset == "LINK" {
                            link_only_long_term += gain;
                        }
                        "long".to_string()
                    } else {
                        short_term_capital_gains += gain;
                        if delta.asset == "LINK" {
                            link_only_short_term += gain;
                        }
                        "short".to_string()
                    };

                    if gain > lowest_gain {
                        println!("");
                        println!("{}", gain);

                        println!("disposition worth {} on {}", revenue, Utc.timestamp_millis(delta.timestamp as i64).to_string());
                        if !deltas::is_uni_cl_position(&delta.asset) {
                            println!("from: {} of {}", rem_acq.qty/delta.qty, delta.value(quote_currency, &prices));
                        }
                        println!("{:#?}", delta);

                        println!("cost of {} on {}, {:?}, {:?}", rem_acq.cost, Utc.timestamp_millis(rem_acq.timestamp as i64).to_string(), rem_acq.host, rem_acq.identifier);
                        lowest_gain = gain;
                    }
                    if symbol != quote_currency {
                        events += &format!
                            (
                            "{},{:.8},{},{},{:.8},{:.8},{:.8},{}\n",
                            symbol,
                            delta.qty,
                            Utc.timestamp_millis(delta.timestamp as i64).to_rfc3339_opts(chrono::SecondsFormat::Millis, true),
                            Utc.timestamp_millis(rem_acq.timestamp as i64).to_rfc3339_opts(chrono::SecondsFormat::Millis, true),
                            revenue,
                            rem_acq.cost,
                            revenue - rem_acq.cost,
                            term
                            )
                    }

                }

            }
        }


        let inventory_method = match method {
            InventoryMethod::Fifo => "FIFO".to_string(),
            InventoryMethod::Lifo => "LIFO".to_string(),
            InventoryMethod::Yipo => "Specific_Id".to_string(),
        };

        let summary = CapitalGainsSummary {
            inventory_method: inventory_method,
            income: income,
            long_term_capital_gains: long_term_capital_gains,
            short_term_capital_gains: short_term_capital_gains,
        };
        println!("link_only: long: {}, short: {}", link_only_long_term, link_only_short_term);
        println!("test_gain: {}", test_gain);
        (summary, events)

    }

    fn remove_empty_positions(&mut self, asset: &str, smallet_by_pair: &HashMap<String, f64>) {

        if deltas::is_uni_cl_position(asset) {


            let remove = if self.0[asset].len() == 0 {
                true
            } else if self.0[asset].len() == 1 {

                let sym = uni_cl_pair_name(asset);
                if self.0[asset][0].qty < smallet_by_pair[&sym] {
                    true
                } else {
                    false
                }
            } else {
                dbg!(&self.0[asset]);
                panic!("");
            };

            if remove {
                self.0.remove(asset);
            }
        }
    }
}




pub struct TaxableTotalsCanada {
    pub income: f64,
    pub capital_gains: f64,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Holding {
    pub qty: f64,
    pub cost: f64,
}

impl Holding {
    pub fn average_price(&self) -> f64 {
        self.cost / self.qty
    }

    pub fn cost_basis(&self, qty: f64) -> f64 {
        self.average_price() * qty
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ConsolidatedInventory ( pub HashMap<String, Holding> );


impl ConsolidatedInventory {
    pub fn initiate_zero_cost(balances: &HashMap<String, f64>) -> Self {

        let mut holdings_inner = HashMap::new();

        for (asset, balance) in balances {
            let h = Holding {
                qty: *balance,
                cost: 0_f64,

            };
            assert!(!holdings_inner.contains_key(asset));
            holdings_inner.insert(asset.clone(), h);
        }
        Self ( holdings_inner )

    }

    pub fn load(path: &str) -> Result<Self, Box<dyn Error>> {
        let data = std::fs::read_to_string(path)?;
        let inner: Self = serde_json::from_str(&data)?;
        Ok(inner)
    }

    pub fn save (&self, path: &str) -> Result<(), Box<dyn Error>> {
        let json_string = serde_json::to_string(&self)?;
        std::fs::write(path, &json_string)?;
        Ok(())
    }


    pub fn consolidate_alias(&mut self, name: &str, alias: &str) {

        self.0.get_mut(name).unwrap().qty += self.0[alias].qty;
        self.0.get_mut(name).unwrap().cost += self.0[alias].cost;
        self.0.remove(alias);
    }

    pub fn apply_deltas(&mut self, linked_deltas: &deltas::LinkedDeltas, quote_currency: &str, prices: &prices::Prices) -> (TaxableTotalsCanada, String) {
        let mut events = "asset,quantity,disposition_date,proceeds_CAD,cost_basis_CAD,capital_gain_CAD\n".to_string();

        let mut capital_gains = 0_f64;
        let mut income = 0_f64;

        for group in &linked_deltas.0 {
            // Process Ins
            for delta in &group.ins {
                if delta.ilk == deltas::Ilk::WrapEth || delta.ilk == deltas::Ilk::UnwrapEth || delta.ilk == deltas::Ilk::TokenMigration {
                    continue
                }

                let symbol = symbols::delta_tax_ticker(&delta);

                income += group.income_for(delta, quote_currency, &prices);

                let cost = group.cost_for(delta, quote_currency, &prices);

                if cost < 0.0 {
                    panic!("");
                }

                if self.0[&symbol].qty < 0.0 {
                    println!("neg acq_vec: {} from delta: {:#?}", self.0[&symbol].qty, delta);

                    assert!(delta.identifier == "0x32eeca6efe92db4119b412a172a909582d7c47a6830ee7c6f1cc334b0e70b0c4" || self.0[&symbol].qty.abs()  < 0.0000001);
                    assert!(self.0[&symbol].qty.abs() < delta.qty);
                }

                self.0.get_mut(&symbol).unwrap().qty += delta.qty;
                self.0.get_mut(&symbol).unwrap().cost += group.cost_for(delta, quote_currency, prices);
            }

            // Process Outs
            for delta in &group.outs {
                if delta.ilk == deltas::Ilk::WrapEth || delta.ilk == deltas::Ilk::UnwrapEth || delta.ilk == deltas::Ilk::TokenMigration {
                    continue
                }

                let symbol = symbols::delta_tax_ticker(&delta);
                let total_revenue = group.revenue_for(delta, quote_currency, prices);

                let cost_basis = self.0[&symbol].cost_basis(delta.qty);

                self.0.get_mut(&symbol).unwrap().qty -= delta.qty;
                self.0.get_mut(&symbol).unwrap().cost -= cost_basis;

                capital_gains += (total_revenue - cost_basis);

                if symbol != quote_currency {
                    events += &format!
                        (
                        "{},{:.8},{},{:.8},{:.8},{:.8}\n",
                        symbol,
                        delta.qty,
                        Utc.timestamp_millis(delta.timestamp as i64).to_rfc3339_opts(chrono::SecondsFormat::Millis, true),
                        total_revenue,
                        cost_basis,
                        total_revenue - cost_basis,
                        )
                }
            }
        }

        let summary = TaxableTotalsCanada {
            income: income,
            capital_gains: capital_gains,
        };
        (summary, events)
    }

}

/// Finds the smallest position quantity per token pair across all Uniswap
/// concentrated-liquidity positions (V3 and V4). Used for dust cleanup
/// — positions smaller than this threshold can be discarded.
fn smallet_by_pair(linked_deltas: &deltas::LinkedDeltas) -> HashMap<String, f64> {

    let mut smallet_by_pair: HashMap<String, f64> = HashMap::new();
    for group in &linked_deltas.0 {
        for d in group.all_deltas() {

            if deltas::is_uni_cl_position(&d.asset) {

                let sym = uni_cl_pair_name(&d.asset);

                if smallet_by_pair.contains_key(&sym) {
                    if d.qty < smallet_by_pair[&sym] {
                        *smallet_by_pair.get_mut(&sym).unwrap() = d.qty;
                    }
                } else {
                    smallet_by_pair.insert(sym, d.qty);
                };

            }
        }
    }
    smallet_by_pair
}

/// Extracts the token pair name (e.g. "WETH-USDC") from a Uniswap
/// concentrated-liquidity position asset identifier. Works for both V3 and V4
/// since both formats have the two token names at underscore-delimited
/// positions 1 and 2:
///   V3: `UNI-V3-LIQUIDITY:{tokenId}_{token0}_{token1}_{fee}_{tickLo}_{tickHi}`
///   V4: `UNI-V4-LIQUIDITY:{tokenId}_{token0}_{token1}_{poolId}_{tickLo}_{tickHi}`
fn uni_cl_pair_name(full_name: &str) -> String {
    assert!(deltas::is_uni_cl_position(full_name));

    let split: Vec<&str> = full_name.split("_").collect();
    let sym = format!("{}-{}", split[1], split[2]);
    sym

}
