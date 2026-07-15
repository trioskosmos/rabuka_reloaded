@echo off
REM Self-play training loop: generate data → train → test → repeat
setlocal enabledelayedexpansion

set ENGINEDIR=engine
set DATAFILE=training_data.bin
set WEIGHTS=card_weights.bin
set BESTWEIGHTS=card_weights_best.bin
set ITERATIONS=%1
if "%ITERATIONS%"=="" set ITERATIONS=5
set GAMES=%2
if "%GAMES%"=="" set GAMES=500

REM Initial test with random weights
echo === Iteration 0: baseline vs random ===
cd %ENGINEDIR%
cargo run --release --bin bot_demo -- ..\%WEIGHTS% 2>&1 | findstr "games —"
cd ..

for /L %%i in (1,1,%ITERATIONS%) do (
    echo.
    echo === Iteration %%i ===

    REM Step 1: Generate training data with current bot
    echo --- Generating %GAMES% games with bot... ---
    cd %ENGINEDIR%
    cargo run --release --bin bot_data_gen -- %GAMES% ..\%DATAFILE% ..\%WEIGHTS% 2>&1
    cd ..

    REM Step 2: Train neural network on GPU
    echo --- Training on GPU... ---
    python train_nn.py %DATAFILE% 15 %WEIGHTS%

    REM Step 3: Test vs random
    echo --- Testing vs random (20 games)... ---
    cd %ENGINEDIR%
    for /f "tokens=*" %%r in ('cargo run --release --bin bot_demo -- ..\%WEIGHTS% 2^>^&1 ^| findstr "games —"') do set RESULT=%%r
    echo !RESULT!
    cd ..

    REM Save best weights
    if not exist %BESTWEIGHTS% (
        copy %WEIGHTS% %BESTWEIGHTS% >nul
    )
)

echo.
echo === Done ===
echo Best weights: %BESTWEIGHTS%
echo Final weights: %WEIGHTS%
