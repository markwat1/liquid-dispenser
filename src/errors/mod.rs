use thiserror::Error;

/// システム全体のエラー型
#[derive(Debug, Error)]
pub enum SystemError {
    #[error("Sensor error: {0}")]
    Sensor(#[from] SensorError),
    
    #[error("Pump error: {0}")]
    Pump(#[from] PumpError),
    
    #[error("Motor error: {0}")]
    Motor(#[from] MotorError),
    
    #[error("Button error: {0}")]
    Button(#[from] ButtonError),
    
    #[error("GPIO error: {0}")]
    Gpio(String),
    
    #[error("Configuration error: {0}")]
    Config(String),
    
    #[error("Relay safety error: {0}")]
    RelaySafety(String),
    
    #[error("Sequence error: {0}")]
    Sequence(String),
}

/// 重量センサー関連のエラー
#[derive(Debug, Error)]
pub enum SensorError {
    #[error("Failed to read sensor data")]
    ReadFailure,
    
    #[error("Sensor communication timeout")]
    Timeout,
    
    #[error("Invalid sensor reading: {0}")]
    #[allow(dead_code)]
    InvalidReading(f32),
    
    #[error("Sensor calibration failed")]
    #[allow(dead_code)]
    CalibrationFailed,
    
    #[error("Sensor out of range: {value}g (max: {max}g)")]
    OutOfRange { value: f32, max: f32 },
    
    #[error("GPIO error: {0}")]
    Gpio(String),
}

/// ポンプ制御関連のエラー
#[derive(Debug, Error)]
pub enum PumpError {
    #[error("Failed to start pump")]
    StartFailure,
    
    #[error("Failed to stop pump")]
    StopFailure,
    
    #[error("Pump is already running")]
    AlreadyRunning,
    
    #[error("Pump is not running")]
    NotRunning,
    
    #[error("GPIO error: {0}")]
    Gpio(String),
}

/// モーター制御関連のエラー
#[derive(Debug, Error)]
pub enum MotorError {
    #[error("Relay control error: {0}")]
    RelayControl(String),
    
    #[error("Unsafe relay state: both relays active")]
    UnsafeState,
    
    #[error("Motor start failure")]
    StartFailure,
    
    #[error("Motor stop failure")]
    StopFailure,
    
    #[error("Motor is already running")]
    AlreadyRunning,
    
    #[error("Motor is not running")]
    NotRunning,
    
    #[error("GPIO error: {0}")]
    Gpio(String),
}

/// ボタン制御関連のエラー
#[derive(Debug, Error)]
pub enum ButtonError {
    #[error("Failed to read button state")]
    #[allow(dead_code)]
    ReadFailure,
    
    #[error("Button debounce timeout")]
    #[allow(dead_code)]
    DebounceTimeout,
    
    #[error("GPIO error: {0}")]
    Gpio(String),
}

/// エラーの重要度レベル
#[derive(Debug, Clone, PartialEq)]
pub enum ErrorLevel {
    Warning,
    Error,
    Critical,
}

impl SystemError {
    /// エラーの重要度を取得
    pub fn level(&self) -> ErrorLevel {
        match self {
            SystemError::Sensor(SensorError::OutOfRange { .. }) => ErrorLevel::Warning,
            SystemError::Sensor(SensorError::InvalidReading(_)) => ErrorLevel::Warning,
            SystemError::Sensor(_) => ErrorLevel::Error,
            SystemError::Pump(_) => ErrorLevel::Critical,
            SystemError::Motor(MotorError::UnsafeState) => ErrorLevel::Critical,
            SystemError::Motor(_) => ErrorLevel::Error,
            SystemError::Button(_) => ErrorLevel::Warning,
            SystemError::Gpio(_) => ErrorLevel::Critical,
            SystemError::Config(_) => ErrorLevel::Error,
            SystemError::RelaySafety(_) => ErrorLevel::Critical,
            SystemError::Sequence(_) => ErrorLevel::Error,
        }
    }
    
    /// エラーが再試行可能かどうか
    #[allow(dead_code)]
    pub fn is_retryable(&self) -> bool {
        match self {
            SystemError::Sensor(SensorError::ReadFailure) => true,
            SystemError::Sensor(SensorError::Timeout) => true,
            SystemError::Motor(MotorError::StartFailure) => true,
            SystemError::Motor(MotorError::StopFailure) => true,
            SystemError::Button(ButtonError::ReadFailure) => true,
            _ => false,
        }
    }
}

#[cfg(test)]
mod tests;