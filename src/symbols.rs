
use crate::deltas;



















pub fn rename_asset (delta: &deltas::Delta) -> String {
    let symbol = if &delta.asset == "WETH" {
        String::from("ETH")
    } else if &delta.asset == "WBTC" {
        String::from("BTC")
    } else if &delta.asset == "REPv2" {
        String::from("REP")
    } else if delta.asset.starts_with("UNI-V3-LIQUIDITY") {
        format!("{}:{}", delta.asset, delta.host.to_string())
    } else {
        String::from(&delta.asset)
    };
    symbol
}
