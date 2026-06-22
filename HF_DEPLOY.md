# Hugging Face Spaces Deployment Guide

## Prerequisites
- A HF access token with write permissions from https://huggingface.co/settings/tokens
- Git LFS installed (`git lfs version`)

## One-time setup

```bash
# Add HF remote (use access token as password)
git remote add space https://user:TOKEN@huggingface.co/spaces/trioskosmos/rabuka_reloaded.git
```

## New deployment (full push)

```bash
# 1. Untrack large files that exceed HF's 10MB limit
git rm --cached engine/cloudflared.exe
echo "cloudflared.exe" >> .gitignore

# 2. Create a clean clone (avoids pushing history with large files)
cd ..
git clone --depth 1 file:///"$(pwd)/rabuka_reloaded" rabuka_hf_temp

# 3. Initialize fresh repo with LFS tracking for binaries
cd rabuka_hf_temp
rm -rf .git
git init
git lfs install
git lfs track "*.webp" "*.png" "*.pdf" "*.exe"

# 4. Merge HF metadata into README
#    (README.md.hf contains the YAML front matter)
#    Prepend the YAML to README.md, then delete README.md.hf

# 5. Exclude images (can be added separately via LFS)
git add -A -- ":!web_ui/img/*"

# 6. Push to HF main branch
git commit -m "Initial HF deploy"
git remote add space https://user:TOKEN@huggingface.co/spaces/trioskosmos/rabuka_reloaded.git
git push space master:main --force

# 7. Clean up
cd ..
rm -rf rabuka_hf_temp
```

## Important notes

### Branch
HF Spaces uses `main` as the default branch. Push to `main` (not `master`).

### Dockerfile
The Dockerfile requires:
- `rust:latest-slim` (versions < 1.88 can't compile `actix-web` 4.13+)
- No frontend build stage (vanilla JS, no package.json/npm)
- File paths matching the Rust binary's expectations:
  - Frontend: `COPY web_ui/ /app/web_ui/` (binary reads `../web_ui`)
  - Decks: `COPY web_ui/decks/ /app/game/decks/` (binary reads `../game/decks/`)
  - Cards: `COPY cards/cards.json /app/cards/cards.json` (binary reads `../cards/cards.json`)

### Pitfalls encountered
| Issue | Fix |
|-------|-----|
| `engine/cloudflared.exe` (52 MB) rejected | Untrack with `git rm --cached`, exclude via `.gitignore` |
| Binary files rejected | Track with Git LFS via `.gitattributes` |
| History contains large files | Use clean `git init` (no history) for HF push, or LFS migrate |
| Wrong branch | Push to `main`, not `master` |
| Missing README metadata | Prepend YAML front matter to `README.md` |
| Missing `game/decks/` directory | Copy from `web_ui/decks/` instead |
| No `package.json` for frontend | Remove node build stage; copy raw `web_ui/` |
| Rust version too old | Use `rust:latest-slim` |

### To add card images (2730 WebP, 221 MB)
```bash
git add web_ui/img/
git commit -m "Add card images"
git push
```
(The `*.webp` LFS tracking is already configured in `.gitattributes`)
