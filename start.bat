@echo off
rem Double-click to run. Opens the test bench; for real play, open /tv on the TV instead.
cd /d "%~dp0"
start "" http://localhost:3000/test
node server.js
pause
