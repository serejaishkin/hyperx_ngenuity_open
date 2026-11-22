@echo off
chcp 65001 >nul
cd /d "%~dp0"
where cargo >nul 2>nul || (echo Rust/cargo not found. Install from https://rustup.rs & pause & exit /b 1)
echo === Building egui version (release) ===
cargo build --release --bin hyperx_ngenuity_open
if errorlevel 1 (echo. & echo BUILD FAILED & pause & exit /b 1)
if not exist dist mkdir dist
copy /y target\release\hyperx_ngenuity_open.exe dist\ >nul
echo.
echo Done: %~dp0dist\hyperx_ngenuity_open.exe
pause
