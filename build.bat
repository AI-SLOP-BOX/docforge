@echo off
REM DocForge Build Script for Windows

echo DocForge Build Script
echo ====================

REM Check if Node.js is installed
where node >nul 2>nul
if %ERRORLEVEL% neq 0 (
    echo Error: Node.js is not installed
    echo Please install Node.js from https://nodejs.org/
    pause
    exit /b 1
)

REM Check if Rust is installed
where rustc >nul 2>nul
if %ERRORLEVEL% neq 0 (
    echo Error: Rust is not installed
    echo Please install Rust from https://www.rust-lang.org/tools/install
    pause
    exit /b 1
)

REM Check if npm is installed
where npm >nul 2>nul
if %ERRORLEVEL% neq 0 (
    echo Error: npm is not installed
    echo Please install npm from https://nodejs.org/
    pause
    exit /b 1
)

REM Install Tauri CLI if not present
cargo tauri --version >nul 2>nul
if %ERRORLEVEL% neq 0 (
    echo Installing Tauri CLI...
    cargo install tauri-cli
)

REM Install npm dependencies
echo Installing npm dependencies...
call npm install

REM Build frontend
echo Building frontend...
call npm run build

REM Build Tauri app
echo Building Tauri app...
call npx tauri build

echo Build complete!
echo Check src-tauri\target\release\bundle\ for output files.
pause
