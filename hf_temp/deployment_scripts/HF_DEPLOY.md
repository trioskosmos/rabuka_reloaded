# Hugging Face Spaces Deployment Guide

## Overview

This project is deployed to a Hugging Face Space using a Docker container. Because the original repository contains a large Git history and files exceeding HF's limits (e.g., `cloudflared.exe`), we use a "Clean Push" strategy.

**Space URL:** https://huggingface.co/spaces/trioskosmos/rabukasim

## Prerequisites

- **HF Access Token**: Create a token with **Write** permissions at [huggingface.co/settings/tokens](https://huggingface.co/settings/tokens).
- **Git LFS**: Must be installed on your local machine.
- **Python 3.10+**: Required for the consolidation scripts.

## The Deployment Workflow

### 1. Image Consolidation
To avoid upload timeouts and reduce asset count, we consolidate images. This script updates `web_ui/js/card_image_mapping.json` to include the correct paths (`img/cards_webp/`) and deletes unused files from the disk.

**Run the script:**
```bash
python deployment_scripts/consolidate_images.py
```

### 2. Uploading to Hugging Face
We use the Hugging Face CLI. To prevent files from being placed in the root folder of the Space, **you must specify both the local path and the destination path** in the `hf upload` command.

#### How to avoid redundant uploads
The `hf upload` command automatically calculates file hashes. If a file with the same content already exists on the Space, **it is skipped and not re-uploaded**. However, the "Checking" phase for thousands of files can trigger environment timeouts.

**Automated Way (Recommended):**
Run the PowerShell helper script. It uses a batching strategy for images to avoid timeouts while still benefiting from hash-skipping.
```powershell
.\deployment_scripts\deploy_hf.ps1
```

**Manual Way (CLI):**
```bash
# Install HF CLI
powershell -ExecutionPolicy ByPass -c "irm https://hf.co/cli/install.ps1 | iex"

# Upload core components [LocalPath] [RemotePath]
hf upload trioskosmos/rabukasim engine/ engine/ --repo-type=space --token YOUR_TOKEN
hf upload trioskosmos/rabukasim cards/ cards/ --repo-type=space --token YOUR_TOKEN
hf upload trioskosmos/rabukasim Dockerfile Dockerfile --repo-type=space --token YOUR_TOKEN
hf upload trioskosmos/rabukasim README.md README.md --repo-type=space --token YOUR_TOKEN
hf upload trioskosmos/rabukasim .gitattributes .gitattributes --repo-type=space --token YOUR_TOKEN

# Upload consolidated images to the CORRECT directory
hf upload trioskosmos/rabukasim web_ui/img/cards_webp/ web_ui/img/cards_webp/ --repo-type=space --token YOUR_TOKEN

# Upload remaining UI files
hf upload trioskosmos/rabukasim web_ui/ web_ui/ --repo-type=space --token YOUR_TOKEN
```

### 3. Finalizing Git State
Ensure the HF Space's `main` branch is perfectly synced with your local consolidated state.
```bash
git branch -m master main
git push space main --force
```

## Technical Configuration

### Port Handling
The Docker Space must listen on port **7860**.
- **Dockerfile**: Sets `ENV PORT=7860`.
- **Rust Engine**: Reads `PORT` env var in `engine/src/game/web_server.rs`.

### LFS Tracking
Tracked in `.gitattributes`: `*.webp`, `*.png`, `*.pdf`, `*.exe`.

### Docker Structure
The engine runs from `/app/engine`. Assets are at:
- `/app/web_ui` (Frontend)
- `/app/cards/cards.json` (Database)
- `/app/game/decks/` (Deck definitions)
The Rust binary uses relative paths (e.g., `../web_ui`) which resolve correctly from `/app/engine`.
