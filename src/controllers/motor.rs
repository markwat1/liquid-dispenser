use crate::errors::MotorError;
use crate::models::MotorState;
use rppal::gpio::{Gpio, OutputPin};
use std::time::{Duration, Instant};
use std::thread;
use log::{info, warn, error, debug};

/// モーターコントローラ
/// 2つのリレーを使用してモーターの正転・逆転・停止を制御
pub struct MotorController {
    relay_a_pin: OutputPin,  // 正転用リレー
    relay_b_pin: OutputPin,  // 逆転用リレー
    current_state: MotorState,
    start_time: Option<Instant>,
    total_runtime: Duration,
}

impl MotorController {
    /// 新しいモーターコントローラを作成
    pub fn new(relay_a_pin_num: u8, relay_b_pin_num: u8) -> Result<Self, MotorError> {
        let gpio = Gpio::new().map_err(|e| MotorError::Gpio(e.to_string()))?;
        
        // リレーA（正転用）の初期化
        let mut relay_a_pin = gpio.get(relay_a_pin_num)
            .map_err(|e| MotorError::Gpio(e.to_string()))?
            .into_output();
        
        // リレーB（逆転用）の初期化
        let mut relay_b_pin = gpio.get(relay_b_pin_num)
            .map_err(|e| MotorError::Gpio(e.to_string()))?
            .into_output();
        
        // 初期状態で両リレーをOFF（安全な停止状態）
        relay_a_pin.set_low();
        relay_b_pin.set_low();
        
        info!("Motor controller initialized: Relay A (GPIO {}), Relay B (GPIO {})", 
              relay_a_pin_num, relay_b_pin_num);
        
        Ok(Self {
            relay_a_pin,
            relay_b_pin,
            current_state: MotorState::Stopped,
            start_time: None,
            total_runtime: Duration::new(0, 0),
        })
    }
    
    /// 正転動作を開始
    pub fn start_forward(&mut self) -> Result<(), MotorError> {
        info!("Starting motor forward rotation");
        
        // 安全性チェック：現在の状態を確認
        if self.current_state != MotorState::Stopped {
            warn!("Motor is not stopped, current state: {:?}", self.current_state);
            self.emergency_stop();
        }
        
        // リレーB（逆転）をOFFにしてからリレーA（正転）をON
        self.relay_b_pin.set_low();
        thread::sleep(Duration::from_millis(10)); // 短時間待機で安全性確保
        
        self.relay_a_pin.set_high();
        thread::sleep(Duration::from_millis(10)); // 確実な動作のための待機
        
        // 状態確認
        if !self.relay_a_pin.is_set_high() || self.relay_b_pin.is_set_high() {
            error!("Failed to set relay states for forward rotation");
            self.emergency_stop();
            return Err(MotorError::StartFailure);
        }
        
        self.current_state = MotorState::Forward;
        self.start_time = Some(Instant::now());
        
        info!("Motor forward rotation started successfully");
        Ok(())
    }
    
    /// 逆転動作を開始
    pub fn start_reverse(&mut self) -> Result<(), MotorError> {
        info!("Starting motor reverse rotation");
        
        // 安全性チェック：現在の状態を確認
        if self.current_state != MotorState::Stopped {
            warn!("Motor is not stopped, current state: {:?}", self.current_state);
            self.emergency_stop();
        }
        
        // リレーA（正転）をOFFにしてからリレーB（逆転）をON
        self.relay_a_pin.set_low();
        thread::sleep(Duration::from_millis(10)); // 短時間待機で安全性確保
        
        self.relay_b_pin.set_high();
        thread::sleep(Duration::from_millis(10)); // 確実な動作のための待機
        
        // 状態確認
        if self.relay_a_pin.is_set_high() || !self.relay_b_pin.is_set_high() {
            error!("Failed to set relay states for reverse rotation");
            self.emergency_stop();
            return Err(MotorError::StartFailure);
        }
        
        self.current_state = MotorState::Reverse;
        self.start_time = Some(Instant::now());
        
        info!("Motor reverse rotation started successfully");
        Ok(())
    }
    
    /// モーターを停止
    pub fn stop(&mut self) -> Result<(), MotorError> {
        info!("Stopping motor");
        
        if self.current_state == MotorState::Stopped {
            debug!("Motor is already stopped");
            return Ok(());
        }
        
        // 両リレーをOFF
        self.relay_a_pin.set_low();
        self.relay_b_pin.set_low();
        
        // 実行時間を記録
        if let Some(start_time) = self.start_time {
            self.total_runtime += start_time.elapsed();
        }
        
        self.current_state = MotorState::Stopped;
        self.start_time = None;
        
        // 短時間待機してモーターが確実に停止されることを確認
        thread::sleep(Duration::from_millis(50));
        
        // 停止状態の確認
        if self.relay_a_pin.is_set_high() || self.relay_b_pin.is_set_high() {
            error!("Failed to stop motor: relays still active");
            return Err(MotorError::StopFailure);
        }
        
        info!("Motor stopped successfully");
        Ok(())
    }
    
    /// 緊急停止（エラーチェックなし）
    pub fn emergency_stop(&mut self) {
        warn!("Emergency stop activated");
        
        // 両リレーを即座にOFF
        self.relay_a_pin.set_low();
        self.relay_b_pin.set_low();
        
        // 実行時間を記録
        if let Some(start_time) = self.start_time {
            self.total_runtime += start_time.elapsed();
        }
        
        self.current_state = MotorState::Stopped;
        self.start_time = None;
        
        info!("Emergency stop completed");
    }
    
    /// 現在のモーター状態を取得
    pub fn current_state(&self) -> &MotorState {
        &self.current_state
    }
    
    /// モーターが動作中かチェック
    pub fn is_running(&self) -> bool {
        self.current_state.is_running()
    }
    
    /// 安全状態かチェック（両リレーが同時ONでないことを確認）
    pub fn is_safe_state(&self) -> bool {
        let relay_a_active = self.relay_a_pin.is_set_high();
        let relay_b_active = self.relay_b_pin.is_set_high();
        
        // 両リレーが同時にONの場合は危険
        !(relay_a_active && relay_b_active)
    }
    
    /// 現在の実行時間を取得
    pub fn current_runtime(&self) -> Duration {
        if let Some(start_time) = self.start_time {
            start_time.elapsed()
        } else {
            Duration::new(0, 0)
        }
    }
    
    /// 累計実行時間を取得
    pub fn total_runtime(&self) -> Duration {
        let current = self.current_runtime();
        self.total_runtime + current
    }
    
    /// 実行時間をリセット
    #[allow(dead_code)]
    pub fn reset_runtime(&mut self) {
        self.total_runtime = Duration::new(0, 0);
        if self.is_running() {
            self.start_time = Some(Instant::now());
        }
    }
    
    /// モーターの健全性チェック
    pub fn health_check(&self) -> Result<(), MotorError> {
        debug!("Performing motor health check");
        
        // 安全状態のチェック
        if !self.is_safe_state() {
            error!("Unsafe motor state detected: both relays are active");
            return Err(MotorError::UnsafeState);
        }
        
        // GPIO状態と内部状態の整合性をチェック
        let relay_a_active = self.relay_a_pin.is_set_high();
        let relay_b_active = self.relay_b_pin.is_set_high();
        
        let expected_state = match (relay_a_active, relay_b_active) {
            (false, false) => MotorState::Stopped,
            (true, false) => MotorState::Forward,
            (false, true) => MotorState::Reverse,
            (true, true) => {
                error!("Both relays are active - unsafe state");
                return Err(MotorError::UnsafeState);
            }
        };
        
        if self.current_state != expected_state {
            error!("Motor state inconsistency: internal={:?}, GPIO={:?}", 
                   self.current_state, expected_state);
            return Err(MotorError::RelayControl(
                "Motor state inconsistent with GPIO state".to_string()
            ));
        }
        
        debug!("Motor health check passed");
        Ok(())
    }
    
    /// 安全停止（タイムアウト付き）
    pub fn safe_stop(&mut self, timeout: Duration) -> Result<(), MotorError> {
        if !self.is_running() {
            return Ok(());
        }
        
        let start = Instant::now();
        
        // 通常の停止を試行
        match self.stop() {
            Ok(()) => return Ok(()),
            Err(_) => {
                warn!("Normal stop failed, attempting emergency stop");
                self.emergency_stop();
                
                // タイムアウトまで停止を確認
                while start.elapsed() < timeout {
                    if !self.relay_a_pin.is_set_high() && !self.relay_b_pin.is_set_high() {
                        return Ok(());
                    }
                    thread::sleep(Duration::from_millis(10));
                }
                
                error!("Failed to stop motor within timeout");
                return Err(MotorError::StopFailure);
            }
        }
    }
    
    /// モーターの状態情報を取得
    #[allow(dead_code)]
    pub fn status(&self) -> MotorStatus {
        MotorStatus {
            state: self.current_state.clone(),
            current_runtime: self.current_runtime(),
            total_runtime: self.total_runtime(),
            relay_a_active: self.relay_a_pin.is_set_high(),
            relay_b_active: self.relay_b_pin.is_set_high(),
            is_safe: self.is_safe_state(),
        }
    }
}

/// モーターの状態情報
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct MotorStatus {
    pub state: MotorState,
    pub current_runtime: Duration,
    pub total_runtime: Duration,
    pub relay_a_active: bool,
    pub relay_b_active: bool,
    pub is_safe: bool,
}

impl MotorStatus {
    /// 状態の文字列表現
    #[allow(dead_code)]
    pub fn as_str(&self) -> &str {
        self.state.as_str()
    }
    
    /// 状態が正常かチェック
    #[allow(dead_code)]
    pub fn is_healthy(&self) -> bool {
        self.is_safe && match self.state {
            MotorState::Stopped => !self.relay_a_active && !self.relay_b_active,
            MotorState::Forward => self.relay_a_active && !self.relay_b_active,
            MotorState::Reverse => !self.relay_a_active && self.relay_b_active,
        }
    }
}

// Drop実装で安全にリソースを解放
impl Drop for MotorController {
    fn drop(&mut self) {
        if self.is_running() {
            warn!("Motor controller dropped while running, performing emergency stop");
            self.emergency_stop();
        }
        info!("Motor controller dropped safely");
    }
}