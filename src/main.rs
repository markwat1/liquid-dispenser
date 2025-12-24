mod controllers;
mod models;
mod errors;

use controllers::SystemController;
use models::SystemConfig;
use errors::SystemError;
use log::{info, error, warn};
use std::process;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use tokio::signal;

#[tokio::main]
async fn main() {
    // ログ初期化
    env_logger::init();
    
    info!("Weight Sensor Pump Controller starting up");
    
    // 設定を読み込み（優先順位付き）
    let config = SystemConfig::load_with_priority();
    info!("Loaded configuration: {:?}", config);
    
    // ビルド情報を表示
    let build_info = SystemConfig::build_info();
    build_info.display();
    
    // シャットダウンシグナル用のフラグ
    let shutdown_flag = Arc::new(AtomicBool::new(false));
    let shutdown_flag_clone = shutdown_flag.clone();
    
    // シグナルハンドラを設定
    tokio::spawn(async move {
        match signal::ctrl_c().await {
            Ok(()) => {
                info!("Received Ctrl+C, initiating shutdown");
                shutdown_flag_clone.store(true, Ordering::Relaxed);
            }
            Err(err) => {
                error!("Failed to listen for shutdown signal: {}", err);
            }
        }
    });
    
    // システムコントローラを初期化
    let mut system_controller = match SystemController::new(config) {
        Ok(controller) => controller,
        Err(e) => {
            error!("Failed to initialize system controller: {}", e);
            process::exit(1);
        }
    };
    
    // 起動シーケンスを実行
    if let Err(e) = system_controller.startup_sequence() {
        error!("Startup sequence failed: {}", e);
        process::exit(1);
    }
    
    info!("System initialized successfully, starting main loop");
    
    // メイン制御ループを非同期で実行
    let result = run_main_loop(&mut system_controller, shutdown_flag).await;
    
    // 終了処理
    info!("Shutting down system");
    if let Err(e) = system_controller.shutdown_sequence() {
        error!("Shutdown sequence failed: {}", e);
    }
    
    match result {
        Ok(()) => {
            info!("System shutdown completed successfully");
            process::exit(0);
        }
        Err(e) => {
            error!("System error: {}", e);
            process::exit(1);
        }
    }
}

/// メイン制御ループを実行
async fn run_main_loop(
    system_controller: &mut SystemController,
    shutdown_flag: Arc<AtomicBool>,
) -> Result<(), SystemError> {
    let mut cycle_count = 0u64;
    let mut last_stats_time = std::time::Instant::now();
    
    loop {
        // シャットダウンシグナルをチェック
        if shutdown_flag.load(Ordering::Relaxed) {
            info!("Shutdown signal received, exiting main loop");
            break;
        }
        
        // 1サイクルの処理を実行
        match system_controller.process_cycle() {
            Ok(should_continue) => {
                if !should_continue {
                    info!("System requested shutdown");
                    break;
                }
            }
            Err(e) => {
                error!("Error in main loop: {}", e);
                
                // 重要なエラーの場合は終了
                match e.level() {
                    errors::ErrorLevel::Critical => {
                        return Err(e);
                    }
                    _ => {
                        // その他のエラーは警告として処理を継続
                        warn!("Non-critical error, continuing: {}", e);
                    }
                }
            }
        }
        
        cycle_count += 1;
        
        // 定期的に統計情報を出力
        if last_stats_time.elapsed().as_secs() >= 60 {
            let stats = system_controller.statistics();
            info!("System stats (cycle {}): {:?}", cycle_count, stats);
            last_stats_time = std::time::Instant::now();
        }
        
        // 短時間待機
        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
    }
    
    Ok(())
}

/// システムコントローラの拡張実装
impl SystemController {
    /// 統計情報を定期的に出力
    #[allow(dead_code)]
    pub fn log_statistics(&self) {
        let stats = self.statistics();
        
        info!("=== System Statistics ===");
        info!("State: {}", stats.state.as_str());
        info!("Current Weight: {:.1}g", stats.current_weight.unwrap_or(0.0));
        info!("Target Weight: {:.1}g", stats.target_weight);
        info!("Error Count: {}", stats.error_count);
        info!("Button Presses: {}", stats.button_press_count);
        info!("Motor Runtime: {:?}", stats.motor_runtime);
        
        if let Some(uptime) = stats.uptime {
            info!("Session Uptime: {:?}", uptime);
        }
        
        info!("========================");
    }
    
    /// デバッグ情報を出力
    #[allow(dead_code)]
    pub fn debug_info(&self) {
        use log::debug;
        
        debug!("=== Debug Information ===");
        debug!("System State: {:?}", self.state());
        debug!("Config: {:?}", self.config());
        
        if let Some(reading) = self.last_weight_reading() {
            debug!("Last Weight: {:.1}g (stable: {})", reading.value, reading.is_stable);
        }
        
        debug!("Error Count: {}", self.error_count());
        debug!("========================");
    }
}