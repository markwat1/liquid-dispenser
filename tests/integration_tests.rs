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
        assert_eq!(config.pwm_forward_pin, 18);
        assert_eq!(config.pwm_reverse_pin, 19);
        assert_eq!(config.button_pin, 2);
        assert_eq!(config.pwm_frequency, 1000.0);
        assert_eq!(config.forward_speed, 0.8);
        assert_eq!(config.reverse_speed, 0.6);
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
        assert!(config.pwm_forward_pin < 40);
        assert!(config.pwm_reverse_pin < 40);
        assert!(config.button_pin < 40);
        assert!(config.pwm_frequency >= 1.0 && config.pwm_frequency <= 5000.0);
        assert!(config.forward_speed >= 0.0 && config.forward_speed <= 1.0);
        assert!(config.reverse_speed >= 0.0 && config.reverse_speed <= 1.0);
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
    #[test]
    fn test_pwm_configuration_validation() {
        let mut config = SystemConfig::default();
        
        // Valid PWM configuration
        assert!(config.validate().is_ok());
        
        // Invalid PWM frequency (too high)
        config.pwm_frequency = 6000.0;
        assert!(config.validate().is_err());
        
        // Invalid PWM frequency (too low)
        config.pwm_frequency = 0.5;
        assert!(config.validate().is_err());
        
        // Reset to valid frequency
        config.pwm_frequency = 2500.0;
        assert!(config.validate().is_ok());
        
        // Invalid forward speed (too high)
        config.forward_speed = 1.5;
        assert!(config.validate().is_err());
        
        // Invalid reverse speed (negative)
        config.forward_speed = 0.8;
        config.reverse_speed = -0.1;
        assert!(config.validate().is_err());
        
        // Valid speeds
        config.reverse_speed = 0.6;
        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_pwm_pin_configuration() {
        let config = SystemConfig::default();
        
        // Check that PWM pins are different
        assert_ne!(config.pwm_forward_pin, config.pwm_reverse_pin);
        
        // Check that PWM pins are valid GPIO numbers
        assert!(config.pwm_forward_pin < 40);
        assert!(config.pwm_reverse_pin < 40);
        
        // Check that PWM pins don't conflict with other pins
        assert_ne!(config.pwm_forward_pin, config.dt_pin);
        assert_ne!(config.pwm_forward_pin, config.sck_pin);
        assert_ne!(config.pwm_forward_pin, config.button_pin);
        assert_ne!(config.pwm_reverse_pin, config.dt_pin);
        assert_ne!(config.pwm_reverse_pin, config.sck_pin);
        assert_ne!(config.pwm_reverse_pin, config.button_pin);
    }

    #[test]
    fn test_motor_state_with_speed() {
        use weight_sensor_pump_controller::models::MotorState;
        
        let stopped = MotorState::Stopped;
        assert!(!stopped.is_running());
        assert_eq!(stopped.speed(), 0.0);
        assert_eq!(stopped.as_str(), "Stopped");
        
        let forward = MotorState::Forward(0.8);
        assert!(forward.is_running());
        assert_eq!(forward.speed(), 0.8);
        assert_eq!(forward.as_str(), "Forward");
        
        let reverse = MotorState::Reverse(0.6);
        assert!(reverse.is_running());
        assert_eq!(reverse.speed(), 0.6);
        assert_eq!(reverse.as_str(), "Reverse");
    }

    #[test]
    fn test_environment_variable_override() {
        // Test PWM frequency override
        std::env::set_var("PWM_FREQUENCY", "2000.0");
        let config = SystemConfig::load_with_priority();
        assert_eq!(config.pwm_frequency, 2000.0);
        std::env::remove_var("PWM_FREQUENCY");
        
        // Test speed overrides
        std::env::set_var("FORWARD_SPEED", "0.9");
        std::env::set_var("REVERSE_SPEED", "0.7");
        let config = SystemConfig::load_with_priority();
        assert_eq!(config.forward_speed, 0.9);
        assert_eq!(config.reverse_speed, 0.7);
        std::env::remove_var("FORWARD_SPEED");
        std::env::remove_var("REVERSE_SPEED");
        
        // Test PWM pin overrides
        std::env::set_var("PWM_FORWARD_PIN", "12");
        std::env::set_var("PWM_REVERSE_PIN", "13");
        let config = SystemConfig::load_with_priority();
        assert_eq!(config.pwm_forward_pin, 12);
        assert_eq!(config.pwm_reverse_pin, 13);
        std::env::remove_var("PWM_FORWARD_PIN");
        std::env::remove_var("PWM_REVERSE_PIN");
    }

    #[test]
    fn test_system_state_extended() {
        use weight_sensor_pump_controller::models::SystemState;
        
        let stabilizing = SystemState::WeightStabilizing;
        assert!(stabilizing.is_active());
        assert!(!stabilizing.is_error());
        assert_eq!(stabilizing.as_str(), "WeightStabilizing");
        
        let target_reached = SystemState::TargetReached;
        assert!(target_reached.is_active());
        assert!(!target_reached.is_error());
        assert_eq!(target_reached.as_str(), "TargetReached");
        
        let reverse = SystemState::Reverse;
        assert!(reverse.is_active());
        assert!(!reverse.is_error());
        assert_eq!(reverse.as_str(), "Reverse");
        
        let completed = SystemState::Completed;
        assert!(!completed.is_active());
        assert!(!completed.is_error());
        assert_eq!(completed.as_str(), "Completed");
    }