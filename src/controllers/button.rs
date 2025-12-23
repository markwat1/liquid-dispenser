use crate::errors::ButtonError;
use rppal::gpio::{Gpio, InputPin};
use std::time::{Duration, Instant};
use std::thread;

/// ボタンコントローラ
pub struct ButtonController {
    button_pin: InputPin,
    last_state: bool,
    debounce_timer: Instant,
    debounce_duration: Duration,
    press_count: u32,
}

impl ButtonController {
    /// 新しいボタンコントローラを作成
    pub fn new(button_pin_num: u8) -> Result<Self, ButtonError> {
        let gpio = Gpio::new().map_err(|e| ButtonError::Gpio(e.to_string()))?;
        
        let button_pin = gpio.get(button_pin_num)
            .map_err(|e| ButtonError::Gpio(e.to_string()))?
            .into_input_pullup(); // プルアップ抵抗を有効化
        
        let initial_state = button_pin.is_low(); // プルアップなので押下時はLOW
        
        Ok(Self {
            button_pin,
            last_state: initial_state,
            debounce_timer: Instant::now(),
            debounce_duration: Duration::from_millis(50), // 50msのデバウンス
            press_count: 0,
        })
    }
    
    /// ボタンが押されているかチェック（デバウンス処理付き）
    #[allow(dead_code)]
    pub fn is_pressed(&mut self) -> bool {
        let current_state = self.button_pin.is_low(); // プルアップなので押下時はLOW
        let now = Instant::now();
        
        // デバウンス期間中は前回の状態を返す
        if now.duration_since(self.debounce_timer) < self.debounce_duration {
            return self.last_state;
        }
        
        // 状態が変化した場合
        if current_state != self.last_state {
            self.debounce_timer = now;
            self.last_state = current_state;
            
            // 押下された場合（HIGHからLOWに変化）
            if current_state {
                self.press_count += 1;
            }
        }
        
        current_state
    }
    
    /// ボタン押下を待機
    #[allow(dead_code)]
    pub fn wait_for_press(&mut self) -> Result<(), ButtonError> {
        self.wait_for_press_with_timeout(Duration::from_secs(u64::MAX))
    }
    
    /// タイムアウト付きでボタン押下を待機
    #[allow(dead_code)]
    pub fn wait_for_press_with_timeout(&mut self, timeout: Duration) -> Result<(), ButtonError> {
        let start = Instant::now();
        let mut was_pressed = self.is_pressed();
        
        while start.elapsed() < timeout {
            let is_pressed = self.is_pressed();
            
            // 押下状態から非押下状態に変化した場合（ボタンが離された）
            if was_pressed && !is_pressed {
                return Ok(());
            }
            
            was_pressed = is_pressed;
            thread::sleep(Duration::from_millis(10));
        }
        
        Err(ButtonError::DebounceTimeout)
    }
    
    /// ボタンが押下された瞬間を検出
    pub fn detect_press(&mut self) -> bool {
        let current_state = self.button_pin.is_low();
        let now = Instant::now();
        
        // デバウンス期間中は無視
        if now.duration_since(self.debounce_timer) < self.debounce_duration {
            return false;
        }
        
        // 前回が非押下で今回が押下の場合
        if !self.last_state && current_state {
            self.debounce_timer = now;
            self.last_state = current_state;
            self.press_count += 1;
            return true;
        }
        
        // 状態が変化した場合はタイマーをリセット
        if current_state != self.last_state {
            self.debounce_timer = now;
            self.last_state = current_state;
        }
        
        false
    }
    
    /// ボタンが離された瞬間を検出
    #[allow(dead_code)]
    pub fn detect_release(&mut self) -> bool {
        let current_state = self.button_pin.is_low();
        let now = Instant::now();
        
        // デバウンス期間中は無視
        if now.duration_since(self.debounce_timer) < self.debounce_duration {
            return false;
        }
        
        // 前回が押下で今回が非押下の場合
        if self.last_state && !current_state {
            self.debounce_timer = now;
            self.last_state = current_state;
            return true;
        }
        
        // 状態が変化した場合はタイマーをリセット
        if current_state != self.last_state {
            self.debounce_timer = now;
            self.last_state = current_state;
        }
        
        false
    }
    
    /// 押下回数を取得
    pub fn press_count(&self) -> u32 {
        self.press_count
    }
    
    /// 押下回数をリセット
    pub fn reset_press_count(&mut self) {
        self.press_count = 0;
    }
    
    /// デバウンス時間を設定
    #[allow(dead_code)]
    pub fn set_debounce_duration(&mut self, duration: Duration) {
        self.debounce_duration = duration;
    }
    
    /// 現在のボタン状態を取得
    #[allow(dead_code)]
    pub fn current_state(&self) -> bool {
        self.button_pin.is_low()
    }
    
    /// ボタンの状態情報を取得
    #[allow(dead_code)]
    pub fn status(&self) -> ButtonStatus {
        ButtonStatus {
            is_pressed: self.last_state,
            press_count: self.press_count,
            debounce_remaining: self.debounce_duration
                .saturating_sub(self.debounce_timer.elapsed()),
            gpio_state: self.button_pin.is_low(),
        }
    }
    
    /// ボタンの健全性チェック
    pub fn health_check(&self) -> Result<(), ButtonError> {
        // GPIO読み取りテスト
        match self.button_pin.is_low() {
            true | false => Ok(()), // 正常に読み取れた
        }
    }
    
    /// 長押し検出
    #[allow(dead_code)]
    pub fn detect_long_press(&mut self, long_press_duration: Duration) -> bool {
        if !self.is_pressed() {
            return false;
        }
        
        // 押下開始からの経過時間をチェック
        self.debounce_timer.elapsed() >= long_press_duration
    }
    
    /// ダブルクリック検出
    #[allow(dead_code)]
    pub fn detect_double_click(&mut self, double_click_window: Duration) -> bool {
        static mut LAST_PRESS_TIME: Option<Instant> = None;
        
        if self.detect_press() {
            unsafe {
                if let Some(last_time) = LAST_PRESS_TIME {
                    if last_time.elapsed() <= double_click_window {
                        LAST_PRESS_TIME = None;
                        return true;
                    }
                }
                LAST_PRESS_TIME = Some(Instant::now());
            }
        }
        
        false
    }
}

/// ボタンの状態情報
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct ButtonStatus {
    pub is_pressed: bool,
    pub press_count: u32,
    pub debounce_remaining: Duration,
    pub gpio_state: bool,
}

impl ButtonStatus {
    /// 状態の文字列表現
    #[allow(dead_code)]
    pub fn as_str(&self) -> &str {
        if self.is_pressed {
            "Pressed"
        } else {
            "Released"
        }
    }
    
    /// デバウンス中かチェック
    #[allow(dead_code)]
    pub fn is_debouncing(&self) -> bool {
        self.debounce_remaining > Duration::new(0, 0)
    }
}