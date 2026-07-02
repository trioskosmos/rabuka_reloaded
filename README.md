---
title: Rabuka Reloaded
emoji: 🃏
colorFrom: blue
colorTo: purple
sdk: docker
pinned: false
---

# Rabuka Reloaded

A web-based simulator of a certain card game.

## Quick Start

### Windows

Run `start.bat` — it builds and launches the Rust backend, then opens the game UI at `http://127.0.0.1:8080`.

### Docker

```bash
docker build -t rabuka-reloaded .
docker run -p 8080:8080 rabuka-reloaded
```

### Manual

```bash
cd engine
cargo run --release --bin rabuka_engine web-server
```

Open `http://127.0.0.1:8080` in a browser.

## How to Play

1. Open the game UI, enter player names, and choose decks.
2. The game follows standard SIF card game rules:
   - Rock-Paper-Scissors for first attacker
   - Mulligan phase
   - Main phase: play members, energy, baton touch
   - Live performance phase: set live cards, yell, judge
3. Use the sidebar for actions, log, and card lookups.
4. **Hotseat**: Both players use the same screen — toggle perspective via the tab buttons ("My Board" / "Opponent") or use the mobile action bar.

## `start.bat` Explained

```batch
@echo off
setlocal enabledelayedexpansion

REM Adds a Windows Firewall rule for port 8080 (LAN play).
REM May prompt for admin approval (UAC).
powershell -NoProfile -Command "Start-Process netsh -Verb RunAs ..."

REM Builds and starts the Rust web server in the background.
cd /d "%~dp0engine"
start /b cargo run --release --bin rabuka_engine web-server

REM Polls the /api/game-state endpoint until the server is ready.
:wait_loop
powershell -Command "try { (Invoke-WebRequest -Uri http://127.0.0.1:8080/api/game-state ...) } catch { { exit 1 }"
if %errorlevel% neq 0 (
    timeout /t 2 /nobreak >nul
    goto wait_loop
)

REM Opens the game UI.
echo Game UI: http://127.0.0.1:8080
pause
```

For online play, pair with a tunnel like **cloudflared** or **ngrok**.

## Tech Stack

| Layer | Technology |
|-------|------------|
| Backend | Rust (actix-web, tokio, serde) |
| Frontend | Vanilla JS, HTML5, CSS3 |
| Build | Vite (web_ui), Cargo (engine) |
| Deployment | Docker / Hugging Face Spaces |

## Card Data

Card definitions are in `cards/cards.json` and abilities in `cards/abilities.json`. The raw images are hosted at the official Love Live! card game site and cached locally after first load.
