#[cfg(test)]
mod tests {
    use crate::models::{WeightReading, SystemConfig, SystemState, MovingAverage};
    
    #[test]
    fn test_weight_reading_creation() {
        let reading = WeightReading::new(25.5, true);
        assert_eq!(reading.value, 25.5);
        assert!(reading.is_stable);
        assert!(reading.is_valid());
    }
    
    #[test]
    fn test_weight_reading_validation() {
        let valid_reading = WeightReading::new(50.0, true);
        assert!(valid_reading.is_valid());
        
        let invalid_reading_negative = WeightReading::new(-5.0, true);
        assert!(!invalid_reading_negative.is_valid());
        
        let invalid_reading_over_limit = WeightReading::new(150.0, true);
        assert!(!invalid_reading_over_limit.is_valid());
    }
    
    #[test]
    fn test_system_config_default() {
        let config = SystemConfig::default();
        assert_eq!(config.target_weight, 50.0);
        assert_eq!(config.dt_pin, 5);
        assert_eq!(config.sck_pin, 6);
        assert_eq!(config.pump_pin, 18);
        assert_eq!(config.button_pin, 2);
    }
    
    #[test]
    fn test_system_config_validation() {
        let valid_config = SystemConfig::default();
        assert!(valid_config.validate().is_ok());
        
        let mut invalid_config = SystemConfig::default();
        invalid_config.target_weight = -10.0;
        assert!(invalid_config.validate().is_err());
        
        let mut duplicate_pin_config = SystemConfig::default();
        duplicate_pin_config.sck_pin = duplicate_pin_config.dt_pin;
        assert!(duplicate_pin_config.validate().is_err());
    }
    
    #[test]
    fn test_system_state() {
        let idle_state = SystemState::Idle;
        assert!(!idle_state.is_active());
        assert!(!idle_state.is_error());
        assert_eq!(idle_state.as_str(), "Idle");
        
        let pumping_state = SystemState::Pumping;
        assert!(pumping_state.is_active());
        assert!(!pumping_state.is_error());
        
        let error_state = SystemState::Error("Test error".to_string());
        assert!(!error_state.is_active());
        assert!(error_state.is_error());
    }
    
    #[test]
    fn test_moving_average() {
        let mut filter = MovingAverage::new(3);
        
        assert_eq!(filter.add(10.0), 10.0);
        assert_eq!(filter.add(20.0), 15.0);
        assert_eq!(filter.add(30.0), 20.0);
        assert_eq!(filter.add(40.0), 30.0); // 古い値(10.0)が除外される
        
        assert_eq!(filter.average(), Some(30.0));
        
        filter.reset();
        assert_eq!(filter.average(), None);
    }
    
    #[test]
    fn test_moving_average_stability() {
        let mut filter = MovingAverage::new(3);
        
        filter.add(10.0);
        filter.add(10.1);
        filter.add(9.9);
        
        assert!(filter.is_stable(0.5)); // 分散が小さいので安定
        
        filter.add(15.0); // 大きく変化
        assert!(!filter.is_stable(0.5)); // 分散が大きいので不安定
    }
}