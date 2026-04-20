@echo off
chcp 65001 >nul
title Win脚本中心 - 开发环境安装

echo ========================================
echo   Win脚本中心 - 开发环境安装
echo ========================================
echo.

:: 检查 Rust
where cargo >nul 2>&1
if %errorlevel% neq 0 (
    echo [1/2] 安装 Rust...
    echo 这可能需要几分钟，请耐心等待...
    curl --proto "=https" --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
    if %errorlevel% neq 0 (
        echo 警告: curl 方式失败，尝试使用 winget...
        winget install Rustlang.Rustup --accept-source-agreements --accept-package-agreements
    )
    :: 刷新环境变量
    set PATH=%USERPROFILE%\.cargo\bin;%PATH%
)

:: 验证 Rust
where cargo >nul 2>&1
if %errorlevel% neq 0 (
    echo.
    echo [错误] Rust 安装失败！
    echo 请手动访问: https://rustup.rs
    pause
    exit /b 1
)

echo [2/2] 启动开发服务器...
echo.

:: 启动 Tauri 开发模式
cd /d "%~dp0"
cargo tauri dev
