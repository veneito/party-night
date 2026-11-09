@echo off
rem Double-click to run. Opens the test bench; for real play, open /tv on the TV instead.
cd /d "%~dp0src-tauri"
start "" http://localhost:3000/test
cargo run -- --server
pause
