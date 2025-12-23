use crate::controllers::{WeightSensorController, PumpController, ButtonController};
use crate::errors::SystemError;
use crate::models::{SystemConfig, SystemState, WeightReading};
use std::time::{Duration, Instant};
use std::thread;
use log::{info, warn, error, debug};

/// システム全体を制御するコントローラ
pub struct SystemController {
    weight_sensor: WeightSensorController,
    pump: PumpController,
    button: ButtonController,
    config: SystemConfig,
    state: SystemState,
    last_weight_reading: Option<WeightReading>,
    error_count: u32,
    start_time: Option<Instant>,
}

impl SystemController {
    /// 新しいシステムコントローラを作成
    pub fn new(config: SystemConfig) -> Result<Self, SystemError> {
        // 設定値の妥当性をチェック
        config.validate().map_err(SystemError::Config)?;
        
        info!("Initializing system controller with config: {:?}", config);
        
        // GPIO初期化
        Self::initialize_gpio(&config)?;
        
        // 各コントローラを初期化
        let weight_sensor = WeightSensorController::new(
            config.dt_pin,
            config.sck_pin,
            config.calibration_factor,
            config.moving_average_window,
        )?;
        
        let pump = PumpController::new(config.pump_pin)?;
        let button = ButtonController::new(config.button_pin)?;
        
        let mut controller = Self {
            weight_sensor,
            pump,
            button,
            config,
            state: SystemState::Idle,
            last_weight_reading: None,
            error_count: 0,
            start_time: None,
        };
        
        // 初期化後の健全性チェック
        controller.health_check()?;
        
        info!("System controller initialized successfully");
        Ok(controller)
    }
    
    /// GPIO初期化処理
    fn initialize_gpio(config: &SystemConfig) -> Result<(), SystemError> {
        info!("Initializing GPIO pins");
        
        use rppal::gpio::Gpio;
        
        let gpio = Gpio::new().map_err(|e| SystemError::Gpio(e.to_string()))?;
        
        // 使用するピンの一覧
        let pins = vec![
            (config.dt_pin, "HX711 DT"),
            (config.sck_pin, "HX711 SCK"),
            (config.pump_pin, "Pump Control"),
            (config.button_pin, "Button"),
        ];
        
        // 各ピンが利用可能かチェック
        for (pin_num, description) in &pins {
            match gpio.get(*pin_num) {
                Ok(_) => {
                    debug!("GPIO {} ({}) is available", pin_num, description);
                }
                Err(e) => {
                    error!("GPIO {} ({}) is not available: {}", pin_num, description, e);
                    return Err(SystemError::Gpio(format!(
                        "GPIO {} ({}) initialization failed: {}", 
                        pin_num, description, e
                    )));
                }
            }
        }
        
        // 出力ピンを安全な初期状態に設定
        {
            // SCKピン（出力）をLOWに設定
            let mut sck_pin = gpio.get(config.sck_pin)
                .map_err(|e| SystemError::Gpio(e.to_string()))?
                .into_output();
            sck_pin.set_low();
            debug!("SCK pin (GPIO {}) set to LOW", config.sck_pin);
        }
        
        {
            // ポンプ制御ピン（出力）をLOWに設定
            let mut pump_pin = gpio.get(config.pump_pin)
                .map_err(|e| SystemError::Gpio(e.to_string()))?
                .into_output();
            pump_pin.set_low();
            debug!("Pump control pin (GPIO {}) set to LOW", config.pump_pin);
        }
        
        // 入力ピンの設定確認
        {
            // DTピン（入力）の設定確認
            let _dt_pin = gpio.get(config.dt_pin)
                .map_err(|e| SystemError::Gpio(e.to_string()))?
                .into_input();
            debug!("DT pin (GPIO {}) configured as input", config.dt_pin);
        }
        
        {
            // ボタンピン（プルアップ入力）の設定確認
            let _button_pin = gpio.get(config.button_pin)
                .map_err(|e| SystemError::Gpio(e.to_string()))?
                .into_input_pullup();
            debug!("Button pin (GPIO {}) configured as input with pullup", config.button_pin);
        }
        
        info!("GPIO initialization completed successfully");
        Ok(())
    }
    
    /// システム起動時の初期化シーケンス
    pub fn startup_sequence(&mut self) -> Result<(), SystemError> {
        info!("Starting system startup sequence");
        
        // 1. 健全性チェック
        self.health_check()?;
        
        // 2. 重量センサーの初期化
        info!("Initializing weight sensor");
        thread::sleep(Duration::from_millis(500)); // センサー安定化待機
        
        // 3. 初期重量測定
        match self.weight_sensor.read_weight() {
            Ok(reading) => {
                self.last_weight_reading = Some(reading.clone());
                info!("Initial weight reading: {:.1}g", reading.value);
            }
            Err(e) => {
                warn!("Initial weight reading failed: {}", e);
                // 起動時は警告のみで続行
            }
        }
        
        // 4. ポンプの初期状態確認
        if self.pump.is_running() {
            warn!("Pump was running at startup, stopping");
            self.pump.emergency_stop();
        }
        
        // 5. ボタンの初期状態確認
        self.button.reset_press_count();
        
        // 6. システム状態を待機に設定
        self.state = SystemState::Idle;
        self.error_count = 0;
        
        info!("System startup sequence completed successfully");
        Ok(())
    }
    
    /// システム終了時のクリーンアップ
    pub fn shutdown_sequence(&mut self) -> Result<(), SystemError> {
        info!("Starting system shutdown sequence");
        
        // 1. ポンプを安全に停止
        if self.pump.is_running() {
            info!("Stopping pump for shutdown");
            match self.pump.safe_stop(Duration::from_secs(5)) {
                Ok(()) => info!("Pump stopped successfully"),
                Err(e) => {
                    warn!("Failed to stop pump gracefully: {}", e);
                    self.pump.emergency_stop();
                    info!("Pump emergency stopped");
                }
            }
        }
        
        // 2. 最終重量測定
        match self.weight_sensor.read_weight() {
            Ok(reading) => {
                info!("Final weight reading: {:.1}g", reading.value);
            }
            Err(e) => {
                debug!("Final weight reading failed: {}", e);
            }
        }
        
        // 3. 統計情報の出力
        let stats = self.statistics();
        info!("System statistics: {:?}", stats);
        
        // 4. 状態をリセット
        self.state = SystemState::Idle;
        self.start_time = None;
        self.error_count = 0;
        
        info!("System shutdown sequence completed");
        Ok(())
    }
    
    /// システムの健全性チェック
    pub fn health_check(&mut self) -> Result<(), SystemError> {
        debug!("Performing system health check");
        
        // 各コンポーネントの健全性をチェック
        self.weight_sensor.health_check()?;
        self.pump.health_check()?;
        self.button.health_check()?;
        
        debug!("System health check passed");
        Ok(())
    }
    
    /// システム状態を取得
    #[allow(dead_code)]
    pub fn state(&self) -> &SystemState {
        &self.state
    }
    
    /// 最後の重量測定値を取得
    #[allow(dead_code)]
    pub fn last_weight_reading(&self) -> Option<&WeightReading> {
        self.last_weight_reading.as_ref()
    }
    
    /// システム設定を取得
    #[allow(dead_code)]
    pub fn config(&self) -> &SystemConfig {
        &self.config
    }
    
    /// エラー回数を取得
    #[allow(dead_code)]
    pub fn error_count(&self) -> u32 {
        self.error_count
    }
    
    /// メインの制御ループを実行
    #[allow(dead_code)]
    pub fn run(&mut self) -> Result<(), SystemError> {
        info!("Starting system control loop");
        
        loop {
            match self.process_cycle() {
                Ok(should_continue) => {
                    if !should_continue {
                        break;
                    }
                }
                Err(e) => {
                    self.handle_error(e)?;
                }
            }
            
            thread::sleep(Duration::from_millis(100));
        }
        
        info!("System control loop ended");
        Ok(())
    }
    
    /// 1サイクルの処理を実行
    pub fn process_cycle(&mut self) -> Result<bool, SystemError> {
        // ボタン状態をチェック
        if self.button.detect_press() {
            self.handle_button_press()?;
        }
        
        // 重量を測定
        match self.weight_sensor.read_weight() {
            Ok(reading) => {
                self.last_weight_reading = Some(reading.clone());
                self.process_weight_reading(reading)?;
            }
            Err(e) => {
                warn!("Weight reading failed: {}", e);
                if self.error_count < 3 {
                    self.error_count += 1;
                } else {
                    return Err(SystemError::Sensor(e));
                }
            }
        }
        
        // 状態に応じた処理
        match &self.state {
            SystemState::Pumping => {
                self.check_target_weight()?;
            }
            SystemState::Stopping => {
                self.finalize_stop()?;
            }
            SystemState::Error(_) => {
                // エラー状態では処理を停止
                return Ok(false);
            }
            _ => {}
        }
        
        Ok(true)
    }
    
    /// ボタン押下を処理
    fn handle_button_press(&mut self) -> Result<(), SystemError> {
        info!("Button pressed, current state: {:?}", self.state);
        
        match &self.state {
            SystemState::Idle => {
                self.start_pumping()?;
            }
            SystemState::Measuring | SystemState::Pumping => {
                self.stop_pumping()?;
            }
            SystemState::Stopping => {
                // 停止処理中は無視
                debug!("Button press ignored during stopping");
            }
            SystemState::Error(_) => {
                // エラー状態からリセット
                self.reset_system()?;
            }
        }
        
        Ok(())
    }
    
    /// ポンプ動作を開始
    fn start_pumping(&mut self) -> Result<(), SystemError> {
        info!("Starting pump operation");
        
        // 重量センサーをリセット
        self.weight_sensor.reset_filter();
        
        // ポンプを開始
        self.pump.start()?;
        
        self.state = SystemState::Pumping;
        self.start_time = Some(Instant::now());
        self.error_count = 0;
        
        info!("Pump started successfully");
        Ok(())
    }
    
    /// ポンプ動作を停止
    fn stop_pumping(&mut self) -> Result<(), SystemError> {
        info!("Stopping pump operation");
        
        self.state = SystemState::Stopping;
        
        // ポンプを安全に停止
        self.pump.safe_stop(Duration::from_secs(5))?;
        
        self.finalize_stop()?;
        
        Ok(())
    }
    
    /// 停止処理を完了
    fn finalize_stop(&mut self) -> Result<(), SystemError> {
        self.state = SystemState::Idle;
        self.start_time = None;
        
        if let Some(reading) = &self.last_weight_reading {
            info!("Pumping completed. Final weight: {:.1}g", reading.value);
        }
        
        Ok(())
    }
    
    /// 重量測定値を処理
    fn process_weight_reading(&mut self, reading: WeightReading) -> Result<(), SystemError> {
        debug!("Weight reading: {:.1}g (stable: {})", reading.value, reading.is_stable);
        
        // 測定値が有効範囲内かチェック
        if !reading.is_valid() {
            warn!("Invalid weight reading: {:.1}g", reading.value);
            return Ok(());
        }
        
        // エラーカウントをリセット
        self.error_count = 0;
        
        Ok(())
    }
    
    /// 目標重量に達したかチェック
    fn check_target_weight(&mut self) -> Result<(), SystemError> {
        if let Some(reading) = &self.last_weight_reading {
            if reading.is_stable && reading.value >= self.config.target_weight {
                info!("Target weight reached: {:.1}g >= {:.1}g", 
                      reading.value, self.config.target_weight);
                self.stop_pumping()?;
            }
        }
        
        Ok(())
    }
    
    /// エラーを処理
    #[allow(dead_code)]
    fn handle_error(&mut self, error: SystemError) -> Result<(), SystemError> {
        error!("System error: {}", error);
        
        // 重要度に応じた処理
        match error.level() {
            crate::errors::ErrorLevel::Warning => {
                warn!("Warning level error, continuing operation");
                Ok(())
            }
            crate::errors::ErrorLevel::Error => {
                if error.is_retryable() && self.error_count < 3 {
                    warn!("Retryable error, attempt {}", self.error_count + 1);
                    self.error_count += 1;
                    thread::sleep(Duration::from_millis(500));
                    Ok(())
                } else {
                    self.enter_error_state(error)
                }
            }
            crate::errors::ErrorLevel::Critical => {
                self.enter_error_state(error)
            }
        }
    }
    
    /// エラー状態に遷移
    #[allow(dead_code)]
    fn enter_error_state(&mut self, error: SystemError) -> Result<(), SystemError> {
        error!("Entering error state: {}", error);
        
        // ポンプを緊急停止
        if self.pump.is_running() {
            self.pump.emergency_stop();
        }
        
        self.state = SystemState::Error(error.to_string());
        self.start_time = None;
        
        Err(error)
    }
    
    /// システムをリセット
    fn reset_system(&mut self) -> Result<(), SystemError> {
        info!("Resetting system");
        
        // ポンプを停止
        if self.pump.is_running() {
            self.pump.emergency_stop();
        }
        
        // 重量センサーをリセット
        self.weight_sensor.reset_filter();
        
        // ボタンの押下回数をリセット
        self.button.reset_press_count();
        
        self.state = SystemState::Idle;
        self.start_time = None;
        self.error_count = 0;
        self.last_weight_reading = None;
        
        // 健全性チェック
        self.health_check()?;
        
        info!("System reset completed");
        Ok(())
    }
    
    /// 校正を実行
    #[allow(dead_code)]
    pub fn calibrate(&mut self) -> Result<(), SystemError> {
        info!("Starting calibration");
        
        if self.state.is_active() {
            return Err(SystemError::Config("Cannot calibrate while system is active".to_string()));
        }
        
        self.weight_sensor.calibrate_zero()?;
        
        info!("Calibration completed");
        Ok(())
    }
    
    /// 既知の重量での校正
    #[allow(dead_code)]
    pub fn calibrate_with_weight(&mut self, known_weight: f32) -> Result<(), SystemError> {
        info!("Starting calibration with known weight: {}g", known_weight);
        
        if self.state.is_active() {
            return Err(SystemError::Config("Cannot calibrate while system is active".to_string()));
        }
        
        self.weight_sensor.calibrate_with_known_weight(known_weight)?;
        
        info!("Calibration with known weight completed");
        Ok(())
    }
    
    /// システム統計を取得
    pub fn statistics(&self) -> SystemStatistics {
        SystemStatistics {
            state: self.state.clone(),
            error_count: self.error_count,
            pump_runtime: self.pump.total_runtime(),
            button_press_count: self.button.press_count(),
            current_weight: self.last_weight_reading.as_ref().map(|r| r.value),
            target_weight: self.config.target_weight,
            uptime: self.start_time.map(|t| t.elapsed()),
        }
    }
}

/// システム統計情報
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct SystemStatistics {
    pub state: SystemState,
    pub error_count: u32,
    pub pump_runtime: Duration,
    pub button_press_count: u32,
    pub current_weight: Option<f32>,
    pub target_weight: f32,
    pub uptime: Option<Duration>,
}

// Drop実装で安全にリソースを解放
impl Drop for SystemController {
    fn drop(&mut self) {
        if self.pump.is_running() {
            self.pump.emergency_stop();
        }
        info!("System controller dropped safely");
    }
}