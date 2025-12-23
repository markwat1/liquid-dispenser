# 使用方法ガイド

## セットアップ

### 1. ハードウェア接続

#### HX711重量センサー
```
HX711    Raspberry Pi
VCC   -> 3.3V (Pin 1)
GND   -> GND (Pin 6)
DT    -> GPIO 5 (Pin 29)
SCK   -> GPIO 6 (Pin 31)
```

#### 蠕動ポンプ
```
ポンプ制御 -> GPIO 18 (Pin 12)
```
注意: ポンプは適切な電源とリレーを使用してください。

#### 制御ボタン
```
ボタン -> GPIO 2 (Pin 3)
GND   -> GND (Pin 9)
```
注意: プルアップ抵抗が内部で有効化されます。

### 2. ソフトウェアインストール

#### 方法1: 事前ビルド済みバイナリ
```bash
# ARM64版（Raspberry Pi 4+）
wget https://github.com/your-repo/releases/latest/weight-sensor-pump-controller-arm64
chmod +x weight-sensor-pump-controller-arm64
sudo mv weight-sensor-pump-controller-arm64 /usr/local/bin/weight-sensor-pump-controller

# ARM32版（Raspberry Pi 3以前）
wget https://github.com/your-repo/releases/latest/weight-sensor-pump-controller-arm32
chmod +x weight-sensor-pump-controller-arm32
sudo mv weight-sensor-pump-controller-arm32 /usr/local/bin/weight-sensor-pump-controller
```

#### 方法2: ソースからビルド
```bash
git clone https://github.com/your-repo/weight-sensor-pump-controller.git
cd weight-sensor-pump-controller
make setup
make build-arm64  # または build-arm32
sudo cp target/aarch64-unknown-linux-gnu/release/weight-sensor-pump-controller /usr/local/bin/
```

## 基本的な使用方法

### 1. 初回セットアップ

#### 校正の実行
```bash
# ゼロ点校正（何も載せない状態で実行）
weight-sensor-pump-controller --calibrate-zero

# 既知の重量での校正（例：50gの分銅を使用）
weight-sensor-pump-controller --calibrate-weight 50.0
```

#### 設定ファイルの作成
```bash
# デフォルト設定ファイルを生成
make config

# 設定ファイルを編集
nano config.toml
```

### 2. 基本操作

#### 手動実行
```bash
# デフォルト設定で実行
./weight-sensor-pump-controller

# 目標重量を指定
TARGET_WEIGHT=75.0 ./weight-sensor-pump-controller

# デバッグモード
RUST_LOG=debug ./weight-sensor-pump-controller
```

#### 操作手順
1. 容器を重量センサーに置く
2. ボタンを押して計量開始
3. 液体が注入される
4. 目標重量に達すると自動停止
5. 緊急停止が必要な場合はボタンを再度押下

### 3. システムサービスとして実行

#### systemdサービスファイルの作成
```bash
sudo nano /etc/systemd/system/weight-sensor-pump-controller.service
```

```ini
[Unit]
Description=Weight Sensor Pump Controller
After=network.target

[Service]
Type=simple
User=pi
Group=gpio
WorkingDirectory=/home/pi
ExecStart=/usr/local/bin/weight-sensor-pump-controller
Restart=always
RestartSec=5
Environment=RUST_LOG=info
Environment=TARGET_WEIGHT=50.0

[Install]
WantedBy=multi-user.target
```

#### サービスの有効化
```bash
sudo systemctl daemon-reload
sudo systemctl enable weight-sensor-pump-controller
sudo systemctl start weight-sensor-pump-controller

# 状態確認
sudo systemctl status weight-sensor-pump-controller

# ログ確認
journalctl -u weight-sensor-pump-controller -f
```

## 高度な設定

### 1. 設定ファイル（config.toml）

```toml
# 基本設定
target_weight = 50.0

# GPIO設定
[gpio]
dt_pin = 5
sck_pin = 6
pump_pin = 18
button_pin = 2

# センサー設定
[sensor]
calibration_factor = 1.0
moving_average_window = 5
max_weight = 100.0

# ポンプ設定
[pump]
max_runtime = 300  # 最大動作時間（秒）
safety_timeout = 5  # 安全停止タイムアウト（秒）

# システム設定
[system]
log_level = "info"
stats_interval = 60  # 統計出力間隔（秒）
health_check_interval = 30  # 健全性チェック間隔（秒）
```

### 2. 環境変数

```bash
# 基本設定
export TARGET_WEIGHT=75.0
export RUST_LOG=debug

# GPIO設定
export DT_PIN=5
export SCK_PIN=6
export PUMP_PIN=18
export BUTTON_PIN=2

# センサー設定
export CALIBRATION_FACTOR=1.0
export MOVING_AVERAGE_WINDOW=5
```

### 3. コマンドライン引数

```bash
# ヘルプ表示
weight-sensor-pump-controller --help

# バージョン表示
weight-sensor-pump-controller --version

# 設定ファイル指定
weight-sensor-pump-controller --config /path/to/config.toml

# 校正モード
weight-sensor-pump-controller --calibrate-zero
weight-sensor-pump-controller --calibrate-weight 50.0

# テストモード（実際のポンプを動作させない）
weight-sensor-pump-controller --test-mode
```

## トラブルシューティング

### 1. よくある問題

#### GPIO権限エラー
```bash
# ユーザーをgpioグループに追加
sudo usermod -a -G gpio $USER

# 再ログインまたは再起動が必要
sudo reboot
```

#### HX711通信エラー
```bash
# 配線確認
# - VCC: 3.3V（5Vではない）
# - GND: 確実に接続
# - DT/SCK: 指定されたGPIOピンに接続

# センサーテスト
RUST_LOG=debug weight-sensor-pump-controller --test-mode
```

#### ポンプが動作しない
```bash
# GPIO出力テスト
echo "18" > /sys/class/gpio/export
echo "out" > /sys/class/gpio/gpio18/direction
echo "1" > /sys/class/gpio/gpio18/value  # ポンプON
echo "0" > /sys/class/gpio/gpio18/value  # ポンプOFF
echo "18" > /sys/class/gpio/unexport
```

### 2. ログ確認

#### アプリケーションログ
```bash
# リアルタイムログ
RUST_LOG=debug weight-sensor-pump-controller 2>&1 | tee app.log

# システムログ
journalctl -u weight-sensor-pump-controller -f

# エラーログのみ
journalctl -u weight-sensor-pump-controller -p err
```

#### デバッグ情報
```bash
# 詳細デバッグ
RUST_LOG=trace weight-sensor-pump-controller

# 特定モジュールのみ
RUST_LOG=weight_sensor_pump_controller::controllers::weight_sensor=debug weight-sensor-pump-controller
```

### 3. 診断コマンド

#### システム情報
```bash
# GPIO状態確認
cat /sys/kernel/debug/gpio

# プロセス確認
ps aux | grep weight-sensor

# ポート使用状況
sudo lsof -i

# システムリソース
top -p $(pgrep weight-sensor)
```

#### ハードウェアテスト
```bash
# I2C/SPIデバイス確認
ls /dev/i2c* /dev/spi*

# GPIO状態
gpio readall  # wiringPiがインストールされている場合
```

## 保守とメンテナンス

### 1. 定期メンテナンス

#### 校正の実行（月1回推奨）
```bash
# システム停止
sudo systemctl stop weight-sensor-pump-controller

# ゼロ点校正
weight-sensor-pump-controller --calibrate-zero

# 既知重量での校正
weight-sensor-pump-controller --calibrate-weight 50.0

# システム再開
sudo systemctl start weight-sensor-pump-controller
```

#### ログローテーション
```bash
# logrotateの設定
sudo nano /etc/logrotate.d/weight-sensor-pump-controller
```

```
/var/log/weight-sensor-pump-controller.log {
    daily
    rotate 7
    compress
    delaycompress
    missingok
    notifempty
    postrotate
        systemctl reload weight-sensor-pump-controller
    endscript
}
```

### 2. バックアップ

#### 設定ファイル
```bash
# 設定バックアップ
cp config.toml config.toml.backup.$(date +%Y%m%d)

# システム設定
sudo cp /etc/systemd/system/weight-sensor-pump-controller.service \
        /etc/systemd/system/weight-sensor-pump-controller.service.backup
```

#### 校正データ
```bash
# 校正データは設定ファイルに保存されるため、
# config.tomlのバックアップで十分
```

### 3. アップデート

#### バイナリ更新
```bash
# サービス停止
sudo systemctl stop weight-sensor-pump-controller

# バイナリ更新
sudo cp new-weight-sensor-pump-controller /usr/local/bin/weight-sensor-pump-controller

# サービス再開
sudo systemctl start weight-sensor-pump-controller

# 動作確認
sudo systemctl status weight-sensor-pump-controller
```

#### 設定ファイル更新
```bash
# 設定ファイルの互換性確認
weight-sensor-pump-controller --check-config config.toml

# 必要に応じて設定を更新
nano config.toml
```

## 安全に関する注意事項

### 1. 電気的安全性
- ポンプは適切な電源とリレーを使用
- 配線は確実に接続し、ショートを避ける
- 湿気の多い環境では防水対策を実施

### 2. 機械的安全性
- 重量センサーの最大荷重を超えない
- ポンプの最大動作時間を設定
- 緊急停止ボタンを常にアクセス可能な位置に配置

### 3. ソフトウェア安全性
- 定期的な校正の実施
- ログの監視
- 異常時の自動停止機能の確認

## サポート

### 1. 問題報告
- GitHub Issues: https://github.com/your-repo/issues
- ログファイルを添付
- ハードウェア構成を明記

### 2. 機能要求
- GitHub Discussions: https://github.com/your-repo/discussions
- 具体的な使用例を記載

### 3. 貢献
- Pull Requests歓迎
- コーディング規約に従う
- テストを含める