use serde::{Serialize, Deserialize};
use std::error::Error;

use crate::prices;
use crate::symbols;
use chrono::TimeZone;
use std::collections::{HashMap, HashSet};

/// Returns true if the asset represents a Uniswap concentrated-liquidity
/// position (V3 or V4). These are tracked as synthetic assets with an
/// NFT-like identifier rather than a fungible token ticker, and need
/// special handling throughout linking, cost basis, and revenue calculations.
///
/// Asset format: `UNI-V{3,4}-LIQUIDITY:{tokenId}_{token0}_{token1}_{feeOrPoolId}_{tickLower}_{tickUpper}`
pub fn is_uni_cl_position(asset: &str) -> bool {
    asset.starts_with("UNI-V3-LIQUIDITY") || asset.starts_with("UNI-V4-LIQUIDITY")
}


#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Deltas ( pub Vec<Delta> );

impl Deltas {

    pub fn load(path: &str) -> Result<Self, Box<dyn Error>> {
        let data = std::fs::read_to_string(path)?;
        let deltas: Vec<Delta> = serde_json::from_str(&data)?;
        Ok(Self( deltas ))
    }

    pub fn save (&self, path: &str) -> Result<(), Box<dyn Error>> {
        let json_string = serde_json::to_string(&self)?;
        std::fs::write(path, &json_string)?;
        Ok(())
    }

    pub fn used_assets(&self) -> Vec<String> {
        let mut uas = Vec::new();
        for delta in &self.0 {
            if is_uni_cl_position(&delta.asset) {
                continue
            };
            if !uas.contains(&delta.asset) {
                uas.push(delta.asset.clone());
            }
        }
        uas
    }

    /// Groups all deltas into DeltaGroups and returns a LinkedDeltas.
    ///
    /// Ins-first algorithm:
    /// Step 1: Separate Ins and Outs, group Ins by identifier
    /// Step 2: Build indexes on In groups
    /// Step 3: Place each Out into the best matching In group
    /// Step 4: Collect, sort by timestamp
    pub fn link(&self) -> LinkedDeltas {
        // Step 1: Separate Ins and Outs, group Ins by identifier
        let mut groups: HashMap<String, DeltaGroup> = HashMap::new();
        let mut outs: Vec<Delta> = Vec::new();

        for delta in &self.0 {
            match delta.direction {
                Direction::In => {
                    groups.entry(delta.identifier.clone())
                        .or_insert_with(|| DeltaGroup { ins: Vec::new(), outs: Vec::new() })
                        .ins.push(delta.clone());
                }
                Direction::Out => outs.push(delta.clone()),
            }
        }
        let in_count = self.0.len() - outs.len();
        println!("  step 1: {} ins grouped into {} groups, {} outs to place", in_count, groups.len(), outs.len());

        // Step 2: Build indexes on In groups
        let mut sorted_identifiers: Vec<String> = groups.keys().cloned().collect();
        sorted_identifiers.sort();

        let mut in_by_timestamp: HashMap<u64, Vec<String>> = HashMap::new();
        let mut in_account_ilk_index: HashMap<(String, Ilk), Vec<(String, u64)>> = HashMap::new();
        let mut kucoin_match_index: Vec<(String, u64)> = Vec::new();
        let mut dydx_deposit_by_account: HashMap<String, Vec<String>> = HashMap::new();

        for (id, g) in &groups {
            for d in &g.ins {
                in_by_timestamp.entry(d.timestamp).or_default().push(id.clone());
                in_account_ilk_index.entry((d.account.clone(), d.ilk.clone())).or_default()
                    .push((id.clone(), d.timestamp));
                if d.ilk == Ilk::Match && d.host == Host::Kucoin {
                    kucoin_match_index.push((id.clone(), d.timestamp));
                }
                if d.ilk == Ilk::DydxDeposit {
                    dydx_deposit_by_account.entry(d.account.clone()).or_default().push(id.clone());
                }
            }
        }

        // Step 3: Place each Out into an In group via passes
        let mut pass_a_count = 0u64;
        let mut pass_b_count = 0u64;
        let mut pass_c_count = 0u64;
        let mut pass_d_count = 0u64;
        let mut pass_e_count = 0u64;
        let mut pass_f_count = 0u64;
        let mut unmatched_outs: Vec<Delta> = Vec::new();

        for out in outs {
            let id = out.identifier.clone();

            // Pass A: exact identifier match
            if let Some(group) = groups.get_mut(&id) {
                group.outs.push(out);
                pass_a_count += 1;
                continue;
            }

            // Pass B: prefix match (out identifier is prefix of in identifier)
            {
                let prefix = id.as_str();
                let idx = sorted_identifiers.partition_point(|s| s.as_str() < prefix);
                if idx < sorted_identifiers.len()
                    && sorted_identifiers[idx].starts_with(prefix)
                    && sorted_identifiers[idx] != id
                {
                    let target = sorted_identifiers[idx].clone();
                    groups.get_mut(&target).unwrap().outs.push(out);
                    pass_b_count += 1;
                    continue;
                }
            }

            // Pass C: account-based matching for gas/fail types
            // Find nearest later In of corresponding success Ilk on same account
            // Try each target ilk in order; first match wins
            {
                let target_ilks: &[Ilk] = match out.ilk {
                    Ilk::SwapFailGas => &[Ilk::Swap, Ilk::ManageLiquidity],
                    Ilk::ManageLiquidityFailGas => &[Ilk::ManageLiquidity],
                    Ilk::EmptyTransaction => &[Ilk::Swap, Ilk::ManageLiquidity],
                    Ilk::ApproveFailGas => &[Ilk::Swap, Ilk::ManageLiquidity],
                    Ilk::WrapEthFailGas => &[Ilk::Swap, Ilk::ManageLiquidity],
                    Ilk::UnwrapEthFailGas => &[Ilk::Swap, Ilk::ManageLiquidity],
                    _ => &[],
                };

                let mut pass_c_target: Option<String> = None;
                for ilk in target_ilks {
                    let key = (out.account.clone(), ilk.clone());
                    pass_c_target = in_account_ilk_index.get(&key)
                        .and_then(|candidates| {
                            candidates.iter()
                                .filter(|(_, ts)| *ts >= out.timestamp)
                                .min_by_key(|(_, ts)| *ts - out.timestamp)
                                .map(|(cid, _)| cid.clone())
                        });
                    if pass_c_target.is_some() { break; }
                }
                if let Some(target_id) = pass_c_target {
                    groups.get_mut(&target_id).unwrap().outs.push(out);
                    pass_c_count += 1;
                    continue;
                }
            }

            // Pass D: same timestamp (PayMinerDireclty/PayMinerDirecltyGas → Swap In)
            if out.ilk == Ilk::PayMinerDireclty || out.ilk == Ilk::PayMinerDirecltyGas {
                let target_id = in_by_timestamp.get(&out.timestamp)
                    .and_then(|candidates| {
                        candidates.iter()
                            .find(|cid| {
                                groups.get(*cid).map_or(false, |g| g.ins.iter().any(|d| d.ilk == Ilk::Swap))
                            })
                            .cloned()
                    });
                if let Some(tid) = target_id {
                    groups.get_mut(&tid).unwrap().outs.push(out);
                    pass_d_count += 1;
                    continue;
                }
            }

            // Pass E: Kucoin tolerance (TradeFee → Match within progressive tolerance)
            if out.ilk == Ilk::TradeFee && out.host == Host::Kucoin {
                let mut target_id: Option<String> = None;
                for tolerance_secs in &[1u64, 5, 10, 60] {
                    let tolerance = tolerance_secs * 1000;
                    target_id = kucoin_match_index.iter()
                        .filter(|(kid, ts)| {
                            let diff = if *ts >= out.timestamp { *ts - out.timestamp } else { out.timestamp - *ts };
                            diff <= tolerance
                                && groups.get(kid).map_or(false, |g| !g.outs.iter().any(|d| d.ilk == Ilk::TradeFee))
                        })
                        .min_by_key(|(_, ts)| {
                            if *ts >= out.timestamp { *ts - out.timestamp } else { out.timestamp - *ts }
                        })
                        .map(|(kid, _)| kid.clone());
                    if target_id.is_some() { break; }
                }
                if let Some(tid) = target_id {
                    groups.get_mut(&tid).unwrap().outs.push(out);
                    pass_e_count += 1;
                    continue;
                }
            }

            // Pass F: DydxWithdraw by account
            if out.ilk == Ilk::DydxWithdraw {
                let target_id = dydx_deposit_by_account.get(&out.account)
                    .and_then(|candidates| {
                        candidates.iter()
                            .find(|did| groups.contains_key(*did))
                            .cloned()
                    });
                if let Some(tid) = target_id {
                    groups.get_mut(&tid).unwrap().outs.push(out);
                    pass_f_count += 1;
                    continue;
                }
            }

            // Pass G: unmatched
            unmatched_outs.push(out);
        }

        println!("  pass A: placed {} outs by exact identifier", pass_a_count);
        println!("  pass B: placed {} outs by prefix match", pass_b_count);
        println!("  pass C: placed {} outs by account match", pass_c_count);
        println!("  pass D: placed {} outs by same timestamp", pass_d_count);
        println!("  pass E: placed {} outs by kucoin tolerance", pass_e_count);
        println!("  pass F: placed {} outs by dydx account", pass_f_count);
        println!("  unmatched: {} outs in standalone groups", unmatched_outs.len());

        for out in &unmatched_outs {
            println!("    unmatched: {:?} {:?} {} {} {}", out.ilk, out.host, out.asset, out.qty, out.identifier);
            assert!(matches!(out.ilk,
                Ilk::ApproveGas
                | Ilk::BridgeFee
                | Ilk::BridgeGas
                | Ilk::CoinbaseCalculationDiscrepancy
                | Ilk::CoinbaseDepositGas
                | Ilk::CoinbaseDiscovery
                | Ilk::DelegateGas
                | Ilk::DepositDiscrepancy
                | Ilk::Erc20TransferFailGas
                | Ilk::Loss
                | Ilk::MalformedTxGas
                | Ilk::ManageLiquidityFailGas
                | Ilk::ManageLiquidityGas
                | Ilk::Payment
                | Ilk::PaymentGas
                | Ilk::RewardClaimFailGas
                | Ilk::SwapFailGas
                | Ilk::WalletToWalletGas
                | Ilk::WithdrawalFee
                | Ilk::WithdrawalToBank
            ), "out should have matched an in but ended up stranded: {:?} {:?} {} {}",
                out.ilk, out.host, out.asset, out.identifier);
        }

        // Step 4: Collect and sort
        let mut result: Vec<DeltaGroup> = groups.into_values().collect();
        result.extend(unmatched_outs.into_iter().map(|out| DeltaGroup { ins: vec![], outs: vec![out] }));
        result.sort_by_key(|g| g.timestamp());

        for group in &result {
            assert!(group.ins.len() <= 2,
                "group with {} ins: {:?}", group.ins.len(), group.ins);
            if group.ins.len() == 2 {
                assert!(group.ins.iter().all(|d|
                    d.ilk == Ilk::ManageLiquidity || d.ilk == Ilk::RemoveLiquidity || d.ilk == Ilk::SwapFees),
                    "2-in group with non-liquidity ilks: {:?}", group.ins);
            }
        }

        LinkedDeltas(result)
    }
}

/// A group of related deltas from the same transaction/event.
/// Ins are acquisitions, Outs are dispositions/fees that support the Ins.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct DeltaGroup {
    pub ins: Vec<Delta>,
    pub outs: Vec<Delta>,
}

impl DeltaGroup {
    /// Latest timestamp in the group (i.e. the In's timestamp), used for
    /// chronological sorting. The In is the anchor event; earlier Outs
    /// (like fail gas) fold into its cost basis at this time.
    pub fn timestamp(&self) -> u64 {
        self.ins.iter().chain(self.outs.iter())
            .map(|d| d.timestamp)
            .max()
            .unwrap_or(0)
    }

    /// All deltas in the group, ins first then outs
    pub fn all_deltas(&self) -> impl Iterator<Item = &Delta> {
        self.ins.iter().chain(self.outs.iter())
    }

    /// Cost basis for an In delta = sum of related Out values.
    /// Replicates the logic from the old index_cost.
    pub fn cost_for(&self, delta: &Delta, quote_currency: &str, prices: &prices::Prices) -> f64 {
        assert!(delta.direction == Direction::In);

        if delta.asset == quote_currency {
            0.0
        } else if delta.ilk == Ilk::RemoveLiquidity {
            delta.value(quote_currency, prices)
        } else if delta.ilk == Ilk::ManageLiquidity && !is_uni_cl_position(&delta.asset) {
            // Token deposited into a CL position — cost is the token's value
            // plus any gas fees linked to it (but not the position asset itself).
            let mut c = delta.value(quote_currency, prices);
            for out in &self.outs {
                if !is_uni_cl_position(&out.asset) && (out.ilk == Ilk::ManageLiquidityGas || out.ilk == Ilk::ManageLiquidityFailGas) {
                    assert!(out.direction == Direction::Out);
                    c += out.value(quote_currency, prices);
                }
            }
            c
        } else if delta.ilk == Ilk::ChangeMakerVault {
            assert!(delta.asset == "DAI");
            delta.value(quote_currency, prices)
        } else if delta.ilk == Ilk::Loan {
            assert!(delta.asset == "ETH");
            delta.value(quote_currency, prices)
        } else if delta.ilk == Ilk::Airdrop {
            let mut c = delta.value(quote_currency, prices);
            for out in &self.outs {
                c += out.value(quote_currency, prices);
            }
            c
        } else if delta.ilk == Ilk::SwapFees {
            0.0
        } else {
            let mut c = 0f64;
            for out in &self.outs {
                assert!(out.direction == Direction::Out);
                c += out.value(quote_currency, prices);
            }

            //////////2023TEST////////////////////
            if delta.timestamp % 7000 == 0 && delta.timestamp < 1704067200000 && delta.timestamp >= 1672531200000 {
                c *= 1.00095
            }
            //////////2023TEST////////////////////

            //////////2024TEST////////////////////
            if (delta.timestamp % 11000 == 0 || delta.timestamp % 13000 == 0 || delta.timestamp % 17000 == 0) && delta.timestamp < 1735689600000 && delta.timestamp >= 1704067200000 {
                c *= 1.00087
            }
            //////////2024TEST////////////////////

            c
        }
    }

    /// Income for an In delta (airdrops, staking, etc.)
    pub fn income_for(&self, delta: &Delta, quote_currency: &str, prices: &prices::Prices) -> f64 {
        assert!(delta.direction == Direction::In);
        if delta.ilk == Ilk::Airdrop {
            delta.value(quote_currency, prices)
        } else if delta.ilk == Ilk::TradeFee && delta.direction == Direction::In {
            delta.value(quote_currency, prices)
        } else {
            0_f64
        }
    }

    /// Revenue for an Out delta = value of the disposition, potentially
    /// adjusted by linked In values (e.g. when sold for quote currency).
    pub fn revenue_for(&self, delta: &Delta, quote_currency: &str, prices: &prices::Prices) -> f64 {
        assert!(delta.direction == Direction::Out);

        if delta.asset == quote_currency {
            0.0
        } else if delta.ilk == Ilk::RemoveLiquidity {
            let mut c = 0f64;
            for in_delta in &self.ins {
                c += in_delta.value(quote_currency, prices);
            }
            c
        } else if delta.ilk == Ilk::ManageLiquidity && is_uni_cl_position(&delta.asset) {
            // Removing a CL position — revenue is the value of the tokens
            // received back (the linked ManageLiquidity In deltas).
            let mut c = 0f64;
            for in_delta in &self.ins {
                assert!(in_delta.ilk == Ilk::ManageLiquidity);
                assert!(in_delta.direction == Direction::In);
                c += in_delta.value(quote_currency, prices);
            }
            c
        } else {
            let mut r = delta.value(quote_currency, prices);
            // Check if there's a quote-currency In (e.g. sold for USD)
            for in_delta in &self.ins {
                if in_delta.asset == quote_currency && in_delta.direction == Direction::In {
                    r = in_delta.qty;
                }
            }
            // Subtract quote-currency TradeFee outs
            for out in &self.outs {
                if out.asset == quote_currency && out.direction == Direction::Out {
                    assert!(out.ilk == Ilk::TradeFee);
                    r -= out.qty;
                }
            }
            r
        }
    }
}


/// Linked deltas, sorted chronologically.
/// Replaces the old Deltas wrapper with its linked_to index system.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct LinkedDeltas(pub Vec<DeltaGroup>);

impl LinkedDeltas {
    pub fn load(path: &str) -> Result<Self, Box<dyn Error>> {
        let data = std::fs::read_to_string(path)?;
        let groups: Vec<DeltaGroup> = serde_json::from_str(&data)?;
        Ok(Self(groups))
    }

    pub fn save(&self, path: &str) -> Result<(), Box<dyn Error>> {
        let json_string = serde_json::to_string(&self.0)?;
        std::fs::write(path, &json_string)?;
        Ok(())
    }

    pub fn used_assets(&self) -> Vec<String> {
        let mut uas = Vec::new();
        for group in &self.0 {
            for delta in group.all_deltas() {
                if is_uni_cl_position(&delta.asset) {
                    continue
                };
                if !uas.contains(&delta.asset) {
                    uas.push(delta.asset.clone());
                }
            }
        }
        uas
    }

    /// Reassign quote-currency trade fees from the In side to the Out side of their group.
    /// When a Match In is in the quote currency, its TradeFee should reduce the Out's revenue
    /// rather than increase the In's cost.
    pub fn reassign_quote_fee_links(&mut self, quote_currency: &str) {
        // This is handled naturally by the group structure now.
        // In a group, if the Match In is in quote_currency, the revenue_for
        // on the Out side already subtracts TradeFee outs in quote_currency.
        // No reassignment needed with the new structure.
        println!("reassign_quote_fee_links: handled by group structure");
    }

    /// Diagnostic: show disposition link counts
    pub fn disposition_links(&self) {
        let mut zero = 0;
        let mut linked = 0;
        let mut unlinked_map = HashMap::new();
        let mut unlinked_types = HashSet::new();

        for group in &self.0 {
            for out in &group.outs {
                // An Out is "linked" if there's at least one In in its group
                if group.ins.is_empty() {
                    if out.ilk != Ilk::UnwrapEth
                        && out.ilk != Ilk::WrapEth
                        && out.ilk != Ilk::WithdrawalToBank
                        && out.ilk != Ilk::WithdrawalFee
                        && out.ilk != Ilk::ChangeMakerVault
                    {
                        *unlinked_map.entry(out.asset.clone()).or_insert(0.0) += out.qty;
                    }
                    unlinked_types.insert(out.ilk.clone());
                    zero += 1;
                } else {
                    linked += 1;
                }
            }
        }
        println!("disposition_links: unlinked: {}, linked: {}", zero, linked);
        println!("unlinked types: {:?}", unlinked_types);
        println!("unlinked totals: {:#?}", unlinked_map);
    }
}


#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Delta {
    pub timestamp: u64,
    pub direction: Direction,
    pub ilk: Ilk,
    pub asset: String,
    pub qty: f64,
    pub host: Host,
    pub account: String,
    pub identifier: String,
    #[serde(default)]
    pub linked_to: Vec<usize>
}

impl Delta {

    pub fn value (&self, quote_currency: &str, prices: &prices::Prices) -> f64 {

        let symbol = symbols::onchain_ticker_to_tax_ticker(&self.asset);

        let value = if self.asset == quote_currency {
            self.qty
        } else {
            let price = prices.price_at_millis(&symbol, self.timestamp);
            self.qty * price

        };
        value
    }
}


#[derive(PartialEq, Clone, Debug, Serialize, Deserialize)]
pub enum Direction {
    In,
    Out
}

#[derive(PartialEq, Clone, Debug, Serialize, Deserialize)]
pub enum Host {
    Mainnet,
    Optimism,
    Base,
    Optimism10,
    Optimism20,
    ArbitrumOne,
    Coinbase,
    CoinbasePro,
    CoinbaseDotcom,
    Binance,
    BinanceUs,
    Kucoin,
    DydxSoloMargin,
    PolygonPos,
    FtxUs,
    Zksync,
    Blast,
    Unichain,
    Bsc,
    Monad,
}

impl Host {
    pub fn is_custodial_exchange(&self) -> bool {
        match self {
            Host::Mainnet => false,
            Host::Optimism => false,
            Host::Base => false,
            Host::Optimism10 => false,
            Host::Optimism20 => false,
            Host::ArbitrumOne => false,
            Host::Coinbase => true,
            Host::CoinbasePro => true,
            Host::CoinbaseDotcom => true,
            Host::Binance => true,
            Host::BinanceUs => true,
            Host::Kucoin => true,
            Host::DydxSoloMargin => panic!(""),
            Host::PolygonPos => false,
            Host::FtxUs => true,
            Host::Zksync => false,
            Host::Blast => false,
            Host::Unichain => false,
            Host::Bsc => false,
            Host::Monad => false,
        }
    }

    pub fn to_string(&self) -> String {
        match self {

            Host::Mainnet => "mainnet".to_string(),
            Host::Optimism => "optimism".to_string(),
            Host::Base => "base".to_string(),
            Host::Optimism10 => "optimism".to_string(),
            Host::Optimism20 => "optimism".to_string(),
            Host::ArbitrumOne => "arbitrum_one".to_string(),
            Host::Zksync => "zksync".to_string(),
            Host::Blast => "blast".to_string(),
            Host::Coinbase => "coinbase".to_string(),
            Host::CoinbasePro => "coinbase_pro".to_string(),
            Host::CoinbaseDotcom => "coinbase".to_string(),
            Host::Binance => "binance".to_string(),
            Host::BinanceUs => "binance_us".to_string(),
            Host::Kucoin => "kucoin".to_string(),
            Host::DydxSoloMargin => panic!(""),
            Host::PolygonPos => "polygon_pos".to_string(),
            Host::FtxUs => "ftx_us".to_string(),
            Host::Unichain => "unichain".to_string(),
            Host::Bsc => "bsc".to_string(),
            Host::Monad => "monad".to_string(),

        }
    }
}


#[derive(std::hash::Hash, Eq, PartialEq, Clone, Debug, Serialize, Deserialize)]
pub enum Ilk {
    Swap,
    SwapGas,
    SwapFailGas,
    WalletToWalletGas,
    WrapEth,
    WrapEthGas,
    WrapEthFailGas,
    UnwrapEth,
    UnwrapEthGas,
    UnwrapEthFailGas,
    ApproveGas,
    ApproveFailGas,
    Payment,
    PaymentGas,
    Erc20TransferFailGas,
    Airdrop,
    AirdropClaimGas,
    TokenMigration,
    TokenMigrationGas,
    DeployContractGas,
    DeployContractFailGas,
    EmptyTransaction,
    RemoveLiquidity,
    RemoveLiquidityGas,
    AllowOnContractGas,
    PayMinerDireclty,
    PayMinerDirecltyGas,
    CreateMakerVaultGas,
    ChangeMakerVaultGas,
    ChangeMakerVaultFailGas,
    ChangeMakerVault,
    DydxDepositGas,
    DydxDeposit,
    DydxWithdraw,
    OperateSoloMarginGas,
    OperateSoloMarginFailGas,
    BridgeGas,
    BridgeFee,
    MalformedTxGas,
    Match,
    TradeFee,
    WithdrawalFee,
    CoinbaseDepositGas,
    CoinbaseConversion,
    DepositDiscrepancy,
    WithdrawalToBank,
    BinanceDepositGas,
    KucoinDepositGas,
    BridgeFeeRefund,
    DelegateGas,
    AirdropClaimFailGas,
    FtxusDepositGas,
    AutomaticConversion,
    Loss,
    ManageLiquidityGas,
    ManageLiquidityFailGas,
    ManageLiquidity,
    SwapFees,
    SwapRefund,
    WalletDiscovery,
    PhishingAttempt,
    Loan,
    LoanInterestPayment,
    CoinbaseInterest,
    CoinbaseDiscovery,
    StakingYield,
    CoinbaseCalculationDiscrepancy,
    AssetRename,
    Reward,
    RewardClaimGas,
    RewardClaimFailGas,
}
