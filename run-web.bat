@echo off
chcp 65001 >nul
REM ============================================================
REM  DeepSeek-LeanSpark Web 形态启动脚本
REM  双击运行：启动后端（cargo run）+ 前端（npm run dev）
REM  访问地址：http://localhost:5173
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

echo ============================================================
echo  DeepSeek-LeanSpark Web 形态启动中...
echo  后端: http://localhost:3000
echo  前端: http://localhost:5173
echo  按 Ctrl+C 退出（会同时关闭后端和前端）
echo ============================================================
echo.

REM ---- 启动后端（新窗口）----
start "DeepSeek-LeanSpark Backend" cmd /k "cd /d %~dp0DeepSeek-LeanSpark && cargo run"

REM ---- 等待后端编译完成（给 3 秒缓冲）----
timeout /t 3 /nobreak >nul

REM ---- 启动前端（当前窗口，会自动打开浏览器）----
cd /d "%~dp0DeepSeek-LeanSpark\frontend"
echo [提示] 前端启动后会自动打开浏览器，如未打开请手动访问 http://localhost:5173
echo.
npm run dev

REM ---- 前端退出后关闭后端窗口 ----
taskkill /fi "WindowTitle eq DeepSeek-LeanSpark Backend*" /f >nul 2>&1
