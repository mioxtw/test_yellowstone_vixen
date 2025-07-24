use crate::types::*;
use rust_decimal::Decimal;
use std::sync::Mutex;
use tracing::{info, debug};
use yellowstone_vixen::{Handler, HandlerResult};
use yellowstone_vixen_jupiter_swap_parser::{
    accounts_parser::JupiterProgramState,
    instructions_parser::JupiterProgramIx,
};

/// MEV 交易處理器
#[derive(Debug)]
pub struct MevHandler {
    /// 是否啟用詳細日誌
    verbose: bool,
    /// 統計信息
    statistics: Mutex<SwapStatistics>,
    /// 已處理的交易簽名（避免重複處理）
    processed_signatures: Mutex<std::collections::HashSet<String>>,
}

impl MevHandler {
    /// 創建新的 MEV 處理器
    pub fn new(verbose: bool) -> Self {
        Self {
            verbose,
            statistics: Mutex::new(SwapStatistics::default()),
            processed_signatures: Mutex::new(std::collections::HashSet::new()),
        }
    }

    /// 處理帳戶更新
    fn process_account_update(&self, pubkey: &str, slot: u64, data: &[u8]) {
        if self.verbose {
            debug!("帳戶更新: {} (槽位: {})", pubkey, slot);
        }

        // 檢查是否是我們監控的 MEV 地址
        if pubkey == MEV_ADDRESS {
            info!("🔍 MEV 地址帳戶更新檢測到");
            info!("   地址: {}", pubkey);
            info!("   槽位: {}", slot);
            info!("   數據大小: {} bytes", data.len());
        }
    }

    /// 處理指令更新
    fn process_instruction_update(&self, program_id: &str, data: &[u8], accounts: &[String]) {
        if self.verbose {
            debug!("指令更新: 程序 {}", program_id);
        }

        // 檢查是否是 Jupiter V6 程序
        if program_id == JUPITER_V6_PROGRAM {
            // 檢查帳戶中是否包含 MEV 地址
            if accounts.contains(&MEV_ADDRESS.to_string()) {
                info!("💎 MEV 相關的 Jupiter 交易檢測到");
                info!("   程序: {}", program_id);
                info!("   指令數據大小: {} bytes", data.len());
                info!("   相關帳戶數: {}", accounts.len());
                
                if self.verbose {
                    info!("   帳戶列表:");
                    for (i, account) in accounts.iter().enumerate() {
                        info!("     [{}] {}", i, account);
                    }
                }

                // 分析指令數據
                self.analyze_jupiter_instruction(data, accounts);
            }
        }
    }

    /// 分析 Jupiter 指令
    fn analyze_jupiter_instruction(&self, data: &[u8], accounts: &[String]) {
        if data.len() < 8 {
            return;
        }

        // 解析指令判別符
        let discriminator = &data[0..8];
        let instruction_type = match discriminator {
            [0x22, 0x5c, 0x7b, 0x45, 0x8a, 0x4e, 0x7f, 0x18] => "Route",
            [0x3a, 0x4b, 0x15, 0x2e, 0x77, 0xc4, 0x9a, 0x6d] => "SharedAccountsRoute",
            _ => "Unknown",
        };

        info!("🔄 Jupiter SWAP 分析:");
        info!("   指令類型: {}", instruction_type);
        
        if instruction_type != "Unknown" {
            // 嘗試解析基本參數
                         if data.len() >= 24 {
                 let in_amount = u64::from_le_bytes(data[8..16].try_into().unwrap_or([0; 8]));
                 let quoted_out_amount = u64::from_le_bytes(data[16..24].try_into().unwrap_or([0; 8]));
                 {
                    let in_sol = Decimal::from(in_amount) / Decimal::new(1_000_000_000, 0);
                    let out_sol = Decimal::from(quoted_out_amount) / Decimal::new(1_000_000_000, 0);
                    let profit = out_sol - in_sol;

                    info!("   輸入金額: {} SOL", self.format_amount(in_sol));
                    info!("   預期輸出: {} SOL", self.format_amount(out_sol));
                    info!("   預期收益: {} SOL", self.format_profit(profit));

                    // 分析路徑
                    self.analyze_swap_path(accounts);

                    // 更新統計
                    self.update_statistics(in_sol, out_sol, profit);
                }
            }
        }

        info!("   ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    }

    /// 分析 SWAP 路徑
    fn analyze_swap_path(&self, accounts: &[String]) {
        // 尋找 WSOL 和其他代幣 mint
        let wsol_mint = WSOL_MINT;
        let mut token_mints = Vec::new();
        let mut is_wsol_to_wsol = false;

        for account in accounts {
            if account == wsol_mint {
                // WSOL 相關
                continue;
            }
            // 這裡可以添加更復雜的代幣識別邏輯
            if account.len() == 44 && !token_mints.contains(account) {
                token_mints.push(account.clone());
            }
        }

        // 判斷路徑類型
        let path_type = if token_mints.is_empty() {
            is_wsol_to_wsol = true;
            "直接 WSOL 套利"
        } else if token_mints.len() == 1 {
            "單跳 SWAP"
        } else {
            "多跳 SWAP"
        };

        info!("   路徑類型: {}", path_type);
        if is_wsol_to_wsol {
            info!("   ✅ 這是一個套利交易！");
        } else {
            info!("   中間代幣數量: {}", token_mints.len());
            for (i, mint) in token_mints.iter().enumerate() {
                info!("     代幣 {}: {}...{}", i + 1, &mint[0..4], &mint[mint.len()-4..]);
            }
        }
    }

    /// 更新統計信息
    fn update_statistics(&self, _input: Decimal, _output: Decimal, profit: Decimal) {
        let mut stats = self.statistics.lock().unwrap();
        stats.total_transactions += 1;
        
        if profit > Decimal::ZERO {
            stats.successful_arbitrages += 1;
        }
        
        stats.total_wsol_profit += profit;
        
        if profit > stats.max_profit {
            stats.max_profit = profit;
        }
        
        if profit < stats.max_loss {
            stats.max_loss = profit;
        }

        // 每 5 筆交易顯示一次統計
        if stats.total_transactions % 5 == 0 {
            self.display_statistics(&stats);
        }
    }

    /// 格式化數量
    fn format_amount(&self, amount: Decimal) -> String {
        if amount == Decimal::ZERO {
            "0".to_string()
        } else if amount < Decimal::new(1, 3) { // < 0.001
            format!("{:.6}", amount)
        } else if amount < Decimal::ONE {
            format!("{:.4}", amount)
        } else {
            format!("{:.2}", amount)
        }
    }

    /// 格式化收益（帶符號）
    fn format_profit(&self, profit: Decimal) -> String {
        if profit > Decimal::ZERO {
            format!("+{}", self.format_amount(profit))
        } else if profit < Decimal::ZERO {
            format!("{}", self.format_amount(profit))
        } else {
            "0".to_string()
        }
    }

    /// 顯示統計信息
    fn display_statistics(&self, stats: &SwapStatistics) {
        info!("📈 累計統計 (每 5 筆交易更新)");
        info!("├─ 總交易數: {}", stats.total_transactions);
        info!("├─ 可能套利: {} ({:.1}%)", 
              stats.successful_arbitrages, 
              if stats.total_transactions > 0 { 
                  stats.successful_arbitrages as f64 / stats.total_transactions as f64 * 100.0 
              } else { 0.0 });
        info!("├─ 總預期收益: {} SOL", self.format_profit(stats.total_wsol_profit));
        info!("├─ 最大收益: {} SOL", self.format_amount(stats.max_profit));
        info!("└─ 最大損失: {} SOL", self.format_amount(stats.max_loss));
    }
}

// 為不同的更新類型實現 Handler trait
// 注意：這些類型可能需要根據實際的 yellowstone-vixen API 調整

impl Handler<Vec<u8>> for MevHandler {
    async fn handle(&self, data: &Vec<u8>) -> HandlerResult<()> {
        // 處理原始數據
        if self.verbose {
            debug!("收到原始數據: {} bytes", data.len());
        }
        Ok(())
    }
}

impl Handler<String> for MevHandler {
    async fn handle(&self, message: &String) -> HandlerResult<()> {
        // 處理字符串消息
        if self.verbose {
            debug!("收到消息: {}", message);
        }
        Ok(())
    }
}

// 為 Jupiter 特定的類型實現 Handler
impl Handler<JupiterProgramState> for MevHandler {
    async fn handle(&self, state: &JupiterProgramState) -> HandlerResult<()> {
        // 使用 Debug 輸出來查看實際結構
        if self.verbose {
            debug!("收到 JupiterProgramState: {:?}", state);
        }
        info!("🔍 Jupiter 程序狀態更新檢測到");
        Ok(())
    }
}

impl Handler<JupiterProgramIx> for MevHandler {
    async fn handle(&self, ix: &JupiterProgramIx) -> HandlerResult<()> {
        // 使用 Debug 輸出來查看實際結構
        if self.verbose {
            debug!("收到 JupiterProgramIx: {:?}", ix);
        }
        info!("💎 Jupiter 指令檢測到");
        Ok(())
    }
}

// 為 Arc<MevHandler> 實現 Handler
impl Handler<JupiterProgramState> for std::sync::Arc<MevHandler> {
    async fn handle(&self, state: &JupiterProgramState) -> HandlerResult<()> {
        (**self).handle(state).await
    }
}

impl Handler<JupiterProgramIx> for std::sync::Arc<MevHandler> {
    async fn handle(&self, ix: &JupiterProgramIx) -> HandlerResult<()> {
        (**self).handle(ix).await
    }
} 