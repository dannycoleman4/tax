use std::error::Error;
use serde::{Serialize, Deserialize};
use std::collections::HashMap;
use chrono::{TimeZone, Utc, Timelike, DateTime};



// #[derive(Clone, Debug, Serialize, Deserialize)]
// pub enum Granularity {
//     M15,
//     H1,
//     D1,
// }

// impl Granularity {
//     pub fn seconds(&self) -> u64 {
//         match self {
//             Self::M15 => 900,
//             Self::H1 => 3600,
//             Self::D1 => 86400,
//         }
//     }
//     pub fn duration(&self) -> chrono::Duration {
//         chrono::Duration::seconds(self.seconds() as i64)
//     }
// }


#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Prices {
    // pub granularity: Granularity,
    pub map: HashMap<String, HashMap<String, f64>>,
}



impl Prices {

    pub fn load_dir(dir_path: &str, assets: &Vec<String>) -> Result<Self, Box<dyn Error>> {

        let mut map_map: HashMap<String, HashMap<String, f64>> = HashMap::new();

        for asset_id in assets {
            let path = format!("{}/{}.json", dir_path, asset_id);
            // println!("{}", path);
            let data = match std::fs::read_to_string(&path) {
                Ok(v) => v,
                Err(err) => {
                    println!("skipped: {}", asset_id);
                    continue
                }
            };
            let price_map: HashMap<String, f64> = serde_json::from_str(&data).unwrap();
            map_map.insert(asset_id.to_string(), price_map);

        }
        Ok(Prices {
            // granularity: Granularity::D1,
            map: map_map,

        })
    }

    // pub fn load_dir_candles(dir_path: &str, quote_asset: &str, base_assets: &Vec<String>) -> Result<Self, Box<dyn Error>> {

    //     let mut map_map: HashMap<String, HashMap<String, f64>> = HashMap::new();
    //     let mut min_diff = u64::MAX;

    //     for asset_id in base_assets {
    //         let path = format!("{}/{}-{}.json", dir_path, asset_id, quote_asset);
    //         
    //         // println!("{}", path);
    //         let data = match std::fs::read_to_string(&path) {
    //             Ok(v) => v,
    //             Err(err) => {
    //                 println!("skipped: {}", asset_id);
    //                 continue
    //             }
    //         };
    //         let candles: Vec<Candle> = serde_json::from_str(&data).unwrap();

    //         let mut last_ts = 0;
    //         let mut price_map = HashMap::new();
    //         for candle in &candles {

    //             if candle.timestamp_u64() - last_ts < min_diff {
    //                 min_diff = candle.timestamp_u64() - last_ts;
    //             }
    //             last_ts = candle.timestamp_u64();
    //             let price = (candle.high() + candle.low()) / 2.0;

    //             price_map.insert(candle.timestamp_rfc3339(), price);
    //         }
    //         map_map.insert(asset_id.to_string(), price_map);
    //     }
    //     let granularity = match min_diff {
    //         900 => Granularity::M15,
    //         3600 => Granularity::H1,
    //         86400 => Granularity::D1,
    //         _=> panic!("")
    //     };
    //     Ok(Prices {
    //         granularity: granularity,
    //         map: map_map,

    //     })
    // }

    pub fn fetch_coingecko(assets: &Vec<String>, from_unix: i64, to_unix: i64, api_key: &str, delay_millis: u64) -> Result<Self, Box<dyn Error>> {
        let mut map_map: HashMap<String, HashMap<String, f64>> = HashMap::new();

        for (i, asset_id) in assets.iter().enumerate() {
            let cg_id = match crate::asset_ids::coingecko_id(asset_id) {
                Some(id) => id,
                None => {
                    println!("skipped (no coingecko id): {}", asset_id);
                    continue;
                }
            };

            let url = format!(
                "https://pro-api.coingecko.com/api/v3/coins/{}/market_chart/range?vs_currency=usd&from={}&to={}",
                cg_id, from_unix, to_unix
            );

            let resp = match ureq::get(&url)
                .set("x-cg-pro-api-key", api_key)
                .call()
            {
                Ok(r) => r,
                Err(e) => {
                    println!("error fetching {}: {}", asset_id, e);
                    continue;
                }
            };

            let body: serde_json::Value = serde_json::from_str(&resp.into_string()?)?;
            let prices_arr = match body["prices"].as_array() {
                Some(arr) => arr,
                None => {
                    println!("no prices array for {}: {}", asset_id, body);
                    continue;
                }
            };

            let mut price_map = HashMap::new();
            for entry in prices_arr {
                let ts_ms = entry[0].as_f64().unwrap() as i64;
                let price = entry[1].as_f64().unwrap();
                let datetime = Utc.timestamp_millis(ts_ms);
                let date = datetime.date().format("%F").to_string();
                price_map.insert(date, price);
            }

            println!("fetched {} ({}): {} days", asset_id, cg_id, price_map.len());
            map_map.insert(asset_id.to_string(), price_map);

            if i < assets.len() - 1 {
                std::thread::sleep(std::time::Duration::from_millis(delay_millis));
            }
        }

        Ok(Prices { map: map_map })
    }

    pub fn load(path: &str) -> Result<Self, Box<dyn Error>> {
        let data = std::fs::read_to_string(path)?;
        let inner: Self = serde_json::from_str(&data)?;


        Ok(inner)
    }

    pub fn patch(&mut self, other: &Self, incl_start: DateTime<Utc>, excl_end: DateTime<Utc>) {
        assert!(incl_start < excl_end);
        let mut patched = 0;
        let mut cant_patch = 0;


        let all_keys_needed = {
            let mut all = Vec::new();
            let mut dt = incl_start;
            while dt < excl_end {
                // all.push(dt.to_rfc3339_opts(chrono::SecondsFormat::Secs, true));
                all.push(dt.date().format("%F").to_string());
                dt = dt + chrono::Duration::days(1);
            }
            all
        };

        for (asset_id, _) in &other.map {
            if !self.map.contains_key(asset_id) {
                self.map.insert(asset_id.clone(), HashMap::new());
            }
        }

        let mut self_assets = Vec::new();
        for asset_id in self.map.keys() {
            self_assets.push(asset_id.clone());
        }

        for asset_id in &self_assets {

            for key in &all_keys_needed {
                if !self.map[asset_id].contains_key(key) {
                    // dbg!(&asset_id);
                    if other.map.contains_key(asset_id) && other.map[asset_id].contains_key(key) {
                        // let price = other.price_at_rfc3339(asset_id, key);
                        let price = other.map[asset_id][key].clone();
                        println!("patch: {}, {}, {}", asset_id, key, price);
                        patched += 1;
                        self.map.get_mut(asset_id).unwrap().insert(key.clone(), price);
                    } else {
                        println!("can't patch: {}, {}", asset_id, key);
                        cant_patch += 1;
                    }
                }
            }
        }
        println!("total patched: {}", patched);
    }


    pub fn save (&self, path: &str) -> Result<(), Box<dyn Error>> {
        let json_string = serde_json::to_string(&self)?;
        std::fs::write(path, &json_string)?;
        Ok(())
    }

    // pub fn price_at_rfc3339(&self, asset: &str, timestamp: &str) -> f64 {
    //     let datetime = DateTime::parse_from_rfc3339(timestamp).unwrap();
    //     match self.granularity {
    //         Granularity::D1 => {
    //             let floor = datetime.date().and_hms(0, 0, 0).to_rfc3339_opts(chrono::SecondsFormat::Secs, true);
    //             self.map[asset][&floor]
    //         },
    //         Granularity::H1 => {
    //             let floor = datetime.date().and_hms(datetime.time().hour(), 0, 0).to_rfc3339_opts(chrono::SecondsFormat::Secs, true);
    //             self.map[asset][&floor]
    //         }
    //         Granularity::M15 => {
    //             let floor = datetime.date().and_hms(datetime.time().hour(), datetime.time().minute(), 0).to_rfc3339_opts(chrono::SecondsFormat::Secs, true);
    //             self.map[asset][&floor]
    //         }
    // 
    //     }

    // }

    pub fn price_at_millis(&self, asset: &str, timestamp: u64) -> f64 {
        let datetime = Utc.timestamp_millis(timestamp as i64);

        let p = {
             // let floor = datetime.date().and_hms(0, 0, 0).to_rfc3339_opts(chrono::SecondsFormat::Secs, true);
             let date: String = datetime.date().format("%F").to_string();;
             if !self.map.contains_key(asset) {
                 dbg!(asset);
             } else if !self.map[asset].contains_key(&date) {
                 dbg!(asset);
                 dbg!(&date);
             }
             self.map[asset][&date]

        };
        // let p = match self.granularity {
        //     Granularity::D1 => {
        //         let floor = datetime.date().and_hms(0, 0, 0).to_rfc3339_opts(chrono::SecondsFormat::Secs, true);
        //         if !self.map.contains_key(asset) {
        //             dbg!(asset);
        //         } else if !self.map[asset].contains_key(&floor) {
        //             dbg!(asset);
        //             dbg!(&floor);
        //         }
        //         self.map[asset][&floor]
        //     },
        //     Granularity::H1 => {
        //         let floor = datetime.date().and_hms(datetime.time().hour(), 0, 0).to_rfc3339_opts(chrono::SecondsFormat::Secs, true);
        //         self.map[asset][&floor]
        //     }
        //     Granularity::M15 => {
        //         let floor = datetime.date().and_hms(datetime.time().hour(), datetime.time().minute(), 0).to_rfc3339_opts(chrono::SecondsFormat::Secs, true);
        //         self.map[asset][&floor]
        //     }
    
        // };
        p
    }

    // pub fn price_at_millis(&self, asset: &str, timestamp: u64) -> f64 {
    //     match self.granularity {
    //         Granularity::D1 => {
    //             let date = Utc.timestamp_millis(timestamp as i64).date();
    //             let floor = date.and_hms(0, 0, 0).to_rfc3339_opts(chrono::SecondsFormat::Secs, true);
    //             self.map[asset][&floor]
    //         },
    //         Granularity::H1 => {
    //             let date = Utc.timestamp_millis(timestamp as i64).date();
    //             let hour = Utc.timestamp_millis(timestamp as i64).time().hour();
    //             let floor = date.and_hms(hour, 0, 0).to_rfc3339_opts(chrono::SecondsFormat::Secs, true);
    //             self.map[asset][&floor]
    //         }
    //         Granularity::M15 => {
    //             let date = Utc.timestamp_millis(timestamp as i64).date();
    //             let hour = Utc.timestamp_millis(timestamp as i64).time().hour();
    //             let minute = Utc.timestamp_millis(timestamp as i64).time().minute();
    //             let floor = date.and_hms(hour, minute, 0).to_rfc3339_opts(chrono::SecondsFormat::Secs, true);
    //             self.map[asset][&floor]
    //         }
    // 
    //     }
    // }

}


#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Candle ( pub (u64, f64, f64, f64, f64, f64) ); 

impl Candle {
    pub fn timestamp_u64(&self) -> u64 {
        self.0.0
    }

    pub fn timestamp_rfc3339(&self) -> String {
        Utc.timestamp(self.0.0 as i64, 0).to_rfc3339_opts(chrono::SecondsFormat::Secs, true)
    }

    pub fn open(&self) -> f64 {
        self.0.1
    }

    pub fn high(&self) -> f64 {
        self.0.2
    }

    pub fn low(&self) -> f64 {
        self.0.3
    }
}
