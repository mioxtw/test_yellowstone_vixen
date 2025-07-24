use crate::types::*;
use rust_decimal::Decimal;
use solana_sdk::pubkey::Pubkey;
use std::collections::HashMap;
use std::str::FromStr;
use tracing::debug;

/// SWAP 分析器
#[derive(Debug)]
pub struct SwapAnalyzer {
    /// 是否啟用詳細日誌
    verbose: bool,
}

impl SwapAnalyzer {
    /// 創建新的 SWAP 分析器
    pub fn new(verbose: bool) -> Self {
        Self { verbose }
    }

    /// 從 Jupiter 交易指令分析 SWAP 路徑
    pub fn analyze_jupiter_swap(
        &self,
        instruction_data: &[u8],
        accounts: &[Pubkey],
        pre_balances: &HashMap<Pubkey, HashMap<String, Decimal>>,
        post_balances: &HashMap<Pubkey, HashMap<String, Decimal>>,
    ) -> Option<SwapPath> {
        if self.verbose {
            debug!("分析 Jupiter SWAP 指令，帳戶數: {}", accounts.len());
        }

        // 解析 Jupiter 指令數據
        let swap_info = self.parse_jupiter_instruction(instruction_data)?;
        
        // 分析餘額變化來構建路徑
        let balance_changes = self.calculate_balance_changes(pre_balances, post_balances);
        
        // 構建 SWAP 路徑
        self.build_swap_path(swap_info, balance_changes, accounts)
    }

    /// 解析 Jupiter 指令數據
    fn parse_jupiter_instruction(&self, data: &[u8]) -> Option<JupiterSwapInfo> {
        if data.len() < 8 {
            return None;
        }

        // Jupiter V6 指令判別符
        let discriminator = &data[0..8];
        
        match discriminator {
            // Route 指令 (最常見的 SWAP)
            [0x22, 0x5c, 0x7b, 0x45, 0x8a, 0x4e, 0x7f, 0x18] => {
                self.parse_route_instruction(&data[8..])
            }
            // SharedAccountsRoute 指令
            [0x3a, 0x4b, 0x15, 0x2e, 0x77, 0xc4, 0x9a, 0x6d] => {
                self.parse_shared_accounts_route(&data[8..])
            }
            _ => {
                if self.verbose {
                    debug!("未知的 Jupiter 指令判別符: {:?}", discriminator);
                }
                None
            }
        }
    }

    /// 解析 Route 指令
    fn parse_route_instruction(&self, data: &[u8]) -> Option<JupiterSwapInfo> {
        if data.len() < 16 {
            return None;
        }

        // 解析基本參數
        let in_amount = u64::from_le_bytes(data[0..8].try_into().ok()?);
        let quoted_out_amount = u64::from_le_bytes(data[8..16].try_into().ok()?);
        
        let offset = 16;
        
        // 解析路徑步驟數
        if offset + 1 > data.len() {
            return None;
        }
        let route_len = data[offset] as usize;

        if self.verbose {
            debug!(
                "Jupiter Route: 輸入 {} lamports, 預期輸出 {} lamports, {} 步",
                in_amount, quoted_out_amount, route_len
            );
        }

        Some(JupiterSwapInfo {
            in_amount: Decimal::from(in_amount),
            quoted_out_amount: Decimal::from(quoted_out_amount),
            route_steps: route_len,
            slippage_bps: 0, // 需要進一步解析
        })
    }

    /// 解析 SharedAccountsRoute 指令
    fn parse_shared_accounts_route(&self, data: &[u8]) -> Option<JupiterSwapInfo> {
        // 類似於 route_instruction，但使用共享帳戶優化
        if data.len() < 17 {
            return None;
        }

        let route_id = data[0];
        let in_amount = u64::from_le_bytes(data[1..9].try_into().ok()?);
        let quoted_out_amount = u64::from_le_bytes(data[9..17].try_into().ok()?);

        if self.verbose {
            debug!(
                "Jupiter SharedAccountsRoute: ID {}, 輸入 {} lamports, 預期輸出 {} lamports",
                route_id, in_amount, quoted_out_amount
            );
        }

        Some(JupiterSwapInfo {
            in_amount: Decimal::from(in_amount),
            quoted_out_amount: Decimal::from(quoted_out_amount),
            route_steps: 1, // 需要進一步分析
            slippage_bps: 0,
        })
    }

    /// 計算餘額變化
    fn calculate_balance_changes(
        &self,
        pre_balances: &HashMap<Pubkey, HashMap<String, Decimal>>,
        post_balances: &HashMap<Pubkey, HashMap<String, Decimal>>,
    ) -> Vec<TokenBalanceChange> {
        let mut changes = Vec::new();

        // 合併所有帳戶
        let mut all_accounts = std::collections::HashSet::new();
        all_accounts.extend(pre_balances.keys());
        all_accounts.extend(post_balances.keys());

        for account in all_accounts {
            let pre_account_balances = pre_balances.get(account).cloned().unwrap_or_default();
            let post_account_balances = post_balances.get(account).cloned().unwrap_or_default();

            // 合併所有代幣 mints
            let mut all_mints = std::collections::HashSet::new();
            all_mints.extend(pre_account_balances.keys());
            all_mints.extend(post_account_balances.keys());

            for mint_str in all_mints {
                let pre_balance = pre_account_balances.get(mint_str).cloned().unwrap_or_default();
                let post_balance = post_account_balances.get(mint_str).cloned().unwrap_or_default();
                let change = post_balance - pre_balance;

                if change != Decimal::ZERO {
                    if let Ok(mint) = Pubkey::from_str(mint_str) {
                        changes.push(TokenBalanceChange {
                            mint,
                            account: *account,
                            pre_balance,
                            post_balance,
                            change,
                        });
                    }
                }
            }
        }

        changes
    }

    /// 構建 SWAP 路徑
    fn build_swap_path(
        &self,
        _swap_info: JupiterSwapInfo,
        balance_changes: Vec<TokenBalanceChange>,
        _accounts: &[Pubkey],
    ) -> Option<SwapPath> {
        if balance_changes.is_empty() {
            return None;
        }

        // 尋找 WSOL 相關的變化
        let wsol_mint = Pubkey::from_str(WSOL_MINT).ok()?;
        let mut wsol_input = Decimal::ZERO;
        let mut wsol_output = Decimal::ZERO;
        let mut intermediate_mints = Vec::new();
        let mut hops = Vec::new();

        // 分析餘額變化以構建路徑
        for change in &balance_changes {
            if change.mint == wsol_mint {
                if change.change < Decimal::ZERO {
                    wsol_input += change.change.abs();
                } else {
                    wsol_output += change.change;
                }
            } else if change.change != Decimal::ZERO {
                intermediate_mints.push(change.mint);
            }
        }

        // 構建跳躍
        if intermediate_mints.is_empty() {
            // 單跳：WSOL -> WSOL（可能是套利）
            hops.push(SwapHop {
                input_mint: wsol_mint,
                output_mint: wsol_mint,
                input_amount: wsol_input,
                output_amount: wsol_output,
                amm_program: Pubkey::from_str(JUPITER_V6_PROGRAM).unwrap(),
                pool_address: None,
            });
        } else {
            // 多跳路徑
            // 第一跳：WSOL -> 中間代幣
            if let Some(&first_intermediate) = intermediate_mints.first() {
                hops.push(SwapHop {
                    input_mint: wsol_mint,
                    output_mint: first_intermediate,
                    input_amount: wsol_input,
                    output_amount: self.estimate_intermediate_amount(&balance_changes, first_intermediate),
                    amm_program: Pubkey::from_str(JUPITER_V6_PROGRAM).unwrap(),
                    pool_address: None,
                });

                // 中間跳躍
                for i in 1..intermediate_mints.len() {
                    let prev_mint = intermediate_mints[i - 1];
                    let curr_mint = intermediate_mints[i];
                    
                    hops.push(SwapHop {
                        input_mint: prev_mint,
                        output_mint: curr_mint,
                        input_amount: self.estimate_intermediate_amount(&balance_changes, prev_mint),
                        output_amount: self.estimate_intermediate_amount(&balance_changes, curr_mint),
                        amm_program: Pubkey::from_str(JUPITER_V6_PROGRAM).unwrap(),
                        pool_address: None,
                    });
                }

                // 最後一跳：中間代幣 -> WSOL
                if let Some(&last_intermediate) = intermediate_mints.last() {
                    hops.push(SwapHop {
                        input_mint: last_intermediate,
                        output_mint: wsol_mint,
                        input_amount: self.estimate_intermediate_amount(&balance_changes, last_intermediate),
                        output_amount: wsol_output,
                        amm_program: Pubkey::from_str(JUPITER_V6_PROGRAM).unwrap(),
                        pool_address: None,
                    });
                }
            }
        }

        let swap_type = if intermediate_mints.is_empty() {
            SwapType::SingleHop
        } else {
            SwapType::MultiHop
        };

        Some(SwapPath {
            swap_type,
            hops,
            total_wsol_input: wsol_input,
            total_wsol_output: wsol_output,
            intermediate_mints,
        })
    }

    /// 估算中間代幣數量
    fn estimate_intermediate_amount(&self, balance_changes: &[TokenBalanceChange], mint: Pubkey) -> Decimal {
        balance_changes
            .iter()
            .find(|change| change.mint == mint)
            .map(|change| change.change.abs())
            .unwrap_or_default()
    }
}

/// Jupiter SWAP 信息
#[derive(Debug, Clone)]
struct JupiterSwapInfo {
    /// 輸入金額
    in_amount: Decimal,
    /// 預期輸出金額
    quoted_out_amount: Decimal,
    /// 路徑步驟數
    route_steps: usize,
    /// 滑點（基點）
    slippage_bps: u16,
} 