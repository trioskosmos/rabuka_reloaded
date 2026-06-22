# Hugging Face Spaces Deployment Guide

## Overview

This repo is deployed at https://huggingface.co/spaces/trioskosmos/rabuka_reloaded

The HF Space runs a separate clean repo (no history) because the original GitHub repo's git history is 1.9 GB and contains `engine/cloudflared.exe` (52 MB) which exceeds HF's 10 MB per-file limit. HF's pre-receive hook scans ALL objects being pushed (not just the latest commit) and rejects pushes containing files > 10 MB in any historical commit.

## Why a temp clone?

```
Original repo (GitHub)       Temp clone (for HF push)
─────────────────────        ────────────────────────
.git = 1.9 GB               .git = fresh (no history)
Contains cloudflared.exe    No cloudflared.exe
in past commits             (file was untracked before cloning)
master branch               master → main (HF default)
Has all WebP images →       Has all WebP images on disk
```

The temp clone uses `git clone --depth 1` to get only the latest snapshot, then `rm -rf .git && git init` to create a fresh repo with zero history. This bypasses HF's history scan entirely.

## Prerequisites

- HF access token with write permissions from https://huggingface.co/settings/tokens
- Git LFS installed (`git lfs version`)

## Remote setup (one-time)

```bash
git remote add space https://user:TOKEN@huggingface.co/spaces/trioskosmos/rabuka_reloaded.git
```

## Full initial deploy

```bash
# 1. Untrack large files that exceed HF's 10MB limit
git rm --cached engine/cloudflared.exe
echo "cloudflared.exe" >> .gitignore

# 2. Create temp clone (fresh git, no history)
cd ..
git clone --depth 1 file:///"$(pwd)/rabuka_reloaded" rabuka_hf_temp
cd rabuka_hf_temp
rm -rf .git
git init
git lfs install
git lfs track "*.webp" "*.png" "*.pdf" "*.exe"

# 3. Add HF metadata to README
#    Prepend YAML front matter from README.md.hf into README.md, then delete README.md.hf

# 4. Add everything EXCEPT card images (add images separately to avoid timeouts)
git add -A -- ":!web_ui/img/*"

# 5. First deploy (code only)
git commit -m "Initial HF deploy (code)"
git remote add space https://user:TOKEN@huggingface.co/spaces/trioskosmos/rabuka_reloaded.git
git push space master:main --force

# 6. Add images in batches (2730 WebP files, ~221 MB total)
#    Split into 10 batches of ~273 files each
ls web_ui/img/cards_webp/*.webp | head -273 | tr '\n' '\0' | xargs -0 git add
git commit -m "Add card images batch 1/10"
git push space master:main
# ... repeat for remaining 9 batches ...

# 7. Clean up
cd ..
rm -rf rabuka_hf_temp
```

## Updating the HF Space (subsequent deploys)

Since the HF repo now exists with LFS configured, future updates are simpler. You can push directly from the original repo using the temp clone method:

```bash
# 1. Create temp clone
cd ..
git clone --depth 1 file:///"$(pwd)/rabuka_reloaded" rabuka_hf_temp
cd rabuka_hf_temp

# 2. Push to HF (the remote is already in the HF repo, not the temp clone)
git remote add space https://user:TOKEN@huggingface.co/spaces/trioskosmos/rabuka_reloaded.git
git fetch space main

# 3. Reset to match HF state, then apply changes
git reset space/main
git add -A
git commit -m "Update HF Space"
git push space HEAD:main --force
```

## Important notes

### Branch
HF Spaces uses `main` as the default branch. The temp clone's local branch is `master`, so you push with `git push space master:main`.

### LFS file types tracked
- `*.webp` — Card images (2736 files)
- `*.png` — UI icons (24 files)
- `*.pdf` — Rulebooks (2 files)
- `*.exe` — Binaries (should be avoided; cloudflared.exe is untracked)

### Dockerfile
The current Dockerfile:
- Uses `rust:slim-bookworm` (Rust 1.88+ required for `actix-web 4.13`)
- Has no frontend build stage (vanilla JS, no `package.json`)
- Copies `web_ui/` to `/app/web_ui/` (matches `fs::Files::new("/", "../web_ui")`)
- Copies `web_ui/decks/` to `/app/game/decks/` (matches `../game/decks/` in code)
- Copies `cards/cards.json` to `/app/cards/cards.json`

## Pitfalls encountered

| Issue | Fix |
|-------|------|
| `engine/cloudflared.exe` (52 MB) rejected | Untrack with `git rm --cached`, exclude via `.gitignore` |
| Binary files (images, PDFs) rejected | Track with Git LFS via `.gitattributes` |
| History contains large files | Use clean `git init` (no history) for HF push |
| Branch mismatch | Push to `main`, not `master` |
| Missing README metadata | Prepend YAML front matter to `README.md` |
| Missing `game/decks/` directory | Copy from `web_ui/decks/` instead |
| No `package.json` for frontend | Remove node build stage; copy raw `web_ui/` |
| Rust version too old | Use `rust:slim-bookworm` |
| `*.json` gitignore hides locale files | Remove `*.json` from `.gitignore`, or add exception |
| Unicode `＋` in filenames blocked by git | Rename to ASCII `+` (git on Windows blocks U+FF0B via `core.protectNTFS`) |
| Large push times out | Push images in batches of ~273 files |
