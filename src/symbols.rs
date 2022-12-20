



















pub fn dealias (name: &str) -> String {
    let symbol = if name == "WETH" {
        String::from("ETH")
    } else if name == "WBTC" {
        String::from("BTC")
    } else if name == "REPv2" {
        String::from("REP")
    } else {
        String::from(name)
    };
    symbol
}
