@echo off
chcp 65001 >nul
REM ============================================================
REM  DeepSeek-LeanSpark Tauri 原生形态启动脚本
REM  双击运行：启动原生桌面窗口（无需浏览器）
REM  首次运行会自动安装 tauri-cli（约需 2-3 分钟编译）
REM ============================================================
cd /d "%~dp0DeepSeek-LeanSpark"

REM ---- 检查 .env 文件 ----
if not exist ".env" (
    echo [警告] 未找到 .env 文件！
    echo 请先复制 .env.example 为 .env 并填入 DEEPSEEK_API_KEY
    echo.
    pause
    exit /b 1
)

REM ---- 检查 tauri-cli 是否安装 ----
echo [检查] tauri-cli 是否已安装...
cargo tauri --version >nul 2>&1
if errorlevel 1 (
    echo [提示] tauri-cli 未安装，正在自动安装（约需 2-3 分钟）...
    echo.
    cargo install tauri-cli --version "^2"
    if errorlevel 1 (
        echo [错误] tauri-cli 安装失败，请手动运行：cargo install tauri-cli --version "^2"
        pause
        exit /b 1
    )
    echo [成功] tauri-cli 安装完成。
    echo.
)

REM ---- 检查前端依赖是否安装 ----
if not exist "frontend\node_modules" (
    echo [提示] 前端依赖未安装，正在安装...
    cd /d "%~dp0DeepSeek-LeanSpark\frontend"
    npm install
    cd /d "%~dp0DeepSeek-LeanSpark"
)

REM ---- 检查图标文件（生产打包需要，dev 模式可跳过）----
if not exist "src-tauri\icons\32x32.png" (
    echo [提示] Tauri 图标未生成，dev 模式可继续，生产打包需要先运行：
    echo         cargo tauri icon path\to\your-icon.png
    echo.
)

echo ============================================================
echo  DeepSeek-LeanSpark Tauri 原生形态启动中...
echo  会同时启动：前端 dev server + Rust 后端 + 原生窗口
echo  关闭原生窗口即退出应用
echo ============================================================
echo.

REM ---- 启动 Tauri dev（会自动启动前端 dev server + 后端 + 原生窗口）----
cargo tauri dev

REM ---- 退出后清理 ----
taskkill /fi "WindowTitle eq DeepSeek-LeanSpark Backend*" /f >nul 2>&1
