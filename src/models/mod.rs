use std::time::Instant;

pub mod config;

#[cfg(test)]
mod tests;

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
    /// ポンプ制御のGPIOピン番号
    pub pump_pin: u8,
    /// ボタンのGPIOピン番号
    pub button_pin: u8,
    /// 重量センサーの校正係数
    pub calibration_factor: f32,
    /// 移動平均のウィンドウサイズ
    pub moving_average_window: usize,
}

impl Default for SystemConfig {
    fn default() -> Self {
        Self {
            target_weight: 50.0,
            dt_pin: 5,
            sck_pin: 6,
            pump_pin: 18,
            button_pin: 2,
            calibration_factor: 1.0,
            moving_average_window: 5,
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
            pump_pin: env!("COMPILED_PUMP_PIN").parse().unwrap_or(18),
            button_pin: env!("COMPILED_BUTTON_PIN").parse().unwrap_or(2),
            calibration_factor: env!("COMPILED_CALIBRATION_FACTOR").parse().unwrap_or(1.0),
            moving_average_window: env!("COMPILED_MOVING_AVERAGE_WINDOW").parse().unwrap_or(5),
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
        
        if let Ok(pin) = std::env::var("PUMP_PIN") {
            if let Ok(pin) = pin.parse::<u8>() {
                config.pump_pin = pin;
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
        let pins = vec![self.dt_pin, self.sck_pin, self.pump_pin, self.button_pin];
        let mut unique_pins = pins.clone();
        unique_pins.sort();
        unique_pins.dedup();
        
        if pins.len() != unique_pins.len() {
            return Err("GPIO pin numbers must be unique".to_string());
        }
        
        Ok(())
    }
}

/// システム状態
#[derive(Debug, Clone, PartialEq)]
pub enum SystemState {
    /// 待機状態
    Idle,
    /// 重量測定中
    #[allow(dead_code)]
    Measuring,
    /// ポンプ動作中
    Pumping,
    /// 停止処理中
    Stopping,
    /// エラー状態
    #[allow(dead_code)]
    Error(String),
}

impl SystemState {
    /// 状態が動作中かどうか
    #[allow(dead_code)]
    pub fn is_active(&self) -> bool {
        matches!(self, SystemState::Measuring | SystemState::Pumping | SystemState::Stopping)
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
            SystemState::Measuring => "Measuring",
            SystemState::Pumping => "Pumping",
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