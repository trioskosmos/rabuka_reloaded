---
title: Rabuka Reloaded
emoji: 🃏
colorFrom: pink
colorTo: purple
sdk: docker
pinned: false
app_port: 7860
---

# Rabuka Reloaded

A browser-based card game engine for Love Live! School Idol Festival trading card game.

## Features

- **Sandbox mode** — single player vs AI  
- **PvP mode** — two players in the same room via room codes  
- Full card ability resolution engine written in Rust  
- Real-time game state via REST API

## How to Play

1. Open the app URL
2. Click **Create Room** → choose **PvP** or **Sandbox**
3. For PvP: share your 4-letter room code with your opponent, they click **Join Room**
4. Each player selects a deck and the game begins

## Tech Stack

- **Backend**: Rust + Actix-Web  
- **Frontend**: Vanilla JS + Vite  
- **Deployment**: Docker on Hugging Face Spaces
