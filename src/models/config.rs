use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;

/// 設定ファイル形式
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConfigFile {
    pub target_weight: Option<f32>,
    pub gpio: Option<GpioConfig>,
    pub sensor: Option<SensorConfig>,
    pub pump: Option<PumpConfig>,
    pub system: Option<SystemSettings>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GpioConfig {
    pub dt_pin: Option<u8>,
    pub sck_pin: Option<u8>,
    pub pump_pin: Option<u8>,
    pub button_pin: Option<u8>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SensorConfig {
    pub calibration_factor: Option<f32>,
    pub moving_average_window: Option<usize>,
    pub max_weight: Option<f32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PumpConfig {
    pub max_runtime: Option<u64>, // 秒
    pub safety_timeout: Option<u64>, // 秒
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemSettings {
    pub log_level: Option<String>,
    pub stats_interval: Option<u64>, // 秒
    pub health_check_interval: Option<u64>, // 秒
}

impl ConfigFile {
    /// 設定ファイルから読み込み
    pub fn from_file<P: AsRef<Path>>(path: P) -> Result<Self, Box<dyn std::error::Error>> {
        let content = fs::read_to_string(path)?;
        let config: ConfigFile = toml::from_str(&content)?;
        Ok(config)
    }
    
    /// 設定ファイルに保存
    #[allow(dead_code)]
    pub fn to_file<P: AsRef<Path>>(&self, path: P) -> Result<(), Box<dyn std::error::Error>> {
        let content = toml::to_string_pretty(self)?;
        fs::write(path, content)?;
        Ok(())
    }
    
    /// デフォルト設定ファイルを作成
    #[allow(dead_code)]
    pub fn create_default() -> Self {
        Self {
            target_weight: Some(50.0),
            gpio: Some(GpioConfig {
                dt_pin: Some(5),
                sck_pin: Some(6),
                pump_pin: Some(18),
                button_pin: Some(2),
            }),
            sensor: Some(SensorConfig {
                calibration_factor: Some(1.0),
                moving_average_window: Some(5),
                max_weight: Some(100.0),
            }),
            pump: Some(PumpConfig {
                max_runtime: Some(300), // 5分
                safety_timeout: Some(5),
            }),
            system: Some(SystemSettings {
                log_level: Some("info".to_string()),
                stats_interval: Some(60),
                health_check_interval: Some(30),
            }),
        }
    }
}

use crate::models::SystemConfig;

impl SystemConfig {
    /// 設定ファイルから読み込み
    pub fn from_config_file<P: AsRef<Path>>(path: P) -> Result<Self, Box<dyn std::error::Error>> {
        let config_file = ConfigFile::from_file(path)?;
        Ok(Self::from_config_file_struct(config_file))
    }
    
    /// ConfigFile構造体からSystemConfigを作成
    pub fn from_config_file_struct(config_file: ConfigFile) -> Self {
        let mut config = Self::default();
        
        if let Some(weight) = config_file.target_weight {
            config.target_weight = weight;
        }
        
        if let Some(gpio) = config_file.gpio {
            if let Some(pin) = gpio.dt_pin {
                config.dt_pin = pin;
            }
            if let Some(pin) = gpio.sck_pin {
                config.sck_pin = pin;
            }
            if let Some(pin) = gpio.pump_pin {
                config.pump_pin = pin;
            }
            if let Some(pin) = gpio.button_pin {
                config.button_pin = pin;
            }
        }
        
        if let Some(sensor) = config_file.sensor {
            if let Some(factor) = sensor.calibration_factor {
                config.calibration_factor = factor;
            }
            if let Some(window) = sensor.moving_average_window {
                config.moving_average_window = window;
            }
        }
        
        config
    }
    
    /// 設定の優先順位付き読み込み
    /// 1. コマンドライン引数
    /// 2. 環境変数
    /// 3. 設定ファイル
    /// 4. コンパイル時設定
    /// 5. デフォルト値
    pub fn load_with_priority() -> Self {
        let mut config = Self::from_compiled();
        
        // 設定ファイルから読み込み（存在する場合）
        if let Ok(file_config) = Self::from_config_file("config.toml") {
            config = file_config;
        }
        
        // 環境変数で上書き
        config = Self::merge_env_vars(config);
        
        config
    }
    
    /// 環境変数で設定を上書き
    fn merge_env_vars(mut config: Self) -> Self {
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
}