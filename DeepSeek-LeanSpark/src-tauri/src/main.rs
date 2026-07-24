// DeepSeek-LeanSpark 桌面壳入口（Tauri 2.x）。
//
// 启动流程：
// 1. 在 Tauri 主进程内启动 axum HTTP 后端（端口 3000 或自定义），
//    前端 Webview 通过 http://localhost:3000/api/* 调用后端。
// 2. 打开 Tauri 主窗口加载前端 dist/index.html。
// 3. 注册对话框、文件系统、shell 插件，供前端通过 invoke 使用。
//
// 这样既保留 Web 形态（开发期 `npm run dev` + `cargo run`），
// 又能在生产期打包为原生桌面应用（`cargo tauri build`）。
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::sync::Arc;
use tauri::Manager;

fn main() {
    // 初始化日志（与主 crate 共用 EnvFilter）
    let _ = tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .try_init();

    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_fs::init())
        .plugin(tauri_plugin_shell::init())
        .setup(|app| {
            // 在 Tauri 主线程上启动后端：用 tokio runtime 承载 axum
            // （主 crate 的 run() 是异步函数，这里用 tokio::spawn 驱动）
            let runtime = Arc::new(tokio::runtime::Runtime::new()?);
            let runtime_clone = runtime.clone();

            // 后端监听地址：默认 127.0.0.1:3000（仅本机访问，避免外网暴露）
            // 用户可通过环境变量 LISTEN_ADDR 覆盖
            std::env::set_var(
                "LISTEN_ADDR",
                std::env::var("LISTEN_ADDR").unwrap_or_else(|_| "127.0.0.1:3000".to_string()),
            );

            runtime_clone.spawn(async move {
                if let Err(e) = deepseek_leanspark::run().await {
                    tracing::error!("后端退出：{e}");
                }
            });

            // 把 runtime 存进 Tauri state，避免被销毁
            app.manage(runtime);

            tracing::info!("Tauri 桌面壳已启动，后端监听 127.0.0.1:3000");
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("Tauri 应用启动失败");
}
