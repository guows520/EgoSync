@echo off
taskkill /F /IM egosync.exe >nul 2>&1
timeout /t 2 /nobreak >nul
del /F /Q "%APPDATA%\com.egosync.app\egosync.db" >nul 2>&1
del /F /Q "%APPDATA%\com.egosync.app\conversations.db" >nul 2>&1
echo Cleaned. Remaining files:
dir "%APPDATA%\com.egosync.app\" 2>nul