use crate::types::*;
use tracing::{info, debug};
use rust_decimal::Decimal;
use std::sync::Mutex;

/// 簡單的日誌處理器
/// 記錄所有接收到的數據並分析 MEV 相關交易
#[derive(Debug)]
pub struct SimpleLogger {
    /// 是否啟用詳細日誌
    verbose: bool,
    /// 交易計數器
    transaction_count: Mutex<u64>,
    /// MEV 交易計數器
    mev_transaction_count: Mutex<u64>,
}

impl SimpleLogger {
    /// 創建新的簡單日誌處理器
    pub fn new(verbose: bool) -> Self {
        Self {
            verbose,
            transaction_count: Mutex::new(0),
            mev_transaction_count: Mutex::new(0),
        }
    }

    /// 檢查交易是否包含 MEV 地址
    fn contains_mev_address(&self, text: &str) -> bool {
        text.contains(MEV_ADDRESS)
    }

    /// 檢查是否為 Jupiter 相關交易
    fn is_jupiter_transaction(&self, text: &str) -> bool {
        text.contains(JUPITER_V6_PROGRAM)
    }

    /// 格式化和記錄交易信息
    fn log_transaction_info(&self, data: &str) {
        let mut total_count = self.transaction_count.lock().unwrap();
        *total_count += 1;

        if self.contains_mev_address(data) {
            let mut mev_count = self.mev_transaction_count.lock().unwrap();
            *mev_count += 1;

            let is_jupiter = self.is_jupiter_transaction(data);
            
            info!("🎯 MEV 相關交易檢測到！");
            info!("┌─────────────────────────────────────────────────");
            info!("│ 交易序號: #{}", *mev_count);
            info!("│ 總交易數: #{}", *total_count);
            info!("│ MEV 地址: {}", MEV_ADDRESS);
            if is_jupiter {
                info!("│ 🚀 Jupiter V6 聚合器交易");
                info!("│ Jupiter 程序: {}", JUPITER_V6_PROGRAM);
            }
            
            if self.verbose {
                info!("├─────────────────────────────────────────────────");
                info!("│ 原始數據: {}", data);
            }
            
            info!("└─────────────────────────────────────────────────");

            // 每 10 筆 MEV 交易顯示統計
            if *mev_count % 10 == 0 {
                self.show_statistics(*total_count, *mev_count);
            }
        } else if self.verbose {
            debug!("普通交易 #{}: 長度 {} 字符", *total_count, data.len());
        }
    }

    /// 顯示統計信息
    fn show_statistics(&self, total: u64, mev_total: u64) {
        let mev_percentage = if total > 0 {
            (mev_total as f64 / total as f64) * 100.0
        } else {
            0.0
        };

        info!("📊 監控統計 (每 10 筆 MEV 交易更新)");
        info!("├─ 總交易監控數: {}", total);
        info!("├─ MEV 相關交易: {}", mev_total);
        info!("├─ MEV 交易比例: {:.2}%", mev_percentage);
        info!("└─ 監控狀態: 🟢 正常運行");
    }
}

// 實現通用的處理器 trait
// 注意：這裡我們實現一個通用的數據處理器，不依賴特定的 Jupiter 類型

impl std::fmt::Display for SimpleLogger {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "SimpleLogger(verbose: {})", self.verbose)
    }
}

// 為不同類型的數據實現處理功能
impl SimpleLogger {
    /// 處理字符串數據
    pub fn handle_string(&self, data: &str) {
        self.log_transaction_info(data);
    }

    /// 處理字節數據
    pub fn handle_bytes(&self, data: &[u8]) {
        if let Ok(string_data) = std::str::from_utf8(data) {
            self.handle_string(string_data);
        } else if self.verbose {
            debug!("收到二進制數據: {} bytes", data.len());
        }
    }

    /// 處理任意可調試類型的數據
    pub fn handle_debug<T: std::fmt::Debug>(&self, data: &T) {
        let debug_string = format!("{:?}", data);
        self.log_transaction_info(&debug_string);
    }
} 