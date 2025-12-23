# Weight Sensor Pump Controller

Raspberry Pi上で動作する重量センサーと蠕動ポンプを使用した液体計量装置の制御システム。

## 特徴

- HX711 ADコンバータを使用した高精度重量測定（最大100g、1g精度）
- 蠕動ポンプの自動制御
- ボタンによる開始/停止制御
- 移動平均フィルタによるノイズ除去
- エラーハンドリングと自動再試行
- クロスコンパイル対応（ARM64/ARM32）
- 設定ファイル対応

## ハードウェア要件

### 推奨GPIO接続

| デバイス | GPIO端子 | 説明 |
|---------|---------|------|
| HX711 DT | GPIO 5 | データ端子 |
| HX711 SCK | GPIO 6 | クロック端子 |
| ポンプ制御 | GPIO 18 | ポンプON/OFF制御 |
| ボタン | GPIO 2 | 開始/停止ボタン（プルアップ抵抗付き） |

### 電源接続

- HX711 VCC → 3.3V
- HX711 GND → GND
- ポンプ → 適切な電源（リレー経由推奨）

## インストール

### 1. クロスコンパイル環境のセットアップ

```bash
# 必要なツールチェーンをインストール
make setup

# または手動で
rustup target add aarch64-unknown-linux-gnu
sudo apt-get install gcc-aarch64-linux-gnu
```

### 2. ビルド

```bash
# Raspberry Pi 4+ (ARM64) 用
make build-arm64

# Raspberry Pi 3以前 (ARM32) 用
make build-arm32

# 両方のターゲット用
make build-all

# 開発用（ローカル）
make build-dev
```

### 3. デプロイ

```bash
# SSH経由でRaspberry Piにデプロイ
make deploy-arm64

# または手動で
scp target/aarch64-unknown-linux-gnu/release/weight-sensor-pump-controller pi@raspberrypi.local:~/
```

## 設定

### 環境変数

```bash
export TARGET_WEIGHT=75.0  # 目標重量（グラム）
export DT_PIN=5            # HX711 DT端子
export SCK_PIN=6           # HX711 SCK端子
export PUMP_PIN=18         # ポンプ制御端子
export BUTTON_PIN=2        # ボタン端子
```

### 設定ファイル

`config.toml`ファイルを作成：

```toml
target_weight = 50.0

[gpio]
dt_pin = 5
sck_pin = 6
pump_pin = 18
button_pin = 2

[sensor]
calibration_factor = 1.0
moving_average_window = 5
max_weight = 100.0
```

## 使用方法

### 基本的な使用方法

1. 容器を重量センサーに置く
2. ボタンを押して計量開始
3. 目標重量に達すると自動停止
4. 緊急停止はボタンを再度押下

### コマンドライン実行

```bash
# デフォルト設定で実行
./weight-sensor-pump-controller

# 環境変数で設定を指定
TARGET_WEIGHT=75.0 ./weight-sensor-pump-controller

# 設定ファイルを使用
./weight-sensor-pump-controller  # config.tomlを自動読み込み
```

### ログレベル設定

```bash
# デバッグログを有効化
RUST_LOG=debug ./weight-sensor-pump-controller

# 情報レベルのみ
RUST_LOG=info ./weight-sensor-pump-controller
```

## 開発

### テスト実行

```bash
# 単体テスト
make test

# プロパティベーステスト
make test-props

# すべてのテスト
cargo test
```

### デバッグビルド

```bash
# デバッグモードでビルド
BUILD_MODE=debug make build-arm64
```

### Docker使用

```bash
# Dockerでクロスコンパイル
make build-docker
```

## トラブルシューティング

### よくある問題

1. **GPIO権限エラー**
   ```bash
   sudo usermod -a -G gpio $USER
   # ログアウト/ログインが必要
   ```

2. **HX711通信エラー**
   - 配線を確認
   - 電源電圧を確認（3.3V）
   - センサーの校正を実行

3. **ポンプが動作しない**
   - GPIO設定を確認
   - リレーの動作を確認
   - 電源供給を確認

### ログ確認

```bash
# システムログ
journalctl -u weight-sensor-pump-controller

# アプリケーションログ
RUST_LOG=debug ./weight-sensor-pump-controller 2>&1 | tee app.log
```

## ライセンス

このプロジェクトはMITライセンスの下で公開されています。

## 貢献

バグ報告や機能要求は、GitHubのIssueでお知らせください。

## 技術仕様

- **言語**: Rust
- **最小Rustバージョン**: 1.70+
- **対応アーキテクチャ**: ARM64, ARM32
- **対応OS**: Ubuntu 20.04+, Raspberry Pi OS
- **依存関係**: rppal, tokio, thiserror, serde, toml