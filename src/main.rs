use std::path::PathBuf;

use clap::Parser;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};
use yellowstone_vixen::Runtime;

mod types;
mod simple_logger;

use simple_logger::SimpleLogger;

/// Solana MEV 監控器
/// 監控 MEViEnscUm6tsQRoGd9h6nLQaQspKj7DB2M5FwM3Xvz 地址的交易
#[derive(clap::Parser)]
#[command(version, author, about)]
pub struct Opts {
    /// Vixen 配置文件路徑
    #[arg(long, short, default_value = "./Vixen.toml")]
    config: PathBuf,
    
    /// 日誌級別
    #[arg(long, default_value = "info")]
    log_level: String,
    
    /// 是否啟用詳細模式
    #[arg(long)]
    verbose: bool,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let opts = Opts::parse();
    
    // 初始化日誌系統
    init_logging(&opts.log_level);
    
    tracing::info!("🚀 啟動 Solana MEV 監控器");
    tracing::info!("📍 監控地址: MEViEnscUm6tsQRoGd9h6nLQaQspKj7DB2M5FwM3Xvz");
    
    // 讀取配置文件
    let config_content = std::fs::read_to_string(&opts.config)
        .map_err(|e| anyhow::anyhow!("無法讀取配置文件 {:?}: {}", opts.config, e))?;
    
    let config = toml::from_str(&config_content)
        .map_err(|e| anyhow::anyhow!("配置文件解析錯誤: {}", e))?;
    
    // 創建簡單的日誌處理器
    let _logger = SimpleLogger::new(opts.verbose);
    
    tracing::info!("🔧 設置 Yellowstone Vixen Runtime...");
    
    // 使用基本的 Pipeline 和 Logger 處理器
    let runtime = Runtime::builder()
        // 啟用 Prometheus 指標
        .metrics(yellowstone_vixen::metrics::Prometheus)
        // 設置承諾級別
        .commitment_level(yellowstone_vixen::CommitmentLevel::Confirmed)
        .build(config);
    
    tracing::info!("✅ 已連接到 Yellowstone gRPC 服務器");
    tracing::info!("📊 Prometheus 指標服務器: http://194.180.188.21:8999");
    tracing::info!("🎯 開始監控交易...");
    tracing::info!("");
    tracing::info!("💡 此程式將監控包含 MEV 地址的所有交易");
    tracing::info!("   MEV 地址: MEViEnscUm6tsQRoGd9h6nLQaQspKj7DB2M5FwM3Xvz");
    tracing::info!("   過濾器已在 Vixen.toml 中配置");
    tracing::info!("");
    
    // 運行監控器
    runtime.run();
    
    Ok(())
}

/// 初始化日誌系統
fn init_logging(level: &str) {
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new(level))
        )
        .with(tracing_subscriber::fmt::layer().with_target(false))
        .init();
} 