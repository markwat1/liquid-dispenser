# 設計書

## 概要

Raspberry Pi上で動作する重量センサー制御システムは、HX711 ADコンバータを使用した重量測定と蠕動ポンプの自動制御を組み合わせた液体計量装置です。システムはRust言語で実装され、クロスコンパイルによってARM64バイナリとして配布されます。

## アーキテクチャ

システムは以下の3つの主要レイヤーで構成されます：

```
┌─────────────────────────────────────┐
│        Application Layer            │
│  (Main Control Logic & State)       │
├─────────────────────────────────────┤
│        Hardware Abstraction        │
│  (GPIO, HX711, Pump, Button)       │
├─────────────────────────────────────┤
│        System Layer                 │
│  (Linux GPIO, Cross-compiled)       │
└─────────────────────────────────────┘
```

### アーキテクチャの特徴

- **非同期処理**: 重量監視、ボタン監視、ポンプ制御を並行実行
- **状態管理**: 明確な状態遷移による安全な動作制御
- **エラー処理**: 通信エラーや範囲外値に対する堅牢な処理
- **クロスプラットフォーム**: 開発環境とターゲット環境の分離

## コンポーネントとインターフェース

### 1. WeightSensorController
HX711との通信を管理し、重量データを取得・処理します。

```rust
pub struct WeightSensorController {
    dt_pin: u8,
    sck_pin: u8,
    calibration_factor: f32,
    moving_average: MovingAverage,
}

impl WeightSensorController {
    pub fn new(dt_pin: u8, sck_pin: u8) -> Self;
    pub fn read_weight(&mut self) -> Result<f32, SensorError>;
    pub fn calibrate_zero(&mut self) -> Result<(), SensorError>;
    pub fn is_in_range(&self, weight: f32) -> bool;
}
```

### 2. PumpController
蠕動ポンプのON/OFF制御を管理します。

```rust
pub struct PumpController {
    control_pin: u8,
    is_running: bool,
}

impl PumpController {
    pub fn new(control_pin: u8) -> Self;
    pub fn start(&mut self) -> Result<(), PumpError>;
    pub fn stop(&mut self) -> Result<(), PumpError>;
    pub fn is_running(&self) -> bool;
}
```

### 3. ButtonController
制御ボタンの状態監視を行います。

```rust
pub struct ButtonController {
    button_pin: u8,
    last_state: bool,
    debounce_timer: Instant,
}

impl ButtonController {
    pub fn new(button_pin: u8) -> Self;
    pub fn is_pressed(&mut self) -> bool;
    pub fn wait_for_press(&mut self) -> Result<(), ButtonError>;
}
```

### 4. SystemController
全体の制御ロジックと状態管理を行います。

```rust
pub struct SystemController {
    weight_sensor: WeightSensorController,
    pump: PumpController,
    button: ButtonController,
    target_weight: f32,
    state: SystemState,
}

#[derive(Debug, Clone, PartialEq)]
pub enum SystemState {
    Idle,
    Measuring,
    Pumping,
    Stopping,
    Error(String),
}
```

## データモデル

### 重量データ
```rust
#[derive(Debug, Clone)]
pub struct WeightReading {
    pub value: f32,        // グラム単位
    pub timestamp: Instant,
    pub is_stable: bool,   // ノイズフィルタ後の安定性
}
```

### システム設定
```rust
#[derive(Debug, Clone)]
pub struct SystemConfig {
    pub target_weight: f32,
    pub dt_pin: u8,
    pub sck_pin: u8,
    pub pump_pin: u8,
    pub button_pin: u8,
    pub calibration_factor: f32,
    pub moving_average_window: usize,
}
```

### GPIO設定
推奨されるGPIO接続：
- HX711 DT端子 → GPIO 5
- HX711 SCK端子 → GPIO 6
- ポンプ制御 → GPIO 18
- ボタン → GPIO 2 (プルアップ抵抗付き)

## 正確性プロパティ

*プロパティとは、システムのすべての有効な実行において真であるべき特性や動作です。これらは人間が読める仕様と機械で検証可能な正確性保証の橋渡しとなります。*

### プロパティ反映

分析したプロパティを確認し、冗長性を排除します：

- プロパティ1（ボタン押下による開始）とプロパティ4（ボタン押下による停止）は、ボタン処理の包括的なプロパティに統合可能
- プロパティ2（注入開始時のポンプ動作）とプロパティ3（目標重量でのポンプ停止）は、ポンプ制御の包括的なプロパティに統合可能
- プロパティ11（測定精度）とプロパティ12（ノイズフィルタ）は、重量測定の包括的なプロパティに統合可能

### プロパティ 1: ボタン制御の一貫性
*任意の*システム状態において、Control_Buttonを押下した場合、システムは適切な状態遷移を実行する（待機中なら開始、動作中なら停止）
**検証対象: 要件 1.1, 1.4**

### プロパティ 2: ポンプ制御の自動化
*任意の*重量測定値において、測定値が目標重量に達した場合、Peristaltic_Pumpは自動的に停止し、測定値が目標重量未満の場合は動作を継続する
**検証対象: 要件 1.2, 1.3**

### プロパティ 3: 重量監視の継続性
*任意の*システム状態において、Weight_Sensor_Systemは継続的に重量を監視し、1g精度で0-100g範囲の測定値を提供する
**検証対象: 要件 1.5, 2.5, 5.1**

### プロパティ 4: GPIO初期化の安全性
*任意の*システム起動時において、すべてのGPIO_Pinは安全な初期状態に設定され、各デバイスとの通信が確立される
**検証対象: 要件 3.4, 3.5**

### プロパティ 5: クロスコンパイルの一貫性
*任意の*目標重量設定において、コンパイル時に指定された値がバイナリに正しく組み込まれ、依存関係なしに動作する
**検証対象: 要件 4.3, 4.4**

### プロパティ 6: ノイズフィルタリングの安定性
*任意の*ノイズを含む重量データにおいて、移動平均フィルタが適用され、安定した測定値が出力される
**検証対象: 要件 5.2, 5.3**

### プロパティ 7: エラーハンドリングの堅牢性
*任意の*測定範囲外の値や通信エラーにおいて、システムは適切なエラー状態を報告し、再試行処理を実行する
**検証対象: 要件 5.4, 5.5**

## エラーハンドリング

### エラータイプ
```rust
#[derive(Debug, thiserror::Error)]
pub enum SystemError {
    #[error("Sensor error: {0}")]
    Sensor(#[from] SensorError),
    
    #[error("Pump error: {0}")]
    Pump(#[from] PumpError),
    
    #[error("Button error: {0}")]
    Button(#[from] ButtonError),
    
    #[error("GPIO error: {0}")]
    Gpio(String),
    
    #[error("Configuration error: {0}")]
    Config(String),
}
```

### エラー処理戦略
1. **通信エラー**: 最大3回の再試行後、エラー状態に遷移
2. **範囲外値**: エラーログ出力後、前回の有効値を使用
3. **ハードウェア障害**: 安全停止後、エラー状態で待機
4. **設定エラー**: 起動時にバリデーション、不正値は既定値で代替

## テスト戦略

### 単体テスト
- 各コンポーネントの基本機能テスト
- エラー条件での動作確認
- GPIO操作のモックテスト

### プロパティベーステスト
- QuickCheckライブラリを使用
- 各プロパティを最低100回実行
- ランダムな入力値での動作検証

**プロパティベーステストの要件**:
- Rustの`quickcheck`クレートを使用
- 各プロパティテストは最低100回の反復実行
- 各テストには設計書のプロパティ番号を明記
- フォーマット: `**Feature: weight-sensor-pump-controller, Property {number}: {property_text}**`

### 統合テスト
- システム全体の状態遷移テスト
- ハードウェアシミュレータを使用した動作確認
- クロスコンパイル後のバイナリテスト