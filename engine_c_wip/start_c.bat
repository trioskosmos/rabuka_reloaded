@echo off
setlocal
cd /d "%~dp0"

where make >nul 2>&1
if errorlevel 1 (
    echo [C] GNU make was not found in PATH.
    exit /b 1
)

if /I "%~1"=="tests" goto tests
if /I "%~1"=="replay" goto replay
if /I "%~1"=="smoke" goto smoke
if /I "%~1"=="interactive" goto interactive
if /I "%~1"=="web" goto web

:build
make -j4
if errorlevel 1 exit /b %errorlevel%

:interactive
echo [C] Starting the incomplete interactive C engine.
echo [C] Type -1 when asked for a choice or live card to skip it.
.\rb_engine.exe --interactive
exit /b %errorlevel%

:smoke
make -j4
if errorlevel 1 exit /b %errorlevel%
.\rb_engine.exe
exit /b %errorlevel%

:replay
make rb_engine_replay
if errorlevel 1 exit /b %errorlevel%
.\rb_engine_replay.exe
exit /b %errorlevel%

:web
make web
if errorlevel 1 exit /b %errorlevel%
echo [C] Starting web server at http://127.0.0.1:18080/
.\rb_web_server.exe
exit /b %errorlevel%

:tests
make -j4
if errorlevel 1 exit /b %errorlevel%
for %%T in (mechanics live-rules phase-rules performance-rules surplus-heart opponent-live-flow live-movement restrictions condition-eval target-selection max-distinct-names baton-touch member-activation b8-live-timing cross-player draw-phase-fix) do (
    echo [C] Running %%T...
    make %%T
    if errorlevel 1 exit /b %errorlevel%
)
echo [C] All C parity targets passed.
exit /b 0
