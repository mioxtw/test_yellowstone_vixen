# Solana MEV 監控器

使用 Yellowstone Vixen 0.4.0 監控 Solana 上的 MEV (Maximum Extractable Value) 交易，專門針對 MEViEnscUm6tsQRoGd9h6nLQaQspKj7DB2M5FwM3Xvz 地址的 SWAP 交易進行分析。

## 功能特性

- 🔍 **實時監控**: 使用 Yellowstone gRPC 即時監控區塊鏈交易
- 🔄 **SWAP 分析**: 自動解析 Jupiter V6 聚合器的 SWAP 路徑
- 📊 **路徑分析**: 識別單跳和多跳交易路徑
- 💰 **收益計算**: 計算 WSOL 投入、產出和淨收益
- 📈 **統計追蹤**: 累計交易統計和套利成功率
- 🚀 **高性能**: 基於 Rust 異步架構，低延遲處理

## 監控目標

- **MEV 地址**: `MEViEnscUm6tsQRoGd9h6nLQaQspKj7DB2M5FwM3Xvz`
- **Jupiter V6**: `JUP6LkbZbjS1jKKwapdHNy74zcZ3tLUZoi5QNyVTaV4`
- **WSOL**: `So11111111111111111111111111111111111111112`

## 系統要求

- Rust 1.70+
- 訪問 Yellowstone gRPC 服務器 (http://194.180.188.21:10000)
- 訪問 Prometheus 指標服務器 (http://194.180.188.21:8999)

## 安裝與設置

### 1. 克隆項目

```bash
git clone <repository-url>
cd solana-mev-monitor
```

### 2. 編譯項目

```bash
cargo build --release
```

### 3. 配置設置

編輯 `Vixen.toml` 文件，確保以下設置正確：

```toml
[yellowstone]
endpoint = "http://194.180.188.21:10000"
x_token = ""  # 如果需要認證，請填入 token

[metrics]
bind_address = "0.0.0.0:8999"
endpoint = "http://194.180.188.21:8999"
```

## 使用方法

### 基本運行

```bash
# 使用默認配置
cargo run

# 或者使用編譯後的二進制文件
./target/release/solana-mev-monitor
```

### 命令行選項

```bash
# 指定配置文件
cargo run -- --config ./my-config.toml

# 啟用詳細日誌
cargo run -- --verbose

# 設置日誌級別
cargo run -- --log-level debug

# 查看幫助
cargo run -- --help
```

### 日誌設置

支持以下日誌級別：
- `error`: 僅錯誤信息
- `warn`: 警告和錯誤
- `info`: 基本運行信息（默認）
- `debug`: 詳細調試信息
- `trace`: 最詳細的追蹤信息

## 輸出示例

### MEV 交易檢測

```
💰 MEV 交易檢測到
┌─────────────────────────────────────────────────
│ 交易簽名: 5K7mX...abc123
│ 區塊槽位: 12345678
│ 交易時間: 2024-01-15 14:30:45 UTC
│ 交易類型: ✅ 套利
├─────────────────────────────────────────────────
│ 📊 SWAP 路徑分析:
│ 多跳: WSOL → EPjF...VZf2 → WSOL
│ 跳躍數量: 2
│   跳躍 1: WSOL → EPjF...VZf2
│           輸入: 10.50 | 輸出: 2500.00
│   跳躍 2: EPjF...VZf2 → WSOL
│           輸入: 2500.00 | 輸出: 11.25
├─────────────────────────────────────────────────
│ 💎 WSOL 流動分析:
│ 總投入 WSOL: 10.50 SOL
│ 總獲得 WSOL: 11.25 SOL
│ WSOL 收益: +0.75 SOL
│ 交易費用: 0.005 SOL
│ 淨收益: +0.745 SOL
├─────────────────────────────────────────────────
│ 🔄 中間代幣:
│   1: EPjF...VZf2
└─────────────────────────────────────────────────
```

### 統計信息

```
📈 累計統計 (每 10 筆交易更新)
├─ 總交易數: 50
├─ 成功套利: 32 (64.0%)
├─ 單跳交易: 15 | 多跳交易: 35
├─ 總 WSOL 收益: +125.50 SOL
├─ 最大收益: +15.25 SOL
├─ 最大損失: -2.10 SOL
└─ 平均交易大小: 8.75 SOL
```

## 數據解析說明

### SWAP 路徑類型

1. **單跳交易**: 直接從 A 代幣交換到 B 代幣
2. **多跳交易**: 通過一個或多個中間代幣進行交換

### 套利檢測

程式會自動檢測套利交易：
- 開始代幣為 WSOL
- 結束代幣為 WSOL
- 計算淨收益（扣除交易費用）

### 收益計算

- **WSOL 收益**: 總輸出 WSOL - 總輸入 WSOL
- **淨收益**: WSOL 收益 - 交易費用（以 SOL 計算）

## 配置說明

### Yellowstone 設置

```toml
[yellowstone]
endpoint = "http://194.180.188.21:10000"  # gRPC 服務器地址
x_token = ""                              # 認證 token
commitment_level = "confirmed"            # 確認級別
reconnect_attempts = 10                   # 重連次數
max_decoding_message_size = 67108864      # 最大消息大小
```

### 監控過濾器

```toml
[accounts.mev_monitor]
pubkeys = ["MEViEnscUm6tsQRoGd9h6nLQaQspKj7DB2M5FwM3Xvz"]

[transactions.mev_transactions]
account_include = ["MEViEnscUm6tsQRoGd9h6nLQaQspKj7DB2M5FwM3Xvz"]
vote = false
failed = false
```

## 故障排除

### 連接問題

1. 確認 gRPC 服務器地址和端口正確
2. 檢查網絡連接和防火牆設置
3. 驗證認證 token（如果需要）

### 沒有數據

1. 確認監控地址是否活躍
2. 檢查過濾器配置
3. 嘗試降低承諾級別到 "processed"

### 性能問題

1. 增加 `max_decoding_message_size` 設置
2. 調整日誌級別到 "warn" 或 "error"
3. 關閉詳細模式 (`--verbose`)

## 技術架構

### 核心組件

- **main.rs**: 程式入口點和配置管理
- **mev_handler.rs**: MEV 交易處理器
- **swap_analyzer.rs**: SWAP 路徑分析器
- **types.rs**: 數據結構定義

### 依賴庫

- `yellowstone-vixen`: Solana 程式解析框架
- `tokio`: 異步運行時
- `tracing`: 日誌系統
- `rust_decimal`: 高精度數值計算
- `solana-sdk`: Solana 開發工具包

## 開發說明

### 添加新功能

1. 修改 `types.rs` 定義新的數據結構
2. 在 `swap_analyzer.rs` 中擴展分析邏輯
3. 更新 `mev_handler.rs` 的顯示格式

### 自定義監控地址

修改 `types.rs` 中的 `MEV_ADDRESS` 常數：

```rust
pub const MEV_ADDRESS: &str = "your-address-here";
```

### 調試模式

使用環境變量啟用調試：

```bash
RUST_LOG=debug cargo run
```

## 許可證

[MIT License](LICENSE)

## 貢獻

歡迎提交 Issue 和 Pull Request！

## 聯絡

如有問題或建議，請通過 GitHub Issues 聯絡。 