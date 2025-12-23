#[cfg(test)]
mod tests {
    use crate::errors::{SystemError, SensorError, PumpError, ErrorLevel};
    
    #[test]
    fn test_system_error_levels() {
        let sensor_out_of_range = SystemError::Sensor(SensorError::OutOfRange { 
            value: 150.0, 
            max: 100.0 
        });
        assert_eq!(sensor_out_of_range.level(), ErrorLevel::Warning);
        
        let sensor_read_failure = SystemError::Sensor(SensorError::ReadFailure);
        assert_eq!(sensor_read_failure.level(), ErrorLevel::Error);
        
        let pump_error = SystemError::Pump(PumpError::StartFailure);
        assert_eq!(pump_error.level(), ErrorLevel::Critical);
        
        let gpio_error = SystemError::Gpio("Test GPIO error".to_string());
        assert_eq!(gpio_error.level(), ErrorLevel::Critical);
    }
    
    #[test]
    fn test_error_retryability() {
        let retryable_error = SystemError::Sensor(SensorError::ReadFailure);
        assert!(retryable_error.is_retryable());
        
        let timeout_error = SystemError::Sensor(SensorError::Timeout);
        assert!(timeout_error.is_retryable());
        
        let non_retryable_error = SystemError::Pump(PumpError::StartFailure);
        assert!(!non_retryable_error.is_retryable());
        
        let config_error = SystemError::Config("Invalid config".to_string());
        assert!(!config_error.is_retryable());
    }
    
    #[test]
    fn test_error_display() {
        let sensor_error = SensorError::OutOfRange { value: 150.0, max: 100.0 };
        let error_string = format!("{}", sensor_error);
        assert!(error_string.contains("150"));
        assert!(error_string.contains("100"));
        
        let pump_error = PumpError::AlreadyRunning;
        let error_string = format!("{}", pump_error);
        assert!(error_string.contains("already running"));
    }
    
    #[test]
    fn test_error_conversion() {
        let sensor_error = SensorError::ReadFailure;
        let system_error: SystemError = sensor_error.into();
        
        match system_error {
            SystemError::Sensor(SensorError::ReadFailure) => {
                // 正しく変換された
            }
            _ => panic!("Error conversion failed"),
        }
    }
}