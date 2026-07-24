@echo off
REM ============================================================
REM  DeepSeek-LeanSpark Tauri Native Mode Launcher
REM  Starts native desktop window - no browser needed
REM  First run auto-installs tauri-cli ~2-3 min
REM ============================================================
setlocal

cd /d "%~dp0DeepSeek-LeanSpark"
if errorlevel 1 (
    echo [ERROR] Cannot enter project dir: %~dp0DeepSeek-LeanSpark
    pause
    exit /b 1
)

echo ============================================================
echo  DeepSeek-LeanSpark Tauri Native Mode
echo ============================================================

REM ---- Check .env file ----
if not exist ".env" (
    echo [ERROR] .env file not found!
    echo Please copy .env.example to .env and fill in DEEPSEEK_API_KEY
    echo.
    pause
    exit /b 1
)

REM ---- Check cargo ----
where cargo >nul 2>&1
if errorlevel 1 (
    echo [ERROR] cargo not found. Install Rust: https://rustup.rs
    pause
    exit /b 1
)

REM ---- Check npm ----
where npm >nul 2>&1
if errorlevel 1 (
    echo [ERROR] npm not found. Install Node.js: https://nodejs.org
    pause
    exit /b 1
)

REM ---- Install frontend deps if missing ----
if not exist "frontend\node_modules" (
    echo [INFO] Installing frontend dependencies...
    cd /d "%~dp0DeepSeek-LeanSpark\frontend"
    call npm install
    if errorlevel 1 (
        echo [ERROR] npm install failed
        pause
        exit /b 1
    )
    cd /d "%~dp0DeepSeek-LeanSpark"
)

REM ---- Check tauri-cli ----
echo [CHECK] Checking tauri-cli...
cargo tauri --version >nul 2>&1
if errorlevel 1 goto :install_tauri
echo [OK] tauri-cli already installed.
goto :run_tauri

:install_tauri
echo [INFO] tauri-cli not installed. Auto-installing - takes 2-3 min...
echo.
REM Install latest tauri-cli (currently 2.x). Version qualifier avoided due to bat escaping issues.
cargo install tauri-cli
if errorlevel 1 (
    echo [ERROR] tauri-cli install failed
    echo Please run manually: cargo install tauri-cli
    pause
    exit /b 1
)
echo [OK] tauri-cli installed successfully.
echo.

:run_tauri
echo.
echo Starting Tauri native window...
echo This starts: frontend dev server + Rust backend + native window
echo Close the native window to exit.
echo.

call cargo tauri dev

echo.
echo [INFO] Tauri app exited.
pause
endlocal
