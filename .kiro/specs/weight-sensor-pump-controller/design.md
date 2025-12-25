# 設計書

## 概要

Raspberry Pi上で動作する重量センサー制御システムは、HX711 ADコンバータを使用した重量測定とPWM制御によるモーター制御を組み合わせた液体計量装置です。システムはRust言語で実装され、クロスコンパイルによってARM64バイナリとして配布されます。モーターはPWM信号（最大5kHz）による可変速度制御で正転・逆転が可能で、重量安定化処理、目標重量到達後の自動逆転処理を含む完全自動化された計量プロセスを提供します。

## アーキテクチャ

システムは以下の3つの主要レイヤーで構成されます：

```
┌─────────────────────────────────────┐
│        Application Layer            │
│  (Main Control Logic & State)       │
├─────────────────────────────────────┤
│        Hardware Abstraction        │
│  (GPIO, HX711, Motor, Button)       │
├─────────────────────────────────────┤
│        System Layer                 │
│  (Linux GPIO, Cross-compiled)       │
└─────────────────────────────────────┘
```

### アーキテクチャの特徴

- **非同期処理**: 重量監視、ボタン監視、モーター制御を並行実行
- **状態管理**: 明確な状態遷移による安全な動作制御（重量安定化、正転、逆転フェーズ）
- **エラー処理**: 通信エラーや範囲外値に対する堅牢な処理
- **安全制御**: リレーの排他制御による安全なモーター動作
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

### 2. MotorController
PWM信号を使用してモーターの正転・逆転・速度制御を行います。

```rust
pub struct MotorController {
    pwm_forward_pin: Pwm,   // 正転用PWM出力
    pwm_reverse_pin: Pwm,   // 逆転用PWM出力
    current_state: MotorState,
    current_speed: f64,     // 現在の速度（0.0-1.0）
    pwm_frequency: f64,     // PWM周波数（Hz）
}

#[derive(Debug, Clone, PartialEq)]
pub enum MotorState {
    Stopped,           // 両PWM出力0%
    Forward(f64),      // 正転（速度0.0-1.0）
    Reverse(f64),      // 逆転（速度0.0-1.0）
}

impl MotorController {
    pub fn new(forward_pin: u8, reverse_pin: u8, frequency: f64) -> Self;
    pub fn start_forward(&mut self, speed: f64) -> Result<(), MotorError>;
    pub fn start_reverse(&mut self, speed: f64) -> Result<(), MotorError>;
    pub fn set_speed(&mut self, speed: f64) -> Result<(), MotorError>;
    pub fn stop(&mut self) -> Result<(), MotorError>;
    pub fn emergency_stop(&mut self);
    pub fn current_state(&self) -> &MotorState;
    pub fn current_speed(&self) -> f64;
    pub fn set_frequency(&mut self, frequency: f64) -> Result<(), MotorError>;
    pub fn is_safe_state(&self) -> bool;  // 両PWM信号が同時に0%以外でないことを確認
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
全体の制御ロジックと状態管理を行います。重量安定化、正転動作、逆転処理の完全自動化を実現します。

```rust
pub struct SystemController {
    weight_sensor: WeightSensorController,
    motor: MotorController,
    button: ButtonController,
    target_weight: f32,
    state: SystemState,
    base_weight: Option<f32>,  // 安定化後の基準重量
    stabilization_timer: Option<Instant>,
    reverse_timer: Option<Instant>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum SystemState {
    Idle,
    WeightStabilizing,  // 3秒間の重量安定化中
    Forward,           // 正転動作中
    TargetReached,     // 目標重量到達、正転停止
    Reverse,           // 3秒間の逆転動作中
    Completed,         // 処理完了
    Error(String),
}

impl SystemController {
    pub fn start_sequence(&mut self) -> Result<(), SystemError>;
    pub fn process_stabilization(&mut self) -> Result<(), SystemError>;
    pub fn process_forward_operation(&mut self) -> Result<(), SystemError>;
    pub fn process_reverse_operation(&mut self) -> Result<(), SystemError>;
    pub fn check_weight_stability(&self, readings: &[f32]) -> bool;
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
    pub pwm_forward_pin: u8,      // 正転用PWM出力
    pub pwm_reverse_pin: u8,      // 逆転用PWM出力
    pub button_pin: u8,
    pub calibration_factor: f32,
    pub moving_average_window: usize,
    pub stabilization_duration: Duration,  // 重量安定化時間（3秒）
    pub reverse_duration: Duration,        // 逆転動作時間（3秒）
    pub weight_tolerance: f32,             // 重量安定判定の許容範囲
    pub pwm_frequency: f64,                // PWM周波数（最大5kHz）
    pub forward_speed: f64,                // 正転時の速度（0.0-1.0）
    pub reverse_speed: f64,                // 逆転時の速度（0.0-1.0）
}
```

### 制御シーケンス
```rust
#[derive(Debug, Clone)]
pub struct ControlSequence {
    pub phase: SequencePhase,
    pub start_time: Instant,
    pub weight_readings: Vec<f32>,  // 安定化判定用
}

#[derive(Debug, Clone, PartialEq)]
pub enum SequencePhase {
    WaitingForStart,
    Stabilizing,
    ForwardOperation,
    ReverseOperation,
    Completed,
}
```

### GPIO設定
推奨されるGPIO接続：
- HX711 DT端子 → GPIO 5
- HX711 SCK端子 → GPIO 6
- PWM正転出力 → GPIO 18 (PWM0)
- PWM逆転出力 → GPIO 19 (PWM1)
- ボタン → GPIO 2 (プルアップ抵抗付き)

## 正確性プロパティ

*プロパティとは、システムのすべての有効な実行において真であるべき特性や動作です。これらは人間が読める仕様と機械で検証可能な正確性保証の橋渡しとなります。*

### プロパティ反映

分析したプロパティを確認し、冗長性を排除します：

- 重量安定化関連のプロパティ（1.1, 1.2, 5.1-5.5）は包括的な安定化プロパティに統合
- モーター制御シーケンス（1.3, 1.5, 1.6, 1.7）は包括的なシーケンス制御プロパティに統合
- リレー制御（2.1-2.5）は包括的なリレー制御プロパティに統合
- 安全機能（1.8, 4.4, 4.5）は包括的な安全プロパティに統合
- エラーハンドリング（2.4, 7.4, 7.5）は包括的なエラー処理プロパティに統合

### プロパティ 1: 重量安定化処理の一貫性
*任意の*重量測定開始時において、システムは3秒間継続的に測定を実行し、変動が許容範囲内の場合は安定と判定し、その値を基準重量として設定する
**検証対象: 要件 1.1, 1.2, 5.1, 5.2, 5.4, 5.5**

### プロパティ 2: モーター制御シーケンスの自動化
*任意の*制御シーケンスにおいて、基準重量設定完了後は正転開始、目標重量到達時は正転停止、その後3秒間の逆転実行、逆転完了後は完全停止の順序で実行される
**検証対象: 要件 1.3, 1.5, 1.6, 1.7**

### プロパティ 3: PWM制御の安全性と精度
*任意の*PWM制御において、デューティ比0%で停止状態維持、1-100%で指定速度動作、両PWM信号同時非ゼロでエラー検知、1Hz-5kHz範囲での周波数動作が実行される
**検証対象: 要件 2.1, 2.2, 2.3, 2.4, 2.5, 2.6, 2.7, 8.1, 8.2, 8.3, 8.5**

### プロパティ 4: 重量監視の継続性
*任意の*システム動作中において、重量センサーは1g精度で継続的に監視を実行し、ノイズに対しては移動平均フィルタで安定化処理を適用する
**検証対象: 要件 1.4, 7.1, 7.2**

### プロパティ 5: 緊急停止の即応性
*任意の*システム状態において、ボタン押下による緊急停止要求に対してモーターは即座に停止し、システム起動時は安全な初期状態で開始される
**検証対象: 要件 1.8, 4.4, 4.5**

### プロパティ 6: エラーハンドリングの堅牢性
*任意の*エラー条件において、測定範囲外の値や通信エラーに対してシステムは適切なエラー状態を報告し、再試行処理を実行する
**検証対象: 要件 7.4, 7.5**

### プロパティ 7: PWM制御範囲の妥当性
*任意の*PWM制御設定において、0-100%デューティ比範囲と1Hz-5kHz周波数範囲で正確な制御が実行され、コンパイル時に指定された目標重量がバイナリに正しく組み込まれる
**検証対象: 要件 2.6, 8.1, 8.2, 6.4**

## エラーハンドリング

### エラータイプ
```rust
#[derive(Debug, thiserror::Error)]
pub enum SystemError {
    #[error("Sensor error: {0}")]
    Sensor(#[from] SensorError),
    
    #[error("Motor error: {0}")]
    Motor(#[from] MotorError),
    
    #[error("PWM configuration error: {0}")]
    PwmConfig(String),
    
    #[error("Button error: {0}")]
    Button(#[from] ButtonError),
    
    #[error("GPIO error: {0}")]
    Gpio(String),
    
    #[error("Configuration error: {0}")]
    Config(String),
    
    #[error("PWM safety error: {0}")]
    PwmSafety(String),
    
    #[error("Sequence error: {0}")]
    Sequence(String),
}

#[derive(Debug, thiserror::Error)]
pub enum MotorError {
    #[error("PWM control error: {0}")]
    PwmControl(String),
    
    #[error("Unsafe PWM state: both PWM signals active")]
    UnsafeState,
    
    #[error("Invalid speed value: {0}")]
    InvalidSpeed(f64),
    
    #[error("Invalid frequency value: {0}")]
    InvalidFrequency(f64),
    
    #[error("Motor start failure")]
    StartFailure,
    
    #[error("Motor stop failure")]
    StopFailure,
    
    #[error("GPIO error: {0}")]
    Gpio(String),
    
    #[error("PWM frequency out of range: {0} Hz (max 5000 Hz)")]
    FrequencyOutOfRange(f64),
}
```

### エラー処理戦略
1. **通信エラー**: 最大3回の再試行後、エラー状態に遷移
2. **範囲外値**: エラーログ出力後、前回の有効値を使用
3. **ハードウェア障害**: 安全停止後、エラー状態で待機
4. **設定エラー**: 起動時にバリデーション、不正値は既定値で代替
5. **PWM安全エラー**: 両PWM信号同時非ゼロ検知時は即座に緊急停止
6. **周波数範囲エラー**: 5kHz超過時は最大値に制限、1Hz未満時は最小値に制限
7. **速度範囲エラー**: 0-100%範囲外の値は最近傍値に制限
8. **シーケンスエラー**: 不正な状態遷移時は安全状態に復帰

## テスト戦略

### 単体テスト
- 各コンポーネントの基本機能テスト
- エラー条件での動作確認
- GPIO操作のモックテスト
- リレー制御の安全性テスト
- 重量安定化ロジックのテスト

### プロパティベーステスト
- QuickCheckライブラリを使用
- 各プロパティを最低100回実行
- ランダムな入力値での動作検証
- モーター制御シーケンスの検証
- 重量安定化処理の検証

**プロパティベーステストの要件**:
- Rustの`quickcheck`クレートを使用
- 各プロパティテストは最低100回の反復実行
- 各テストには設計書のプロパティ番号を明記
- フォーマット: `**Feature: weight-sensor-pump-controller, Property {number}: {property_text}**`

### 統合テスト
- システム全体の状態遷移テスト
- ハードウェアシミュレータを使用した動作確認
- クロスコンパイル後のバイナリテスト
- 完全な制御シーケンスのテスト（安定化→正転→逆転→停止）
- 緊急停止機能のテスト
### プロパティ 8: 重量測定範囲の妥当性
*任意の*重量センサー設定において、0-100gの範囲で正確な測定が実行される
**検証対象: 要件 3.5**