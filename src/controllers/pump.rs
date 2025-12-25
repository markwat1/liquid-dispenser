use crate::errors::PumpError;
use rppal::gpio::{Gpio, OutputPin};
use std::time::{Duration, Instant};
use std::thread;

/// 蠕動ポンプコントローラ
#[allow(dead_code)]
pub struct PumpController {
    control_pin: OutputPin,
    is_running: bool,
    start_time: Option<Instant>,
    total_runtime: Duration,
}

#[allow(dead_code)]
impl PumpController {
    /// 新しいポンプコントローラを作成
    pub fn new(control_pin_num: u8) -> Result<Self, PumpError> {
        let gpio = Gpio::new().map_err(|e| PumpError::Gpio(e.to_string()))?;
        
        let mut control_pin = gpio.get(control_pin_num)
            .map_err(|e| PumpError::Gpio(e.to_string()))?
            .into_output();
        
        // 初期状態でポンプを停止
        control_pin.set_low();
        
        Ok(Self {
            control_pin,
            is_running: false,
            start_time: None,
            total_runtime: Duration::new(0, 0),
        })
    }
    
    /// ポンプを開始
    pub fn start(&mut self) -> Result<(), PumpError> {
        if self.is_running {
            return Err(PumpError::AlreadyRunning);
        }
        
        // GPIO出力をHIGHに設定してポンプを開始
        self.control_pin.set_high();
        
        self.is_running = true;
        self.start_time = Some(Instant::now());
        
        // 短時間待機してポンプが確実に開始されることを確認
        thread::sleep(Duration::from_millis(10));
        
        // ポンプが実際に開始されたかチェック
        if !self.control_pin.is_set_high() {
            self.is_running = false;
            self.start_time = None;
            return Err(PumpError::StartFailure);
        }
        
        Ok(())
    }
    
    /// ポンプを停止
    pub fn stop(&mut self) -> Result<(), PumpError> {
        if !self.is_running {
            return Err(PumpError::NotRunning);
        }
        
        // GPIO出力をLOWに設定してポンプを停止
        self.control_pin.set_low();
        
        // 実行時間を記録
        if let Some(start_time) = self.start_time {
            self.total_runtime += start_time.elapsed();
        }
        
        self.is_running = false;
        self.start_time = None;
        
        // 短時間待機してポンプが確実に停止されることを確認
        thread::sleep(Duration::from_millis(10));
        
        // ポンプが実際に停止されたかチェック
        if self.control_pin.is_set_high() {
            return Err(PumpError::StopFailure);
        }
        
        Ok(())
    }
    
    /// ポンプが動作中かチェック
    pub fn is_running(&self) -> bool {
        self.is_running
    }
    
    /// 緊急停止（エラーチェックなし）
    pub fn emergency_stop(&mut self) {
        self.control_pin.set_low();
        
        // 実行時間を記録
        if let Some(start_time) = self.start_time {
            self.total_runtime += start_time.elapsed();
        }
        
        self.is_running = false;
        self.start_time = None;
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
        if self.is_running {
            self.start_time = Some(Instant::now());
        }
    }
    
    /// ポンプの状態を取得
    #[allow(dead_code)]
    pub fn status(&self) -> PumpStatus {
        PumpStatus {
            is_running: self.is_running,
            current_runtime: self.current_runtime(),
            total_runtime: self.total_runtime(),
            gpio_state: self.control_pin.is_set_high(),
        }
    }
    
    /// ポンプの健全性チェック
    pub fn health_check(&self) -> Result<(), PumpError> {
        // GPIO状態と内部状態の整合性をチェック
        let gpio_high = self.control_pin.is_set_high();
        
        if self.is_running && !gpio_high {
            return Err(PumpError::Gpio("GPIO state inconsistent with running state".to_string()));
        }
        
        if !self.is_running && gpio_high {
            return Err(PumpError::Gpio("GPIO state inconsistent with stopped state".to_string()));
        }
        
        Ok(())
    }
    
    /// 安全停止（タイムアウト付き）
    pub fn safe_stop(&mut self, timeout: Duration) -> Result<(), PumpError> {
        if !self.is_running {
            return Ok(());
        }
        
        let start = Instant::now();
        
        // 通常の停止を試行
        match self.stop() {
            Ok(()) => return Ok(()),
            Err(_) => {
                // 通常停止に失敗した場合、緊急停止を実行
                self.emergency_stop();
                
                // タイムアウトまで停止を確認
                while start.elapsed() < timeout {
                    if !self.control_pin.is_set_high() {
                        return Ok(());
                    }
                    thread::sleep(Duration::from_millis(10));
                }
                
                return Err(PumpError::StopFailure);
            }
        }
    }
}

/// ポンプの状態情報
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct PumpStatus {
    pub is_running: bool,
    pub current_runtime: Duration,
    pub total_runtime: Duration,
    pub gpio_state: bool,
}

impl PumpStatus {
    /// 状態の文字列表現
    #[allow(dead_code)]
    pub fn as_str(&self) -> &str {
        if self.is_running {
            "Running"
        } else {
            "Stopped"
        }
    }
    
    /// 状態が正常かチェック
    #[allow(dead_code)]
    pub fn is_healthy(&self) -> bool {
        self.is_running == self.gpio_state
    }
}

// Drop実装で安全にリソースを解放
#[allow(dead_code)]
impl Drop for PumpController {
    fn drop(&mut self) {
        if self.is_running {
            self.emergency_stop();
        }
    }
}