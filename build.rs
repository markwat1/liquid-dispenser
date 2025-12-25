use std::env;

fn main() {
    // 環境変数からターゲット重量を取得
    let target_weight = env::var("TARGET_WEIGHT")
        .unwrap_or_else(|_| "50.0".to_string());
    
    // コンパイル時定数として設定
    println!("cargo:rustc-env=COMPILED_TARGET_WEIGHT={}", target_weight);
    
    // 他の設定値も同様に処理
    let dt_pin = env::var("DT_PIN").unwrap_or_else(|_| "5".to_string());
    let sck_pin = env::var("SCK_PIN").unwrap_or_else(|_| "6".to_string());
    let pwm_forward_pin = env::var("PWM_FORWARD_PIN").unwrap_or_else(|_| "18".to_string());
    let pwm_reverse_pin = env::var("PWM_REVERSE_PIN").unwrap_or_else(|_| "19".to_string());
    let button_pin = env::var("BUTTON_PIN").unwrap_or_else(|_| "2".to_string());
    let calibration_factor = env::var("CALIBRATION_FACTOR")
        .unwrap_or_else(|_| "1.0".to_string());
    let moving_average_window = env::var("MOVING_AVERAGE_WINDOW")
        .unwrap_or_else(|_| "5".to_string());
    let pwm_frequency = env::var("PWM_FREQUENCY").unwrap_or_else(|_| "1000.0".to_string());
    let forward_speed = env::var("FORWARD_SPEED").unwrap_or_else(|_| "0.8".to_string());
    let reverse_speed = env::var("REVERSE_SPEED").unwrap_or_else(|_| "0.6".to_string());
    
    println!("cargo:rustc-env=COMPILED_DT_PIN={}", dt_pin);
    println!("cargo:rustc-env=COMPILED_SCK_PIN={}", sck_pin);
    println!("cargo:rustc-env=COMPILED_PWM_FORWARD_PIN={}", pwm_forward_pin);
    println!("cargo:rustc-env=COMPILED_PWM_REVERSE_PIN={}", pwm_reverse_pin);
    println!("cargo:rustc-env=COMPILED_BUTTON_PIN={}", button_pin);
    println!("cargo:rustc-env=COMPILED_CALIBRATION_FACTOR={}", calibration_factor);
    println!("cargo:rustc-env=COMPILED_MOVING_AVERAGE_WINDOW={}", moving_average_window);
    println!("cargo:rustc-env=COMPILED_PWM_FREQUENCY={}", pwm_frequency);
    println!("cargo:rustc-env=COMPILED_FORWARD_SPEED={}", forward_speed);
    println!("cargo:rustc-env=COMPILED_REVERSE_SPEED={}", reverse_speed);
    
    // ビルド情報
    println!("cargo:rustc-env=BUILD_TIMESTAMP={}", chrono::Utc::now().to_rfc3339());
    println!("cargo:rustc-env=BUILD_TARGET={}", env::var("TARGET").unwrap_or_else(|_| "unknown".to_string()));
    
    // 再ビルドトリガー
    println!("cargo:rerun-if-env-changed=TARGET_WEIGHT");
    println!("cargo:rerun-if-env-changed=DT_PIN");
    println!("cargo:rerun-if-env-changed=SCK_PIN");
    println!("cargo:rerun-if-env-changed=PWM_FORWARD_PIN");
    println!("cargo:rerun-if-env-changed=PWM_REVERSE_PIN");
    println!("cargo:rerun-if-env-changed=BUTTON_PIN");
    println!("cargo:rerun-if-env-changed=CALIBRATION_FACTOR");
    println!("cargo:rerun-if-env-changed=MOVING_AVERAGE_WINDOW");
    println!("cargo:rerun-if-env-changed=PWM_FREQUENCY");
    println!("cargo:rerun-if-env-changed=FORWARD_SPEED");
    println!("cargo:rerun-if-env-changed=REVERSE_SPEED");
}