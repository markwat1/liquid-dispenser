use weight_sensor_pump_controller::{SystemConfig, SystemController, SystemState};

/// 統合テスト用のモックシステムコントローラ
/// 実際のGPIOを使用せずにシステムの動作をテスト
#[cfg(test)]
mod integration_tests {
    use super::*;

    #[test]
    fn test_system_config_creation() {
        let config = SystemConfig::default();
        assert_eq!(config.target_weight, 50.0);
        assert_eq!(config.dt_pin, 5);
        assert_eq!(config.sck_pin, 6);
        assert_eq!(config.relay_a_pin, 18);
        assert_eq!(config.relay_b_pin, 19);
        assert_eq!(config.button_pin, 2);
    }

    #[test]
    fn test_system_config_validation() {
        let valid_config = SystemConfig::default();
        assert!(valid_config.validate().is_ok());

        let mut invalid_config = SystemConfig::default();
        invalid_config.target_weight = -10.0;
        assert!(invalid_config.validate().is_err());
    }

    #[test]
    fn test_system_config_from_compiled() {
        let config = SystemConfig::from_compiled();
        // コンパイル時設定が正しく読み込まれることを確認
        assert!(config.target_weight > 0.0);
        assert!(config.dt_pin < 40); // 有効なGPIO範囲
        assert!(config.sck_pin < 40);
        assert!(config.relay_a_pin < 40);
        assert!(config.relay_b_pin < 40);
        assert!(config.button_pin < 40);
    }

    #[test]
    fn test_build_info() {
        let build_info = SystemConfig::build_info();
        assert!(!build_info.version.is_empty());
        assert!(!build_info.target.is_empty());
        assert!(!build_info.timestamp.is_empty());
        assert!(build_info.compiled_target_weight >= 0.0);
    }

    #[test]
    fn test_config_priority_loading() {
        // 環境変数をクリア
        std::env::remove_var("TARGET_WEIGHT");
        
        let config = SystemConfig::load_with_priority();
        assert!(config.validate().is_ok());
        
        // 環境変数を設定してテスト
        std::env::set_var("TARGET_WEIGHT", "75.5");
        let config_with_env = SystemConfig::load_with_priority();
        assert_eq!(config_with_env.target_weight, 75.5);
        
        // クリーンアップ
        std::env::remove_var("TARGET_WEIGHT");
    }

    // 注意: 実際のGPIOが必要なテストはRaspberry Pi上でのみ実行
    #[test]
    #[ignore] // 実際のハードウェアが必要
    fn test_system_controller_initialization() {
        let config = SystemConfig::default();
        
        // 実際のハードウェアがある場合のみテスト
        match SystemController::new(config) {
            Ok(mut controller) => {
                // 健全性チェック
                assert!(controller.health_check().is_ok());
                
                // 初期状態の確認
                assert_eq!(*controller.state(), SystemState::Idle);
                assert_eq!(controller.error_count(), 0);
            }
            Err(_) => {
                // ハードウェアがない場合はスキップ
                println!("Hardware not available, skipping hardware test");
            }
        }
    }

    #[test]
    fn test_system_state_transitions() {
        let idle = SystemState::Idle;
        let forward = SystemState::Forward;
        let stopping = SystemState::Stopping;
        let error = SystemState::Error("Test error".to_string());

        assert!(!idle.is_active());
        assert!(!idle.is_error());
        assert_eq!(idle.as_str(), "Idle");

        assert!(forward.is_active());
        assert!(!forward.is_error());
        assert_eq!(forward.as_str(), "Forward");

        assert!(stopping.is_active());
        assert!(!stopping.is_error());
        assert_eq!(stopping.as_str(), "Stopping");

        assert!(!error.is_active());
        assert!(error.is_error());
        assert_eq!(error.as_str(), "Error");
    }

    #[test]
    fn test_weight_reading_properties() {
        use weight_sensor_pump_controller::WeightReading;
        
        let reading = WeightReading::new(25.5, true);
        assert_eq!(reading.value, 25.5);
        assert!(reading.is_stable);
        assert!(reading.is_valid());
        
        let invalid_reading = WeightReading::new(-5.0, false);
        assert!(!invalid_reading.is_valid());
        
        let over_limit_reading = WeightReading::new(150.0, true);
        assert!(!over_limit_reading.is_valid());
    }

    #[test]
    fn test_error_handling_properties() {
        use weight_sensor_pump_controller::SystemError;
        use weight_sensor_pump_controller::errors::{SensorError, MotorError};
        
        let sensor_error = SystemError::Sensor(SensorError::ReadFailure);
        assert!(sensor_error.is_retryable());
        
        let motor_error = SystemError::Motor(MotorError::StartFailure);
        assert!(motor_error.is_retryable());
        
        let config_error = SystemError::Config("Invalid config".to_string());
        assert!(!config_error.is_retryable());
    }
}