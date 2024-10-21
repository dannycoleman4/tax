use crate::deltas;









pub fn delta_tax_ticker (delta: &deltas::Delta) -> String {
    if delta.asset.starts_with("UNI-V3-LIQUIDITY") {
        format!("{}:{}", delta.asset, delta.host.to_string())
    } else {
        onchain_ticker_to_tax_ticker(&delta.asset)
    }
}

pub fn onchain_ticker_to_tax_ticker (onchain_ticker: &str) -> String {
    let symbol = if onchain_ticker == "WETH" {
        String::from("ETH")
    } else if onchain_ticker == "WBTC" {
        String::from("BTC")
    } else if onchain_ticker == "REPv2" {
        String::from("REP")
    } else if onchain_ticker == "USDC.ARBITRUM" {
        String::from("USDC")
    } else if onchain_ticker == "USDC.BASE" {
        String::from("USDC")
    } else if onchain_ticker == "USDC.OPTIMISM" {
        String::from("USDC")
    } else if onchain_ticker == "WSOL" {
        String::from("SOL")
    } else if onchain_ticker.starts_with("UNI-V3-LIQUIDITY") {
        panic!();
    } else {
        String::from(onchain_ticker)
    };
    symbol
}

pub fn batch_onchain_to_tax_ticker(onchain_names: &Vec<String>) -> Vec<String> {
    let mut tax_names = Vec::new();
    for onchain_name in onchain_names {

        let tax_name = onchain_ticker_to_tax_ticker(onchain_name);
        if !tax_names.contains(&tax_name) {
            tax_names.push(tax_name);
        }
    }
    tax_names
}
