@echo off
REM Rabuka Reloaded - Android Package Creator (Windows)
REM Run this to create a transfer package for Android

set REPO_ROOT=%~dp0..
set PACKAGE_DIR=%REPO_ROOT%\android_package
set PACKAGE_NAME=rabuka_android_%DATE:~10,4%%DATE:~4,2%%DATE:~7,2%.tar.gz

echo Creating Android transfer package...

REM Clean previous package
if exist "%PACKAGE_DIR%" rmdir /s /q "%PACKAGE_DIR%"
mkdir "%PACKAGE_DIR%"

echo Copying engine source...
xcopy "%REPO_ROOT%\engine" "%PACKAGE_DIR%\engine\" /E /I /Q

echo Copying web_ui...
xcopy "%REPO_ROOT%\web_ui" "%PACKAGE_DIR%\web_ui\" /E /I /Q

echo Copying cards...
xcopy "%REPO_ROOT%\cards" "%PACKAGE_DIR%\cards\" /E /I /Q

echo Copying Android scripts...
copy "%REPO_ROOT%\android\setup_termux.sh" "%PACKAGE_DIR\"
copy "%REPO_ROOT%\android\start_android.sh" "%PACKAGE_DIR\"
copy "%REPO_ROOT%\android\README_ANDROID.md" "%PACKAGE_DIR\"

echo Creating transfer instructions...
cat > "%PACKAGE_DIR%\TRANSFER_INSTRUCTIONS.txt" << 'EOF'
TRANSFER TO ANDROID (Windows):

Option 1: ADB (USB Debugging)
1. Enable Developer Options -> USB Debugging on Android
2. Connect phone via USB
3. Run: adb push %PACKAGE_NAME% /sdcard/Download/
4. In Termux:
   cd /sdcard/Download
   tar -xzf %PACKAGE_NAME%
   cd rabuka_reloaded
   ./setup_termux.sh

Option 2: Cloud Storage
1. Upload %PACKAGE_NAME% to Google Drive / Dropbox / etc.
2. Download on Android (Files app)
3. In Termux:
   cd ~/storage/downloads
   tar -xzf %PACKAGE_NAME%
   cd rabuka_reloaded
   ./setup_termux.sh

Option 3: Local Network (Termux + Python)
1. In Termux: pkg install python
2. On PC: cd %PACKAGE_DIR% && python -m http.server 8000
3. In Termux: cd /sdcard/Download && wget http://<PC_IP>:8000/%PACKAGE_NAME%
4. tar -xzf %PACKAGE_NAME% && cd rabuka_reloaded && ./setup_termux.sh
EOF

echo.
echo ==========================================
echo Package folder ready: %PACKAGE_DIR%
echo ==========================================
echo.
echo To create tarball (requires tar/gzip - use WSL or Git Bash):
echo   tar -czf %PACKAGE_NAME% -C %PACKAGE_DIR% .
echo.
echo Then transfer %PACKAGE_NAME% to Android using one of the methods above.
pause