use crate::errors::MotorError;
use crate::models::MotorState;
use rppal::pwm::{Pwm, Channel, Polarity};
use std::time::{Duration, Instant};
use std::thread;
use log::{info, warn, error, debug};

/// PWMモーターコントローラ
/// 2つのPWM出力を使用してモーターの正転・逆転・速度制御を実行
pub struct MotorController {
    pwm_forward: Pwm,       // 正転用PWM出力
    pwm_reverse: Pwm,       // 逆転用PWM出力
    current_state: MotorState,
    current_speed: f64,     // 現在の速度（0.0-1.0）
    pwm_frequency: f64,     // PWM周波数（Hz）
    start_time: Option<Instant>,
    total_runtime: Duration,
}

impl MotorController {
    /// 新しいPWMモーターコントローラを作成
    pub fn new(forward_pin: u8, reverse_pin: u8, frequency: f64) -> Result<Self, MotorError> {
        // 周波数の範囲チェック
        if frequency < 1.0 || frequency > 5000.0 {
            return Err(MotorError::FrequencyOutOfRange(frequency));
        }
        
        // PWMチャンネルの決定（GPIO 18 = PWM0, GPIO 19 = PWM1）
        let forward_channel = match forward_pin {
            18 => Channel::Pwm0,
            19 => Channel::Pwm1,
            _ => return Err(MotorError::Gpio(format!("GPIO {} does not support PWM", forward_pin))),
        };
        
        let reverse_channel = match reverse_pin {
            18 => Channel::Pwm0,
            19 => Channel::Pwm1,
            _ => return Err(MotorError::Gpio(format!("GPIO {} does not support PWM", reverse_pin))),
        };
        
        // 同じチャンネルを使用していないかチェック
        if forward_channel == reverse_channel {
            return Err(MotorError::Gpio("Forward and reverse pins cannot use the same PWM channel".to_string()));
        }
        
        // PWM初期化
        let pwm_forward = Pwm::with_frequency(forward_channel, frequency, 0.0, Polarity::Normal, true)
            .map_err(|e| MotorError::PwmControl(e.to_string()))?;
        
        let pwm_reverse = Pwm::with_frequency(reverse_channel, frequency, 0.0, Polarity::Normal, true)
            .map_err(|e| MotorError::PwmControl(e.to_string()))?;
        
        // 初期状態で両PWM出力を0%（安全な停止状態）
        pwm_forward.set_duty_cycle(0.0)
            .map_err(|e| MotorError::PwmControl(e.to_string()))?;
        pwm_reverse.set_duty_cycle(0.0)
            .map_err(|e| MotorError::PwmControl(e.to_string()))?;
        
        info!("PWM Motor controller initialized: Forward (GPIO {}), Reverse (GPIO {}), Frequency: {}Hz", 
              forward_pin, reverse_pin, frequency);
        
        Ok(Self {
            pwm_forward,
            pwm_reverse,
            current_state: MotorState::Stopped,
            current_speed: 0.0,
            pwm_frequency: frequency,
            start_time: None,
            total_runtime: Duration::new(0, 0),
        })
    }
    
    /// 正転動作を開始
    pub fn start_forward(&mut self, speed: f64) -> Result<(), MotorError> {
        // 速度の範囲チェック
        if speed < 0.0 || speed > 1.0 {
            return Err(MotorError::InvalidSpeed(speed));
        }
        
        info!("Starting motor forward rotation at speed: {:.1}%", speed * 100.0);
        
        // 安全性チェック：現在の状態を確認
        if self.current_state != MotorState::Stopped {
            warn!("Motor is not stopped, current state: {:?}", self.current_state);
            self.emergency_stop();
        }
        
        // 逆転PWMを0%にしてから正転PWMを設定
        self.pwm_reverse.set_duty_cycle(0.0)
            .map_err(|e| MotorError::PwmControl(e.to_string()))?;
        thread::sleep(Duration::from_millis(10)); // 短時間待機で安全性確保
        
        if speed > 0.0 {
            self.pwm_forward.set_duty_cycle(speed)
                .map_err(|e| MotorError::PwmControl(e.to_string()))?;
        }
        thread::sleep(Duration::from_millis(10)); // 確実な動作のための待機
        
        // 状態確認
        let forward_duty = self.pwm_forward.duty_cycle()
            .map_err(|e| MotorError::PwmControl(e.to_string()))?;
        let reverse_duty = self.pwm_reverse.duty_cycle()
            .map_err(|e| MotorError::PwmControl(e.to_string()))?;
        
        if (speed > 0.0 && forward_duty < speed * 0.9) || reverse_duty > 0.01 {
            error!("Failed to set PWM states for forward rotation: forward={:.3}, reverse={:.3}", 
                   forward_duty, reverse_duty);
            self.emergency_stop();
            return Err(MotorError::StartFailure);
        }
        
        self.current_state = MotorState::Forward(speed);
        self.current_speed = speed;
        self.start_time = Some(Instant::now());
        
        info!("Motor forward rotation started successfully at {:.1}%", speed * 100.0);
        Ok(())
    }
    
    /// 逆転動作を開始
    pub fn start_reverse(&mut self, speed: f64) -> Result<(), MotorError> {
        // 速度の範囲チェック
        if speed < 0.0 || speed > 1.0 {
            return Err(MotorError::InvalidSpeed(speed));
        }
        
        info!("Starting motor reverse rotation at speed: {:.1}%", speed * 100.0);
        
        // 安全性チェック：現在の状態を確認
        if self.current_state != MotorState::Stopped {
            warn!("Motor is not stopped, current state: {:?}", self.current_state);
            self.emergency_stop();
        }
        
        // 正転PWMを0%にしてから逆転PWMを設定
        self.pwm_forward.set_duty_cycle(0.0)
            .map_err(|e| MotorError::PwmControl(e.to_string()))?;
        thread::sleep(Duration::from_millis(10)); // 短時間待機で安全性確保
        
        if speed > 0.0 {
            self.pwm_reverse.set_duty_cycle(speed)
                .map_err(|e| MotorError::PwmControl(e.to_string()))?;
        }
        thread::sleep(Duration::from_millis(10)); // 確実な動作のための待機
        
        // 状態確認
        let forward_duty = self.pwm_forward.duty_cycle()
            .map_err(|e| MotorError::PwmControl(e.to_string()))?;
        let reverse_duty = self.pwm_reverse.duty_cycle()
            .map_err(|e| MotorError::PwmControl(e.to_string()))?;
        
        if (speed > 0.0 && reverse_duty < speed * 0.9) || forward_duty > 0.01 {
            error!("Failed to set PWM states for reverse rotation: forward={:.3}, reverse={:.3}", 
                   forward_duty, reverse_duty);
            self.emergency_stop();
            return Err(MotorError::StartFailure);
        }
        
        self.current_state = MotorState::Reverse(speed);
        self.current_speed = speed;
        self.start_time = Some(Instant::now());
        
        info!("Motor reverse rotation started successfully at {:.1}%", speed * 100.0);
        Ok(())
    }
    
    /// 速度を変更（動作中のみ）
    pub fn set_speed(&mut self, speed: f64) -> Result<(), MotorError> {
        // 速度の範囲チェック
        if speed < 0.0 || speed > 1.0 {
            return Err(MotorError::InvalidSpeed(speed));
        }
        
        match &self.current_state {
            MotorState::Forward(_) => {
                self.pwm_forward.set_duty_cycle(speed)
                    .map_err(|e| MotorError::PwmControl(e.to_string()))?;
                self.current_state = MotorState::Forward(speed);
                self.current_speed = speed;
                info!("Forward speed changed to {:.1}%", speed * 100.0);
            },
            MotorState::Reverse(_) => {
                self.pwm_reverse.set_duty_cycle(speed)
                    .map_err(|e| MotorError::PwmControl(e.to_string()))?;
                self.current_state = MotorState::Reverse(speed);
                self.current_speed = speed;
                info!("Reverse speed changed to {:.1}%", speed * 100.0);
            },
            MotorState::Stopped => {
                return Err(MotorError::NotRunning);
            }
        }
        
        Ok(())
    }
    
    /// モーターを停止
    pub fn stop(&mut self) -> Result<(), MotorError> {
        info!("Stopping motor");
        
        if self.current_state == MotorState::Stopped {
            debug!("Motor is already stopped");
            return Ok(());
        }
        
        // 両PWM出力を0%
        self.pwm_forward.set_duty_cycle(0.0)
            .map_err(|e| MotorError::PwmControl(e.to_string()))?;
        self.pwm_reverse.set_duty_cycle(0.0)
            .map_err(|e| MotorError::PwmControl(e.to_string()))?;
        
        // 実行時間を記録
        if let Some(start_time) = self.start_time {
            self.total_runtime += start_time.elapsed();
        }
        
        self.current_state = MotorState::Stopped;
        self.current_speed = 0.0;
        self.start_time = None;
        
        // 短時間待機してモーターが確実に停止されることを確認
        thread::sleep(Duration::from_millis(50));
        
        // 停止状態の確認
        let forward_duty = self.pwm_forward.duty_cycle()
            .map_err(|e| MotorError::PwmControl(e.to_string()))?;
        let reverse_duty = self.pwm_reverse.duty_cycle()
            .map_err(|e| MotorError::PwmControl(e.to_string()))?;
        
        if forward_duty > 0.01 || reverse_duty > 0.01 {
            error!("Failed to stop motor: PWM still active (forward={:.3}, reverse={:.3})", 
                   forward_duty, reverse_duty);
            return Err(MotorError::StopFailure);
        }
        
        info!("Motor stopped successfully");
        Ok(())
    }
    
    /// 緊急停止（エラーチェックなし）
    pub fn emergency_stop(&mut self) {
        warn!("Emergency stop activated");
        
        // 両PWM出力を即座に0%
        let _ = self.pwm_forward.set_duty_cycle(0.0);
        let _ = self.pwm_reverse.set_duty_cycle(0.0);
        
        // 実行時間を記録
        if let Some(start_time) = self.start_time {
            self.total_runtime += start_time.elapsed();
        }
        
        self.current_state = MotorState::Stopped;
        self.current_speed = 0.0;
        self.start_time = None;
        
        info!("Emergency stop completed");
    }
    
    /// 現在のモーター状態を取得
    pub fn current_state(&self) -> &MotorState {
        &self.current_state
    }
    
    /// 現在の速度を取得
    pub fn current_speed(&self) -> f64 {
        self.current_speed
    }
    
    /// PWM周波数を設定
    pub fn set_frequency(&mut self, frequency: f64) -> Result<(), MotorError> {
        if frequency < 1.0 || frequency > 5000.0 {
            return Err(MotorError::FrequencyOutOfRange(frequency));
        }
        
        // 現在の状態を保存
        let was_running = self.is_running();
        let current_speed = self.current_speed;
        let current_state = self.current_state.clone();
        
        // 一時停止
        if was_running {
            self.stop()?;
        }
        
        // 周波数を変更
        self.pwm_forward.set_frequency(frequency, 0.0)
            .map_err(|e| MotorError::PwmControl(e.to_string()))?;
        self.pwm_reverse.set_frequency(frequency, 0.0)
            .map_err(|e| MotorError::PwmControl(e.to_string()))?;
        
        self.pwm_frequency = frequency;
        
        // 元の状態に復帰
        if was_running {
            match current_state {
                MotorState::Forward(_) => self.start_forward(current_speed)?,
                MotorState::Reverse(_) => self.start_reverse(current_speed)?,
                MotorState::Stopped => {},
            }
        }
        
        info!("PWM frequency changed to {}Hz", frequency);
        Ok(())
    }
    
    /// モーターが動作中かチェック
    pub fn is_running(&self) -> bool {
        self.current_state.is_running()
    }
    
    /// 安全状態かチェック（両PWM信号が同時に0%以外でないことを確認）
    pub fn is_safe_state(&self) -> bool {
        match (self.pwm_forward.duty_cycle(), self.pwm_reverse.duty_cycle()) {
            (Ok(forward_duty), Ok(reverse_duty)) => {
                // 両PWM信号が同時に0%以外の場合は危険
                !(forward_duty > 0.01 && reverse_duty > 0.01)
            },
            _ => false, // PWM読み取りエラーの場合は安全でないと判定
        }
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
            error!("Unsafe motor state detected: both PWM signals are active");
            return Err(MotorError::UnsafeState);
        }
        
        // PWM状態と内部状態の整合性をチェック
        let forward_duty = self.pwm_forward.duty_cycle()
            .map_err(|e| MotorError::PwmControl(e.to_string()))?;
        let reverse_duty = self.pwm_reverse.duty_cycle()
            .map_err(|e| MotorError::PwmControl(e.to_string()))?;
        
        let expected_state = match (forward_duty > 0.01, reverse_duty > 0.01) {
            (false, false) => MotorState::Stopped,
            (true, false) => MotorState::Forward(forward_duty),
            (false, true) => MotorState::Reverse(reverse_duty),
            (true, true) => {
                error!("Both PWM signals are active - unsafe state");
                return Err(MotorError::UnsafeState);
            }
        };
        
        // 状態の整合性チェック（速度の微小な差は許容）
        match (&self.current_state, &expected_state) {
            (MotorState::Stopped, MotorState::Stopped) => {},
            (MotorState::Forward(internal_speed), MotorState::Forward(pwm_speed)) => {
                if (internal_speed - pwm_speed).abs() > 0.1 {
                    error!("Motor speed inconsistency: internal={:.3}, PWM={:.3}", 
                           internal_speed, pwm_speed);
                    return Err(MotorError::PwmControl(
                        "Motor speed inconsistent with PWM state".to_string()
                    ));
                }
            },
            (MotorState::Reverse(internal_speed), MotorState::Reverse(pwm_speed)) => {
                if (internal_speed - pwm_speed).abs() > 0.1 {
                    error!("Motor speed inconsistency: internal={:.3}, PWM={:.3}", 
                           internal_speed, pwm_speed);
                    return Err(MotorError::PwmControl(
                        "Motor speed inconsistent with PWM state".to_string()
                    ));
                }
            },
            _ => {
                error!("Motor state inconsistency: internal={:?}, PWM={:?}", 
                       self.current_state, expected_state);
                return Err(MotorError::PwmControl(
                    "Motor state inconsistent with PWM state".to_string()
                ));
            }
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
                    if let (Ok(forward_duty), Ok(reverse_duty)) = 
                        (self.pwm_forward.duty_cycle(), self.pwm_reverse.duty_cycle()) {
                        if forward_duty <= 0.01 && reverse_duty <= 0.01 {
                            return Ok(());
                        }
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
        let forward_duty = self.pwm_forward.duty_cycle().unwrap_or(0.0);
        let reverse_duty = self.pwm_reverse.duty_cycle().unwrap_or(0.0);
        
        MotorStatus {
            state: self.current_state.clone(),
            current_runtime: self.current_runtime(),
            total_runtime: self.total_runtime(),
            forward_duty_cycle: forward_duty,
            reverse_duty_cycle: reverse_duty,
            pwm_frequency: self.pwm_frequency,
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
    pub forward_duty_cycle: f64,
    pub reverse_duty_cycle: f64,
    pub pwm_frequency: f64,
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
        self.is_safe && match &self.state {
            MotorState::Stopped => self.forward_duty_cycle <= 0.01 && self.reverse_duty_cycle <= 0.01,
            MotorState::Forward(speed) => {
                self.forward_duty_cycle > 0.01 && 
                self.reverse_duty_cycle <= 0.01 &&
                (self.forward_duty_cycle - speed).abs() < 0.1
            },
            MotorState::Reverse(speed) => {
                self.forward_duty_cycle <= 0.01 && 
                self.reverse_duty_cycle > 0.01 &&
                (self.reverse_duty_cycle - speed).abs() < 0.1
            },
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
        info!("PWM Motor controller dropped safely");
    }
}