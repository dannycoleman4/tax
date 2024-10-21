


pub fn requires_price (id: &str) -> bool {
    if ["WETH", "WBTC", "UNI-V1:ZRX", "UNI-V1:REP", "USDC.ARBITRUM"].contains(id) {
        false
    } else {
        true
    }
}


pub fn alias (id: &str) -> Alias {
    if id == "WETH" {
        Alias::Yes("ETH")
    } else if id == "WBTC" {
        Alias::Yes("BTC")
    } else if id == "USDC.ARBITRUM" {
        Alias::Yes("USDC")
    } else {
        Alias::No
    }
}



pub enum Alias {
    Yes(String),
    No
}
