@echo off
chcp 65001 >nul
echo ========================================
echo         应用程序启动器
echo ========================================
echo.

:: 在这里修改你要启动的程序路径
set "APP_PATH=C:\Users\lms\AppData\Local\youdao\dict\Application\YoudaoDict.exe"

:: 如果程序路径包含空格，用引号包裹
:: set "APP_PATH=C:\Program Files (x86)\Microsoft VS Code\Code.exe"

:: 检查程序是否存在
if not exist "%APP_PATH%" (
    echo [错误] 程序不存在: %APP_PATH%
    echo.
    echo 请修改脚本中的 APP_PATH 变量！
    pause
    exit /b 1
)

echo [信息] 正在启动: %APP_PATH%
echo.

:: 启动程序（用 start 可以让脚本立即返回）
start "" "%APP_PATH%"

echo [成功] 程序已启动！
timeout /t 2 /nobreak >nul
