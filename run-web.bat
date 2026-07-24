@echo off
REM ============================================================
REM  DeepSeek-LeanSpark Web Mode Launcher
REM  Starts backend + frontend, auto-opens browser
REM  URL: http://localhost:5173
REM ============================================================
setlocal

cd /d "%~dp0DeepSeek-LeanSpark"
if errorlevel 1 (
    echo [ERROR] Cannot enter project dir: %~dp0DeepSeek-LeanSpark
    pause
    exit /b 1
)

echo ============================================================
echo  DeepSeek-LeanSpark Web Mode
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

echo.
echo [1/2] Starting backend in new window...
start "DeepSeek-LeanSpark Backend" cmd /k "cd /d %~dp0DeepSeek-LeanSpark && cargo run"

echo [2/2] Starting frontend + opening browser...
echo       URL: http://localhost:5173
echo       Close this window to stop frontend.
echo.

cd /d "%~dp0DeepSeek-LeanSpark\frontend"

REM --open auto-opens browser when vite is ready
call npm run dev -- --open

echo.
echo [INFO] Frontend exited. Close backend window manually.
pause
endlocal
