use std::time::{Duration, Instant};

pub mod config;

#[cfg(test)]
mod tests;

/// モーター状態
#[derive(Debug, Clone, PartialEq)]
pub enum MotorState {
    /// 停止状態（両PWM出力0%）
    Stopped,
    /// 正転状態（速度0.0-1.0）
    Forward(f64),
    /// 逆転状態（速度0.0-1.0）
    Reverse(f64),
}

impl MotorState {
    /// 状態の文字列表現
    pub fn as_str(&self) -> &str {
        match self {
            MotorState::Stopped => "Stopped",
            MotorState::Forward(_) => "Forward",
            MotorState::Reverse(_) => "Reverse",
        }
    }
    
    /// 状態が動作中かどうか
    pub fn is_running(&self) -> bool {
        matches!(self, MotorState::Forward(_) | MotorState::Reverse(_))
    }
    
    /// 現在の速度を取得（0.0-1.0）
    pub fn speed(&self) -> f64 {
        match self {
            MotorState::Stopped => 0.0,
            MotorState::Forward(speed) => *speed,
            MotorState::Reverse(speed) => *speed,
        }
    }
}

/// 重量測定データ
#[derive(Debug, Clone)]
pub struct WeightReading {
    /// 重量値（グラム単位）
    pub value: f32,
    /// 測定時刻
    #[allow(dead_code)]
    pub timestamp: Instant,
    /// ノイズフィルタ後の安定性
    pub is_stable: bool,
}

impl WeightReading {
    /// 新しい重量測定データを作成
    pub fn new(value: f32, is_stable: bool) -> Self {
        Self {
            value,
            timestamp: Instant::now(),
            is_stable,
        }
    }
    
    /// 測定値が有効範囲内かチェック
    pub fn is_valid(&self) -> bool {
        self.value >= 0.0 && self.value <= 100.0
    }
    
    /// 測定値を1g単位に丸める
    #[allow(dead_code)]
    pub fn rounded_value(&self) -> f32 {
        (self.value * 10.0).round() / 10.0
    }
}

/// システム設定
#[derive(Debug, Clone)]
pub struct SystemConfig {
    /// 目標重量（グラム）
    pub target_weight: f32,
    /// HX711 DT端子のGPIOピン番号
    pub dt_pin: u8,
    /// HX711 SCK端子のGPIOピン番号
    pub sck_pin: u8,
    /// PWM正転出力のGPIOピン番号
    pub pwm_forward_pin: u8,
    /// PWM逆転出力のGPIOピン番号
    pub pwm_reverse_pin: u8,
    /// ボタンのGPIOピン番号
    pub button_pin: u8,
    /// 重量センサーの校正係数
    pub calibration_factor: f32,
    /// 移動平均のウィンドウサイズ
    pub moving_average_window: usize,
    /// 重量安定化時間（3秒）
    pub stabilization_duration: Duration,
    /// 逆転動作時間（3秒）
    pub reverse_duration: Duration,
    /// 重量安定判定の許容範囲
    pub weight_tolerance: f32,
    /// PWM周波数（最大5kHz）
    pub pwm_frequency: f64,
    /// 正転時の速度（0.0-1.0）
    pub forward_speed: f64,
    /// 逆転時の速度（0.0-1.0）
    pub reverse_speed: f64,
}

impl Default for SystemConfig {
    fn default() -> Self {
        Self {
            target_weight: 50.0,
            dt_pin: 5,
            sck_pin: 6,
            pwm_forward_pin: 18,
            pwm_reverse_pin: 19,
            button_pin: 2,
            calibration_factor: 1.0,
            moving_average_window: 5,
            stabilization_duration: Duration::from_secs(3),
            reverse_duration: Duration::from_secs(3),
            weight_tolerance: 0.5,
            pwm_frequency: 1000.0,  // 1kHz default
            forward_speed: 0.8,     // 80% speed
            reverse_speed: 0.6,     // 60% speed for reverse
        }
    }
}

impl SystemConfig {
    /// 環境変数から設定を読み込み
    #[allow(dead_code)]
    pub fn from_env() -> Self {
        let mut config = Self::default();
        
        if let Ok(weight) = std::env::var("TARGET_WEIGHT") {
            if let Ok(weight) = weight.parse::<f32>() {
                config.target_weight = weight;
            }
        }
        
        config
    }
    
    /// コンパイル時設定から読み込み
    pub fn from_compiled() -> Self {
        Self {
            target_weight: env!("COMPILED_TARGET_WEIGHT").parse().unwrap_or(50.0),
            dt_pin: env!("COMPILED_DT_PIN").parse().unwrap_or(5),
            sck_pin: env!("COMPILED_SCK_PIN").parse().unwrap_or(6),
            pwm_forward_pin: env!("COMPILED_PWM_FORWARD_PIN").parse().unwrap_or(18),
            pwm_reverse_pin: env!("COMPILED_PWM_REVERSE_PIN").parse().unwrap_or(19),
            button_pin: env!("COMPILED_BUTTON_PIN").parse().unwrap_or(2),
            calibration_factor: env!("COMPILED_CALIBRATION_FACTOR").parse().unwrap_or(1.0),
            moving_average_window: env!("COMPILED_MOVING_AVERAGE_WINDOW").parse().unwrap_or(5),
            stabilization_duration: Duration::from_secs(3),
            reverse_duration: Duration::from_secs(3),
            weight_tolerance: 0.5,
            pwm_frequency: env!("COMPILED_PWM_FREQUENCY").parse().unwrap_or(1000.0),
            forward_speed: env!("COMPILED_FORWARD_SPEED").parse().unwrap_or(0.8),
            reverse_speed: env!("COMPILED_REVERSE_SPEED").parse().unwrap_or(0.6),
        }
    }
    
    /// 環境変数とコンパイル時設定を組み合わせて読み込み
    #[allow(dead_code)]
    pub fn from_env_or_compiled() -> Self {
        let mut config = Self::from_compiled();
        
        // 環境変数があれば上書き
        if let Ok(weight) = std::env::var("TARGET_WEIGHT") {
            if let Ok(weight) = weight.parse::<f32>() {
                config.target_weight = weight;
            }
        }
        
        if let Ok(pin) = std::env::var("DT_PIN") {
            if let Ok(pin) = pin.parse::<u8>() {
                config.dt_pin = pin;
            }
        }
        
        if let Ok(pin) = std::env::var("SCK_PIN") {
            if let Ok(pin) = pin.parse::<u8>() {
                config.sck_pin = pin;
            }
        }
        
        if let Ok(pin) = std::env::var("PWM_FORWARD_PIN") {
            if let Ok(pin) = pin.parse::<u8>() {
                config.pwm_forward_pin = pin;
            }
        }
        
        if let Ok(pin) = std::env::var("PWM_REVERSE_PIN") {
            if let Ok(pin) = pin.parse::<u8>() {
                config.pwm_reverse_pin = pin;
            }
        }
        
        if let Ok(pin) = std::env::var("BUTTON_PIN") {
            if let Ok(pin) = pin.parse::<u8>() {
                config.button_pin = pin;
            }
        }
        
        if let Ok(factor) = std::env::var("CALIBRATION_FACTOR") {
            if let Ok(factor) = factor.parse::<f32>() {
                config.calibration_factor = factor;
            }
        }
        
        if let Ok(window) = std::env::var("MOVING_AVERAGE_WINDOW") {
            if let Ok(window) = window.parse::<usize>() {
                config.moving_average_window = window;
            }
        }
        
        if let Ok(freq) = std::env::var("PWM_FREQUENCY") {
            if let Ok(freq) = freq.parse::<f64>() {
                config.pwm_frequency = freq;
            }
        }
        
        if let Ok(speed) = std::env::var("FORWARD_SPEED") {
            if let Ok(speed) = speed.parse::<f64>() {
                config.forward_speed = speed;
            }
        }
        
        if let Ok(speed) = std::env::var("REVERSE_SPEED") {
            if let Ok(speed) = speed.parse::<f64>() {
                config.reverse_speed = speed;
            }
        }
        
        config
    }
    
    /// ビルド情報を取得
    pub fn build_info() -> BuildInfo {
        BuildInfo {
            timestamp: env!("BUILD_TIMESTAMP").to_string(),
            target: env!("BUILD_TARGET").to_string(),
            version: env!("CARGO_PKG_VERSION").to_string(),
            compiled_target_weight: env!("COMPILED_TARGET_WEIGHT").parse().unwrap_or(0.0),
        }
    }
    
    /// 設定値の妥当性をチェック
    pub fn validate(&self) -> Result<(), String> {
        if self.target_weight <= 0.0 || self.target_weight > 100.0 {
            return Err(format!("Invalid target weight: {}g", self.target_weight));
        }
        
        if self.moving_average_window == 0 {
            return Err("Moving average window must be greater than 0".to_string());
        }
        
        // GPIO番号の重複チェック
        let pins = vec![self.dt_pin, self.sck_pin, self.pwm_forward_pin, self.pwm_reverse_pin, self.button_pin];
        let mut unique_pins = pins.clone();
        unique_pins.sort();
        unique_pins.dedup();
        
        if pins.len() != unique_pins.len() {
            return Err("GPIO pin numbers must be unique".to_string());
        }
        
        // PWM周波数の範囲チェック
        if self.pwm_frequency < 1.0 || self.pwm_frequency > 5000.0 {
            return Err(format!("PWM frequency must be between 1Hz and 5kHz, got: {}Hz", self.pwm_frequency));
        }
        
        // 速度の範囲チェック
        if self.forward_speed < 0.0 || self.forward_speed > 1.0 {
            return Err(format!("Forward speed must be between 0.0 and 1.0, got: {}", self.forward_speed));
        }
        
        if self.reverse_speed < 0.0 || self.reverse_speed > 1.0 {
            return Err(format!("Reverse speed must be between 0.0 and 1.0, got: {}", self.reverse_speed));
        }
        
        Ok(())
    }
}

/// システム状態
#[derive(Debug, Clone, PartialEq)]
pub enum SystemState {
    /// 待機状態
    Idle,
    /// 重量安定化中（3秒間）
    WeightStabilizing,
    /// 正転動作中
    Forward,
    /// 目標重量到達、正転停止
    TargetReached,
    /// 逆転動作中（3秒間）
    Reverse,
    /// 処理完了
    Completed,
    /// 停止処理中
    Stopping,
    /// エラー状態
    Error(String),
}

impl SystemState {
    /// 状態が動作中かどうか
    pub fn is_active(&self) -> bool {
        matches!(self, 
            SystemState::WeightStabilizing | 
            SystemState::Forward | 
            SystemState::TargetReached |
            SystemState::Reverse | 
            SystemState::Stopping
        )
    }
    
    /// 状態がエラーかどうか
    #[allow(dead_code)]
    pub fn is_error(&self) -> bool {
        matches!(self, SystemState::Error(_))
    }
    
    /// 状態の文字列表現
    #[allow(dead_code)]
    pub fn as_str(&self) -> &str {
        match self {
            SystemState::Idle => "Idle",
            SystemState::WeightStabilizing => "WeightStabilizing",
            SystemState::Forward => "Forward",
            SystemState::TargetReached => "TargetReached",
            SystemState::Reverse => "Reverse",
            SystemState::Completed => "Completed",
            SystemState::Stopping => "Stopping",
            SystemState::Error(_) => "Error",
        }
    }
}

/// ビルド情報
#[derive(Debug, Clone)]
pub struct BuildInfo {
    pub timestamp: String,
    pub target: String,
    pub version: String,
    pub compiled_target_weight: f32,
}

impl BuildInfo {
    /// ビルド情報の表示
    pub fn display(&self) {
        println!("=== Build Information ===");
        println!("Version: {}", self.version);
        println!("Build Target: {}", self.target);
        println!("Build Timestamp: {}", self.timestamp);
        println!("Compiled Target Weight: {}g", self.compiled_target_weight);
        println!("========================");
    }
}

/// 移動平均フィルタ
#[derive(Debug, Clone)]
pub struct MovingAverage {
    window_size: usize,
    values: Vec<f32>,
    sum: f32,
}

impl MovingAverage {
    /// 新しい移動平均フィルタを作成
    pub fn new(window_size: usize) -> Self {
        Self {
            window_size: window_size.max(1),
            values: Vec::new(),
            sum: 0.0,
        }
    }
    
    /// 新しい値を追加して平均を計算
    pub fn add(&mut self, value: f32) -> f32 {
        self.values.push(value);
        self.sum += value;
        
        if self.values.len() > self.window_size {
            let removed = self.values.remove(0);
            self.sum -= removed;
        }
        
        self.sum / self.values.len() as f32
    }
    
    /// 現在の平均値を取得
    pub fn average(&self) -> Option<f32> {
        if self.values.is_empty() {
            None
        } else {
            Some(self.sum / self.values.len() as f32)
        }
    }
    
    /// フィルタをリセット
    pub fn reset(&mut self) {
        self.values.clear();
        self.sum = 0.0;
    }
    
    /// 値が安定しているかチェック（分散が小さいか）
    pub fn is_stable(&self, threshold: f32) -> bool {
        if self.values.len() < self.window_size {
            return false;
        }
        
        let avg = self.average().unwrap_or(0.0);
        let variance = self.values.iter()
            .map(|&x| (x - avg).powi(2))
            .sum::<f32>() / self.values.len() as f32;
        
        variance < threshold
    }
}