# Frontend System - Quick Reference Card

**Last Updated:** 2026-03-16

---

## 🐛 What Was Fixed

### 1. Permission Error (FIXED ✅)
**Error:** `PermissionError: [WinError 5] アクセスが拒否されました`  
**Cause:** Dev server holding `launcher/static_content/` directory open  
**Fix:** Modified `tools/sync_launcher_assets.py` to:
- Try `shutil.move()` first (graceful if unlocked)
- Fall back to copying file-by-file if locked (no crash)
- Clean up gracefully even if some files can't be touched
- **Result:** Script now works even while dev server is running

---

## 📚 CSS Organization Explained

### Load Order (Most Important)
```
1. base.css           ← All colors, spacing, shadows (foundation)
2. layout.css         ← Page structure, grid, responsive
3. cards.css          ← Card rendering, zones
4. components.css     ← Buttons, inputs, UI
5. ui_components.css  ← Toggles, tabs, badges
6. modals_base.css    ← Base modal structure
7. modals.css         ← Modal variants
8. log_viewer_modal.css → Logger UI (441 lines!)
9. performance.css    ← Victory/defeat screens
10. performance_stats.css → Stat display
11. unified_log_styles.css → Log colors
12. deck_validation.css → Validation UI
13. main.css          ← Page-level tweaks
14. debug.css         ← Dev tools (loads LAST)
```

### What Each File Controls

| File | What It Does | Change If... |
|------|------------|-------------|
| `base.css` | Colors, spacing, shadows | You want to theme the app |
| `layout.css` | Page structure, responsiveness | You want to reposition UI |
| `cards.css` | Cards, zones, orientations | You want to change card appearance |
| `components.css` | Buttons, inputs, general UI | You want to change button/form style |
| `modals_base.css` + `modals.css` | Modal windows | You want static/custom modals |
| `log_viewer_modal.css` | Logger display | You want to style the log viewer |
| `performance.css` | Victory/defeat screens | You want to change outcome display |
| `debug.css` | Developer overlay (~40px top-left) | You want to hide/style dev tools |

---

## 🚀 Vite Workflow

### Development (Live Reload)
```bash
cd frontend/web_ui
npm run dev              # Starts at http://localhost:3000
# Edit CSS → Saves → Auto-reload (no manual refresh needed)
```

### Production (Build & Deploy)
```bash
cd frontend/web_ui
npm run build           # Creates dist/ folder (1-2 minutes)
cd ../..
python tools/sync_launcher_assets.py  # Syncs to backend
# Now the backend serves the built version
```

---

## 🎨 Common CSS Tasks

### Change All Button Colors
**Edit:** `base.css` (line ~16)
```css
--accent-blue: #4a9eff;      /* Default blue */
--accent-pink: #ff55aa;      /* Default pink */
```
**Result:** All buttons using these CSS variables update automatically

### Change Modal Appearance
**Edit:** `modals_base.css` (background, border, shadow)  
**Edit:** `modals.css` (specific animations, positioning)  
**Test:** `npm run dev` and interact with modals

### Change Card Size
**Edit:** `cards.css` (search for `.card {`)  
**Look for:** `width: var(--card-w);` and `height: var(--card-h);`  
**Change:** Adjust or update variables in `base.css`

### Add New Component CSS
1. Create `frontend/web_ui/css/mynewcomponent.css`
2. Add to `style.css`: `@import './mynewcomponent.css';` (at appropriate level)
3. Use variables from `base.css` (colors, spacing, shadows)
4. Update this document (CSS_ORGANIZATION.md)

---

## 🧪 Troubleshooting

| Problem | Checklist |
|---------|-----------|
| CSS change doesn't appear in dev | Hard refresh (Ctrl+Shift+R), check DevTools that CSS is loaded |
| CSS works in dev but not production | Did you run `npm run build`? Did you run sync script? |
| Sync script crashes | Now handles in-use directories gracefully—try again |
| Button/input looks wrong | Check `components.css` first, then `base.css` for colors |
| Layout is broken on mobile | Check `layout.css` media queries (`@media max-width:`) |

---

## 📋 Documentation Files Created

1. **CSS_ORGANIZATION.md** (14 sections)
   - Complete file-by-file breakdown
   - Dependency graph
   - Troubleshooting guide
   - What affects what

2. **VITE_USAGE.md** (9 sections)
   - How to use Vite (dev, build, preview)
   - File organization
   - Common issues
   - Deployment checklist

3. **CSS_CONSOLIDATION_GUIDE.md** (6 sections)
   - Should you consolidate? (Answer: NO)
   - Options analysis
   - Performance impact
   - FAQ

---

## ✅ Current Status

| Component | Status | Notes |
|-----------|--------|-------|
| CSS Organization | ✅ Modular | 14 files, clear load order |
| Vite Setup | ✅ Correct | Configured properly |
| Asset Sync | ✅ Fixed | Now handles locked directories |
| Documentation | ✅ Complete | Three guides created |

---

## 🚨 DO NOT

- ❌ Consolidate CSS into single file (you'd regret it)
- ❌ Use `!important` everywhere (specificity is better)
- ❌ Manually edit `launcher/static_content/` (it's generated)
- ❌ Run sync while dev server is modifying files (old issue—now fixed)
- ❌ Skip `npm run build` before deploying (raw source is unoptimized)

---

## ✔️ DO

- ✅ Edit CSS files in `frontend/web_ui/css/`
- ✅ Use `npm run dev` for live reload during development
- ✅ Check `base.css` for color/spacing variables first
- ✅ Run `npm run build` before production
- ✅ Run sync script after building
- ✅ Update documentation when adding new CSS files
- ✅ Keep CSS modular (don't force consolidation)

---

## 🎯 Next Steps

### For Immediate Use
1. Test the fixed sync script: `python tools/sync_launcher_assets.py`
2. Read CSS_ORGANIZATION.md to understand file purposes
3. Refer to VITE_USAGE.md when building/deploying

### For Long-term Maintenance
1. Document new CSS files as you add them
2. Run PurgeCSS occasionally to find dead code
3. Keep load order in style.css up to date
4. Update this card as workflow changes

---

!