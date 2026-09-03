@echo off
setlocal enabledelayedexpansion

echo === Rabuka 3DS Build Script ===
echo.

if not exist "C:\devkitPro\devkitARM\bin\arm-none-eabi-gcc.exe" (
    echo [FAIL] devkitPro not found at C:\devkitPro
    echo Install devkitPro with devkitARM + libctru from:
    echo   https://github.com/devkitPro/installer/releases
    pause
    exit /b 1
)
echo [1/7] devkitPro found

REM Fix link.exe shadowing: devkitPro msys2 'link' (hardlink util) shadows MSVC's
REM link.exe, breaking cargo's host build-script link step. Strip msys2 bins first.
set "PATH=%PATH:C:\devkitPro\msys2\usr\bin;=%"
set "PATH=%PATH:C:\devkitPro\msys2\mingw64\bin;=%"
set "PATH=%PATH:C:\devkitPro\msys2\mingw32\bin;=%"
set "PATH=%PATH:C:/devkitPro/msys2/usr/bin;=%"
set DEVKITPRO=C:\devkitPro
set DEVKITARM=%DEVKITPRO%\devkitARM
set "PATH=%DEVKITARM%\bin;%DEVKITPRO%\tools\bin;%PATH%"

echo [2/7] Checking Rust nightly (pinned to nightly-2025-08-15 for 3DS)...
REM Use where with full path to avoid msys2 link.exe shadowing
where "%SystemRoot%\System32\link.exe" >nul 2>&1
if %errorlevel% neq 0 (
  echo [2/7] MSVC link.exe not found, using GNU host toolchain
  rustup toolchain list 2>nul | findstr /c:"nightly-2025-08-15-x86_64-pc-windows-gnu" >nul
  if %errorlevel% neq 0 ( rustup toolchain install nightly-2025-08-15-x86_64-pc-windows-gnu --no-self-update )
  rustup component add rust-src --toolchain nightly-2025-08-15-x86_64-pc-windows-gnu 2>nul
  set "CARGO_3DS_TOOLCHAIN=+nightly-2025-08-15-x86_64-pc-windows-gnu"
  set "USE_GNU_TOOLCHAIN=1"
) else (
  rustup toolchain list 2>nul | findstr /c:"nightly-2025-08-15" >nul
  if %errorlevel% neq 0 ( rustup toolchain install nightly-2025-08-15 )
  rustup component add rust-src --toolchain nightly-2025-08-15-x86_64-pc-windows-msvc 2>nul
  set "CARGO_3DS_TOOLCHAIN=+nightly-2025-08-15-x86_64-pc-windows-msvc"
  set "USE_GNU_TOOLCHAIN=0"
)
echo [2/7] Rust nightly-2025-08-15 + rust-src ready [!CARGO_3DS_TOOLCHAIN!]

echo [3/7] Checking cargo-3ds...
cargo 3ds --version >nul 2>&1
if %errorlevel% neq 0 ( cargo install cargo-3ds )
echo [3/7] cargo-3ds ready

echo [4/7] Pre-baking card data (unified bake tool)...
if not exist "%~dp0romfs" mkdir "%~dp0romfs"
if not exist "%~dp0romfs\decks" mkdir "%~dp0romfs\decks"
call :need_bake "%~dp0romfs\cards.bin"
if errorlevel 1 (
    echo [4/7] Baking cards.bin...
    cd /d "%~dp0..\..\tools\bake"
    if "!USE_GNU_TOOLCHAIN!"=="1" (
        set "PATH=%PATH%;C:\devkitPro\msys2\usr\bin"
    )
    cargo %CARGO_3DS_TOOLCHAIN% run --release -- 3ds "%~dp0romfs"
    if "!USE_GNU_TOOLCHAIN!"=="1" (
        REM Restore PATH (strip msys2 again)
        set "PATH=%PATH:C:\devkitPro\msys2\usr\bin;=%"
    )
    if !errorlevel! neq 0 (
        echo [FAIL] bake failed.
        pause
        exit /b 1
    )
    cd /d "%~dp0..\.."
) else (
    echo [4/7] cards.bin is up to date - skipping bake
)
echo [4/7] cards.bin + abilities.json ready

copy /Y "%~dp0..\..\web_ui\decks\*.txt" "%~dp0romfs\decks\" >nul

rem Single source of truth: ability translations live in web_ui; the 3DS romfs
rem copies them at build time (ability_en.json == ability_translations.json).
if not exist "%~dp0romfs\locales" mkdir "%~dp0romfs\locales"
copy /Y "%~dp0..\..\web_ui\js\i18n\ability_translations.json" "%~dp0romfs\locales\ability_en.json" >nul

echo [4/7] Card images (incremental atlas build)...
if exist "%~dp0..\..\web_ui\img\cards_webp\*.webp" (
    cd /d "%~dp0"
    call :need_images
    if errorlevel 1 (
        REM Find python with Pillow
        set "PY_CMD="
        where py >nul 2>&1 && py -c "import PIL" >nul 2>&1 && set "PY_CMD=py"
        if not defined PY_CMD (
            where python3 >nul 2>&1 && python3 -c "import PIL" >nul 2>&1 && set "PY_CMD=python3"
        )
        if defined PY_CMD (
            %PY_CMD% scripts/convert_cards.py
        ) else (
            echo [WARN] Python with Pillow not found - skipping card image conversion
        )
    ) else (
        echo [4/7] Card images are up to date - skipping conversion
    )
) else (
    echo [WARN] No card webp images found - skipping card image conversion
)

echo [4/7] Japanese font (subset, auto-rebuild on new chars)...
cd /d "%~dp0"
set "PY_CMD="
where py >nul 2>&1 && set "PY_CMD=py"
where python3 >nul 2>&1 && if not defined PY_CMD set "PY_CMD=python3"
if defined PY_CMD (
    %PY_CMD% scripts\build_font.py
) else (
    echo [WARN] py not found - skipping font rebuild
)

echo [5/7] Building 3DS binary...
if exist "%~dp0target" rmdir /s /q "%~dp0target"
if exist "C:\rust_targets\armv6k-nintendo-3ds\release\rabuka_3ds.3dsx" del "C:\rust_targets\armv6k-nintendo-3ds\release\rabuka_3ds.3dsx"
cd /d "%~dp0"
set RUSTFLAGS=
set CARGO_PROFILE_RELEASE_LTO=false
set CARGO_PROFILE_RELEASE_CODEGEN_UNITS=16
cargo %CARGO_3DS_TOOLCHAIN% 3ds build --bin rabuka_3ds --release --features 3ds
if %errorlevel% neq 0 (
    echo Build FAILED.
    pause
    exit /b 1
)
echo [5/7] Build succeeded

if not exist "%~dp0output" mkdir "%~dp0output"
copy /Y "C:\rust_targets\armv6k-nintendo-3ds\release\rabuka_3ds.3dsx" "%~dp0output\rabuka_3ds.3dsx" >nul
echo [5/7] 3DSX: output\rabuka_3ds.3dsx

echo [6/7] Creating CIA (requires makerom from devkitPro)...
set "SRC_ELF=C:\rust_targets\armv6k-nintendo-3ds\release\rabuka_3ds.elf"
set "SRC_SMDH=C:\rust_targets\armv6k-nintendo-3ds\release\rabuka_3ds.smdh"
set "OUT_CIA=%~dp0output\rabuka_3ds.cia"

if not exist "%DEVKITPRO%\tools\bin\makerom.exe" goto :skip_cia
if not exist "%SRC_ELF%" goto :skip_cia
echo [6/7] Building RomFS binary...
set "ROMFS_BIN=%~dp0output\_romfs.bin"
set "RSF_FILE=%~dp0rabuka.rsf"
"%DEVKITPRO%\tools\bin\build_romfs.exe" "%~dp0romfs" "%ROMFS_BIN%" >nul 2>&1
if errorlevel 1 echo [6/7] build_romfs failed -- trying makerom without it.
if exist "%ROMFS_BIN%" (
    makerom -f cia -o "%OUT_CIA%" -elf "%SRC_ELF%" -romfs "%ROMFS_BIN%" -rsf "%RSF_FILE%"
) else (
    makerom -f cia -o "%OUT_CIA%" -elf "%SRC_ELF%" -rsf "%RSF_FILE%"
)
if errorlevel 1 (
    echo [6/7] CIA creation failed. Use 3DSX instead.
) else (
    echo [6/7] CIA: output\rabuka_3ds.cia (install via FBI)
)
if exist "%ROMFS_BIN%" del "%ROMFS_BIN%" >nul 2>&1
goto :cia_done

:skip_cia
echo [6/7] makerom/ELF not found -- skipping CIA.
echo        3DSX file is ready for use via 3dslink or SD card.

:cia_done

echo.
echo === Build Complete ===
echo   3DSX: output\rabuka_3ds.3dsx (run via 3dslink / SD card)
echo   CIA:  see above (install via FBI on 3DS)
echo.
pause
exit /b 0

:need_bake
rem Returns errorlevel 0 if cards.bin is up to date (no rebuild needed),
rem 1 if any source (cards.json / abilities.json / deck txt) is newer.
set "DST=%~1"
if not exist "%DST%" exit /b 1
powershell -NoProfile -Command "$src=@('%~dp0..\..\cards\cards.json','%~dp0..\..\cards\abilities.json');$d='%~dp0..\..\web_ui\decks';if(Test-Path $d){$src+=(Get-ChildItem $d -Filter '*.txt' | ForEach-Object { $_.FullName })};$dst='%~1';if(-not (Test-Path $dst)){exit 1};$max=[datetime]::MinValue;$any=$false;foreach($s in $src){if(Test-Path $s){$t=(Get-Item $s).LastWriteTime;if($t -gt $max){$max=$t};$any=$true}};if(-not $any){exit 1};if($max -gt (Get-Item $dst).LastWriteTime){exit 1}else{exit 0}"
if errorlevel 1 ( exit /b 1 ) else ( exit /b 0 )

:need_images
rem Returns errorlevel 0 if atlases are up to date (no rebuild needed),
rem 1 if the manifest is missing/incomplete, the resolution/format changed, or
rem any webp is newer than the newest atlas. "Incomplete" means the manifest has
rem fewer card entries than there are webp sources (guards against a previous
rem partial build being wrongly skipped).
if "%RABUKA_CARD_RES%"=="" set "RABUKA_CARD_RES=192"
if "%RABUKA_TEX_FMT%"=="" set "RABUKA_TEX_FMT=auto-etc1"
powershell -NoProfile -Command "$m='%~dp0romfs\cards_manifest.json';$a=Get-ChildItem '%~dp0romfs\cards\cards_*.t3x' -ErrorAction SilentlyContinue;if(-not (Test-Path $m) -or -not $a){exit 1};$r='%~dp0romfs\cards_res.txt';$tr=$env:RABUKA_CARD_RES;$sr='';if(Test-Path $r){$sr=(Get-Content $r -Raw).Trim()};if($sr -ne $tr){exit 1};$f='%~dp0romfs\cards_fmt.txt';$tf=$env:RABUKA_TEX_FMT;$sf='';if(Test-Path $f){$sf=(Get-Content $f -Raw).Trim()};if($sf -ne $tf){exit 1};$w=Get-ChildItem '%~dp0..\..\web_ui\img\cards_webp\*.webp' -ErrorAction SilentlyContinue;if(-not $w){exit 1};$mc=([regex]::Matches((Get-Content $m -Raw),'atlas')).Count;if($mc -lt $w.Count){exit 1};$wMax=($w | ForEach-Object { $_.LastWriteTime } | Measure-Object -Maximum).Maximum;$aMax=($a | ForEach-Object { $_.LastWriteTime } | Measure-Object -Maximum).Maximum;if($wMax -gt $aMax){exit 1}else{exit 0}"
if errorlevel 1 ( exit /b 1 ) else ( exit /b 0 )