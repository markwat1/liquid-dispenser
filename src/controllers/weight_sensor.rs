use crate::errors::SensorError;
use crate::models::{WeightReading, MovingAverage};
use rppal::gpio::{Gpio, InputPin, OutputPin};
use std::thread;
use std::time::{Duration, Instant};
use log::{info, warn, debug};

/// HX711重量センサーコントローラ
pub struct WeightSensorController {
    dt_pin: InputPin,
    sck_pin: OutputPin,
    calibration_factor: f32,
    zero_offset: i32,
    moving_average: MovingAverage,
    max_weight: f32,
}

impl WeightSensorController {
    /// 新しい重量センサーコントローラを作成
    pub fn new(dt_pin_num: u8, sck_pin_num: u8, calibration_factor: f32, window_size: usize) -> Result<Self, SensorError> {
        let gpio = Gpio::new().map_err(|e| SensorError::Gpio(e.to_string()))?;
        
        let dt_pin = gpio.get(dt_pin_num).map_err(|e| SensorError::Gpio(e.to_string()))?
            .into_input();
        let mut sck_pin = gpio.get(sck_pin_num).map_err(|e| SensorError::Gpio(e.to_string()))?
            .into_output();
        
        // SCKピンを初期化（LOW）
        sck_pin.set_low();
        
        let mut controller = Self {
            dt_pin,
            sck_pin,
            calibration_factor,
            zero_offset: 0,
            moving_average: MovingAverage::new(window_size),
            max_weight: 100.0,
        };
        
        // 初期化時にセンサーをリセット
        controller.reset_sensor()?;
        
        Ok(controller)
    }
    
    /// センサーをリセット
    fn reset_sensor(&mut self) -> Result<(), SensorError> {
        // SCKを25回パルスしてリセット
        for _ in 0..25 {
            self.sck_pin.set_high();
            thread::sleep(Duration::from_micros(1));
            self.sck_pin.set_low();
            thread::sleep(Duration::from_micros(1));
        }
        
        // センサーが準備完了まで待機
        self.wait_for_ready(Duration::from_millis(100))?;
        
        Ok(())
    }
    
    /// センサーが準備完了まで待機
    fn wait_for_ready(&self, timeout: Duration) -> Result<(), SensorError> {
        let start = Instant::now();
        
        while self.dt_pin.is_high() {
            if start.elapsed() > timeout {
                return Err(SensorError::Timeout);
            }
            thread::sleep(Duration::from_micros(10));
        }
        
        Ok(())
    }
    
    /// HX711から生データを読み取り
    fn read_raw(&mut self) -> Result<i32, SensorError> {
        // センサーが準備完了まで待機
        self.wait_for_ready(Duration::from_millis(100))?;
        
        let mut value: i32 = 0;
        
        // 24ビットのデータを読み取り
        for _ in 0..24 {
            self.sck_pin.set_high();
            thread::sleep(Duration::from_micros(1));
            
            value <<= 1;
            if self.dt_pin.is_high() {
                value |= 1;
            }
            
            self.sck_pin.set_low();
            thread::sleep(Duration::from_micros(1));
        }
        
        // ゲイン設定のための追加パルス（128倍ゲイン用に1パルス）
        self.sck_pin.set_high();
        thread::sleep(Duration::from_micros(1));
        self.sck_pin.set_low();
        thread::sleep(Duration::from_micros(1));
        
        // 24ビット符号付き整数に変換
        if value & 0x800000 != 0 {
            value |= 0xFF000000u32 as i32;
        }
        
        Ok(value)
    }
    
    /// 重量を読み取り
    pub fn read_weight(&mut self) -> Result<WeightReading, SensorError> {
        const MAX_RETRIES: usize = 3;
        let mut last_error = None;
        
        for attempt in 0..MAX_RETRIES {
            match self.try_read_weight() {
                Ok(reading) => return Ok(reading),
                Err(e) => {
                    last_error = Some(e);
                    if attempt < MAX_RETRIES - 1 {
                        thread::sleep(Duration::from_millis(10));
                    }
                }
            }
        }
        
        Err(last_error.unwrap_or(SensorError::ReadFailure))
    }
    
    /// 重量読み取りの実際の処理
    fn try_read_weight(&mut self) -> Result<WeightReading, SensorError> {
        let raw_value = self.read_raw()?;
        
        // ゼロオフセットを適用
        let adjusted_value = raw_value - self.zero_offset;
        
        // 校正係数を適用して重量に変換
        let weight = adjusted_value as f32 / self.calibration_factor;
        
        // 範囲チェック
        if weight < 0.0 || weight > self.max_weight {
            return Err(SensorError::OutOfRange { 
                value: weight, 
                max: self.max_weight 
            });
        }
        
        // 移動平均フィルタを適用
        let filtered_weight = self.moving_average.add(weight);
        let is_stable = self.moving_average.is_stable(0.5); // 0.5g以内の変動で安定とみなす
        
        Ok(WeightReading::new(filtered_weight, is_stable))
    }
    
    /// ゼロ点校正
    #[allow(dead_code)]
    pub fn calibrate_zero(&mut self) -> Result<(), SensorError> {
        const CALIBRATION_SAMPLES: usize = 10;
        let mut sum = 0i64;
        
        // 移動平均をリセット
        self.moving_average.reset();
        
        // 複数回測定して平均を取る
        for _ in 0..CALIBRATION_SAMPLES {
            let raw_value = self.read_raw()?;
            sum += raw_value as i64;
            thread::sleep(Duration::from_millis(100));
        }
        
        self.zero_offset = (sum / CALIBRATION_SAMPLES as i64) as i32;
        
        Ok(())
    }
    
    /// 重量が範囲内かチェック
    #[allow(dead_code)]
    pub fn is_in_range(&self, weight: f32) -> bool {
        weight >= 0.0 && weight <= self.max_weight
    }
    
    /// 校正係数を設定
    #[allow(dead_code)]
    pub fn set_calibration_factor(&mut self, factor: f32) {
        self.calibration_factor = factor;
    }
    
    /// 最大重量を設定
    #[allow(dead_code)]
    pub fn set_max_weight(&mut self, max_weight: f32) {
        self.max_weight = max_weight;
    }
    
    /// センサーが利用可能かチェック
    pub fn is_available(&self) -> bool {
        self.dt_pin.is_low()
    }
    
    /// 移動平均ウィンドウサイズを変更
    #[allow(dead_code)]
    pub fn set_moving_average_window(&mut self, window_size: usize) {
        self.moving_average = MovingAverage::new(window_size);
    }
    
    /// 現在の移動平均値を取得
    #[allow(dead_code)]
    pub fn get_current_average(&self) -> Option<f32> {
        self.moving_average.average()
    }
    
    /// フィルタをリセット
    pub fn reset_filter(&mut self) {
        self.moving_average.reset();
    }
    
    /// 高度な校正（既知の重量での校正）
    #[allow(dead_code)]
    pub fn calibrate_with_known_weight(&mut self, known_weight: f32) -> Result<(), SensorError> {
        if known_weight <= 0.0 {
            return Err(SensorError::CalibrationFailed);
        }
        
        const CALIBRATION_SAMPLES: usize = 10;
        let mut sum = 0i64;
        
        // 複数回測定して平均を取る
        for _ in 0..CALIBRATION_SAMPLES {
            let raw_value = self.read_raw()?;
            sum += (raw_value - self.zero_offset) as i64;
            thread::sleep(Duration::from_millis(100));
        }
        
        let average_raw = sum as f32 / CALIBRATION_SAMPLES as f32;
        self.calibration_factor = average_raw / known_weight;
        
        if self.calibration_factor <= 0.0 {
            return Err(SensorError::CalibrationFailed);
        }
        
        Ok(())
    }
    
    /// 重量安定化処理を実行（3秒間の継続測定）
    pub fn stabilize_weight(&mut self, duration: Duration, tolerance: f32) -> Result<f32, SensorError> {
        info!("Starting weight stabilization for {:?}", duration);
        
        let start_time = Instant::now();
        let mut readings = Vec::new();
        
        // フィルタをリセット
        self.reset_filter();
        
        // 指定時間内で継続的に測定
        while start_time.elapsed() < duration {
            match self.read_weight() {
                Ok(reading) => {
                    readings.push(reading.value);
                    debug!("Stabilization reading: {:.1}g", reading.value);
                }
                Err(e) => {
                    warn!("Failed to read weight during stabilization: {}", e);
                    // エラーが発生しても継続
                }
            }
            
            thread::sleep(Duration::from_millis(100)); // 100ms間隔で測定
        }
        
        if readings.is_empty() {
            return Err(SensorError::ReadFailure);
        }
        
        // 安定性をチェック
        if self.check_weight_stability(&readings, tolerance) {
            let average_weight = readings.iter().sum::<f32>() / readings.len() as f32;
            info!("Weight stabilized at {:.1}g", average_weight);
            Ok(average_weight)
        } else {
            warn!("Weight did not stabilize within tolerance {:.1}g", tolerance);
            Err(SensorError::CalibrationFailed) // 安定化失敗
        }
    }
    
    /// 重量の安定性をチェック
    pub fn check_weight_stability(&self, readings: &[f32], tolerance: f32) -> bool {
        if readings.len() < 5 {
            return false; // 最低5回の測定が必要
        }
        
        let average = readings.iter().sum::<f32>() / readings.len() as f32;
        
        // 全ての測定値が許容範囲内かチェック
        for &reading in readings {
            if (reading - average).abs() > tolerance {
                return false;
            }
        }
        
        true
    }
    
    /// 継続的な重量監視（目標重量到達チェック用）
    pub fn monitor_weight_for_target(&mut self, target_weight: f32, base_weight: f32) -> Result<bool, SensorError> {
        let reading = self.read_weight()?;
        
        if reading.is_stable {
            let net_weight = reading.value - base_weight;
            debug!("Net weight: {:.1}g (target: {:.1}g)", net_weight, target_weight);
            
            if net_weight >= target_weight {
                info!("Target weight reached: {:.1}g >= {:.1}g", net_weight, target_weight);
                return Ok(true);
            }
        }
        
        Ok(false)
    }
    
    /// センサーの健全性チェック
    pub fn health_check(&mut self) -> Result<(), SensorError> {
        // センサーが応答するかチェック
        if !self.is_available() {
            return Err(SensorError::ReadFailure);
        }
        
        // 複数回読み取りを試行
        for _ in 0..3 {
            match self.read_raw() {
                Ok(_) => return Ok(()),
                Err(e) => {
                    thread::sleep(Duration::from_millis(50));
                    if matches!(e, SensorError::Timeout) {
                        continue;
                    }
                    return Err(e);
                }
            }
        }
        
        Err(SensorError::ReadFailure)
    }
}