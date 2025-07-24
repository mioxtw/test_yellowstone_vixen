use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use solana_sdk::pubkey::Pubkey;
use std::collections::HashMap;

/// MEV 監控的目標地址
pub const MEV_ADDRESS: &str = "MEViEnscUm6tsQRoGd9h6nLQaQspKj7DB2M5FwM3Xvz";

/// WSOL (Wrapped SOL) Mint 地址
pub const WSOL_MINT: &str = "So11111111111111111111111111111111111111112";

/// Jupiter V6 聚合器程序地址
pub const JUPITER_V6_PROGRAM: &str = "JUP6LkbZbjS1jKKwapdHNy74zcZ3tLUZoi5QNyVTaV4";

/// SWAP 交易類型
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum SwapType {
    /// 單跳交易（直接 A -> B）
    SingleHop,
    /// 多跳交易（A -> C -> B）
    MultiHop,
}

/// SWAP 路徑中的一跳
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SwapHop {
    /// 輸入代幣 mint
    pub input_mint: Pubkey,
    /// 輸出代幣 mint
    pub output_mint: Pubkey,
    /// 輸入數量
    pub input_amount: Decimal,
    /// 輸出數量
    pub output_amount: Decimal,
    /// 使用的 AMM 程序
    pub amm_program: Pubkey,
    /// 流動性池地址
    pub pool_address: Option<Pubkey>,
}

/// 完整的 SWAP 路徑
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SwapPath {
    /// SWAP 類型
    pub swap_type: SwapType,
    /// 所有跳躍
    pub hops: Vec<SwapHop>,
    /// 總輸入 WSOL 數量
    pub total_wsol_input: Decimal,
    /// 總輸出 WSOL 數量  
    pub total_wsol_output: Decimal,
    /// 中間代幣 mints（用於多跳）
    pub intermediate_mints: Vec<Pubkey>,
}

/// MEV 交易分析結果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MevTransaction {
    /// 交易簽名
    pub signature: String,
    /// 區塊槽位
    pub slot: u64,
    /// 時間戳
    pub timestamp: i64,
    /// SWAP 路徑
    pub swap_path: SwapPath,
    /// WSOL 收益（可能是負數，表示損失）
    pub wsol_profit: Decimal,
    /// 交易費用（lamports）
    pub transaction_fee: u64,
    /// 淨收益（收益 - 費用）
    pub net_profit: Decimal,
    /// 是否是套利交易
    pub is_arbitrage: bool,
}

/// 代幣餘額變化
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenBalanceChange {
    /// 代幣 mint
    pub mint: Pubkey,
    /// 帳戶地址
    pub account: Pubkey,
    /// 交易前餘額
    pub pre_balance: Decimal,
    /// 交易後餘額
    pub post_balance: Decimal,
    /// 變化量
    pub change: Decimal,
}

/// 帳戶餘額變化映射
pub type BalanceChanges = HashMap<Pubkey, Vec<TokenBalanceChange>>;

/// SWAP 統計信息
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SwapStatistics {
    /// 總交易數
    pub total_transactions: u64,
    /// 成功的套利交易數
    pub successful_arbitrages: u64,
    /// 總 WSOL 收益
    pub total_wsol_profit: Decimal,
    /// 單跳交易數
    pub single_hop_count: u64,
    /// 多跳交易數
    pub multi_hop_count: u64,
    /// 最大單筆收益
    pub max_profit: Decimal,
    /// 最大單筆損失
    pub max_loss: Decimal,
    /// 平均交易大小（WSOL）
    pub average_trade_size: Decimal,
}

impl SwapPath {
    /// 計算路徑的總 WSOL 收益
    pub fn calculate_wsol_profit(&self) -> Decimal {
        self.total_wsol_output - self.total_wsol_input
    }
    
    /// 檢查是否為套利交易（開始和結束都是 WSOL）
    pub fn is_arbitrage(&self) -> bool {
        if self.hops.is_empty() {
            return false;
        }
        
        let first_hop = &self.hops[0];
        let last_hop = &self.hops[self.hops.len() - 1];
        
        first_hop.input_mint.to_string() == WSOL_MINT 
            && last_hop.output_mint.to_string() == WSOL_MINT
    }
    
    /// 獲取路徑描述字符串
    pub fn get_path_description(&self) -> String {
        if self.hops.is_empty() {
            return "空路徑".to_string();
        }
        
        let mut path_parts = Vec::new();
        
        for (i, hop) in self.hops.iter().enumerate() {
            if i == 0 {
                path_parts.push(format!("{}", hop.input_mint));
            }
            path_parts.push(format!("{}", hop.output_mint));
        }
        
        match self.swap_type {
            SwapType::SingleHop => format!("單跳: {}", path_parts.join(" → ")),
            SwapType::MultiHop => format!("多跳: {}", path_parts.join(" → ")),
        }
    }
}

impl MevTransaction {
    /// 創建新的 MEV 交易記錄
    pub fn new(
        signature: String,
        slot: u64,
        swap_path: SwapPath,
        transaction_fee: u64,
    ) -> Self {
        let wsol_profit = swap_path.calculate_wsol_profit();
        let net_profit = wsol_profit - Decimal::from(transaction_fee) / Decimal::new(1_000_000_000, 0); // 轉換 lamports 到 SOL
        let is_arbitrage = swap_path.is_arbitrage();
        
        Self {
            signature,
            slot,
            timestamp: chrono::Utc::now().timestamp(),
            swap_path,
            wsol_profit,
            transaction_fee,
            net_profit,
            is_arbitrage,
        }
    }
}

impl SwapStatistics {
    /// 更新統計信息
    pub fn update(&mut self, transaction: &MevTransaction) {
        self.total_transactions += 1;
        
        if transaction.is_arbitrage && transaction.net_profit > Decimal::ZERO {
            self.successful_arbitrages += 1;
        }
        
        self.total_wsol_profit += transaction.wsol_profit;
        
        match transaction.swap_path.swap_type {
            SwapType::SingleHop => self.single_hop_count += 1,
            SwapType::MultiHop => self.multi_hop_count += 1,
        }
        
        if transaction.wsol_profit > self.max_profit {
            self.max_profit = transaction.wsol_profit;
        }
        
        if transaction.wsol_profit < self.max_loss {
            self.max_loss = transaction.wsol_profit;
        }
        
        // 更新平均交易大小
        if self.total_transactions > 0 {
            let total_volume = self.average_trade_size * Decimal::from(self.total_transactions - 1) + transaction.swap_path.total_wsol_input;
            self.average_trade_size = total_volume / Decimal::from(self.total_transactions);
        } else {
            self.average_trade_size = transaction.swap_path.total_wsol_input;
        }
    }
} 