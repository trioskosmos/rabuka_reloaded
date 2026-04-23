# Frontend System Architecture & Visual Flows

---

## 1. CSS Load Order & Dependency Flow

```
┌────────────────────────────────────────────────────────────────┐
│                    HTML Pages (5 entry points)                │
│  index.html, deck_builder.html, deck_converter.html, ...     │
└────────────────────┬───────────────────────────────────────────┘
                     │
                     │ <link rel="stylesheet" href="css/style.css">
                     ↓
          ┌──────────────────────┐
          │   style.css          │
          │  (Master Import)     │
          └────────────┬─────────┘
                       │
        ┌──────────────┴──────────────┐
        │   CSS Import Chain          │
        ├──────────────┬──────────────┤
        │              │              │
    Phase 1         Phase 2       Phase 3
    Foundation    Components     Features
        │              │              │
    (Lines 4-5)    (Lines 8-12)  (Lines 15-19)
        │              │              │
        ▼              ▼              ▼
   ┌─────────┐   ┌──────────────┐  ┌────────────────┐
   │base.css │   │cards.css     │  │log_viewer_...  │
   │layout.  │   │components.css│  │performance.css │
   │css      │   │modals_base.  │  │debug.css       │
   │         │   │modals.css    │  │...etc...       │
   └─────────┘   │ui_components.│  └────────────────┘
                 └──────────────┘
                       │
        ╔══════════════╩══════════════╗
        ║   BROWSER INTERPRETER      ║
        ║  (CSS Cascade & Specificity)║
        ║  Later files override earlier║
        ╚══════════════╦══════════════╝
                       │
              ┌────────▼────────┐
              │  DOM Styled UI  │
              └─────────────────┘
```

---

## 2. What Each CSS File Affects

```
HIERARCHY OF CONTROL:

base.css (CSS Variables - FOUNDATION)
  │
  ├─→ All color tokens (--accent-pink, --bg-primary, etc.)
  ├─→ All spacing (--space-xs to --space-xxl)
  ├─→ All shadows, borders, transitions
  └─→ Z-index scale for layering
      ↓
      Affects EVERYTHING below ▼

layout.css (Structure & Grid)
  │
  ├─→ .header-utilities
  ├─→ .game-board
  ├─→ .sidebar / .mobile-sidebar
  ├─→ .action-bar
  └─→ Media queries for responsive design
      ↓
      Affects: Cards, Components ▼

cards.css (Game Board - Cards & Zones)
  │
  ├─→ .card {transform, scale, hover}
  ├─→ .card.type-live (rotated)
  ├─→ .zone-deck, .zone-hand, .zone-field
  ├─→ .zone-bg (background colors)
  └─→ Zone highlighting on interaction
      ↓
      Only affects: Card display ▼

components.css (Buttons, Inputs, General UI)
  │
  ├─→ .btn (all buttons)
  ├─→ .btn-primary, .btn-secondary (variants)
  ├─→ .input-field, .select-dropdown
  ├─→ .badge, .tag
  └─→ Form interactions
      ↓
      Only affects: UI components ▼

ui_components.css (Utilities)
  │
  ├─→ Toggles, tabs, spinners
  ├─→ Additional badges/status indicators
  └─→ Less common components
      ↓
      Rarely affects: Specialized UI ▼

modals_base.css (Shared Modal Structure)
  │
  ├─→ .modal-overlay (full-screen background)
  ├─→ .modal-content (box shadow, border)
  ├─→ .modal-header, .modal-body, .modal-footer
  └─→ Base positioning & sizing
      ↓
      MUST load before specific modals ▼

modals.css (Modal Variants)
  │
  ├─→ .modal.lobby-setup
  ├─→ .modal.game-setup
  ├─→ .modal.victory-modal
  └─→ Modal-specific animations
      ↓
      Overrides: modals_base.css ▼

log_viewer_modal.css (Logger)
  │
  ├─→ #log-viewer-modal (logger container)
  ├─→ .log-entry {styling}
  ├─→ .log-entry.action/.error/.info (types)
  ├─→ .log-timestamp, .log-text, .log-metadata
  └─→ Largest feature-specific CSS (441 lines)
      ↓
      Independent: Only for logger ▼

performance.css (Stats & Outcomes)
  │
  ├─→ .perf-overview-shell (container)
  ├─→ .perf-player-grid (2-column layout)
  ├─→ .perf-panel {styling}
  ├─→ .perf-panel.success / .failure (colors)
  └─→ Performance metrics display
      ↓
      Used for: Victory/Defeat screens ▼

performance_stats.css (Detailed Stats)
  │
  ├─→ .perf-stat-cell (single stat box)
  ├─→ .stat-label, .stat-value (formatting)
  ├─→ .stat-bar (progress indicator)
  └─→ Stat icons
      ↓
      Child of: performance.css ▼

unified_log_styles.css (Log Text Colors)
  │
  ├─→ .log-action-* {text colors}
  ├─→ .log-player-name {highlighting}
  ├─→ .log-card-name {styling}
  ├─→ .log-error-text, .log-highlight
  └─→ Text color coding (not layout)
      ↓
      Cross-referenced: log_viewer_modal.css ▼

deck_validation.css (Validation UI)
  │
  ├─→ .validation-check {styling}
  ├─→ .validation-check.pass/.fail (states)
  ├─→ .validation-banner (summary)
  ├─→ .validation-error-msg
  └─→ .deck-count-indicator
      ↓
      Isolated: Only deck builder pages ▼

main.css (Application Overrides)
  │
  ├─→ Page-specific CSS tweaks
  ├─→ Context-specific adjustments
  ├─→ Not used for major features
  └─→ Acts as catch-all
      ↓
      Applies AFTER: All features ▼

debug.css (Dev Tools - LAST)
  │
  ├─→ #debug-overlay {positioning & visibility}
  ├─→ .debug-error-panel {styling}
  ├─→ .floating-tooltip {dev tooltips}
  └─→ Developer tools only
      ↓
      LOADS LAST: Can override anything
      (for debugging purposes)
```

---

## 3. Asset Pipeline: Development vs. Production

```
DEVELOPMENT FLOW (npm run dev):
═════════════════════════════════════════════════════════════════

frontend/web_ui/
  ├─ css/
  │   ├─ base.css (137 lines)
  │   ├─ layout.css (315 lines)
  │   ├─ cards.css (489 lines)
  │   └─ ... [12 more files]
  │
  ├─ js/
  │   ├─ main.js
  │   ├─ modals.js
  │   └─ compat.js (global namespace)
  │
  └─ *.html (5 entry points)
      │
      ↓ npm run dev (Vite dev server @ localhost:3000)
      │
      ☐─ Files served DIRECTLY (not bundled)
      ☐─ CSS changes = instant refresh (HMR)
      ☐─ JS changes = refresh with hot reload
      ☐─ Assets cached in memory
      │
      ↓ Browser
      │
    [Live, unoptimized version]


PRODUCTION FLOW (npm run build → python sync):
═════════════════════════════════════════════════════════════════

frontend/web_ui/
  ├─ css/ + js/ + img/ + *.html
      │
      ↓ npm run build (Vite bundler)
      │
   ╔═ dist/ (Generated)
   ║  ├─ index.html (bundled CSS/JS inlined)
   ║  ├─ deck_builder.html
   ║  ├─ css/
   ║  │   └─ [minified CSS bundles]
   ║  ├─ js/
   ║  │   └─ [minified JS bundles]
   ║  └─ img/ (optimized, WebP format)
   ║
   ║  ※ CSS files COMBINED into single minified output
   ║  ※ JS modules BUNDLED together
   ║  ※ All imports resolved statically
   ║
      ↓ python tools/sync_launcher_assets.py
      │
   launcher/.static_content_staging/
      │ [Staging - temporary]
      │
      ├─ Atomic Flip (now Windows-safe)
      │ [Handles locked directories gracefully]
      │
   launcher/static_content/
      │ [Final - served by backend]
      │
      ↓ Backend Server (http://backend:5000)
      │
      ↓ Browser
      │
    [Optimized, minified, fast]
```

---

## 4. Windows Permission Error Fix

```
OLD APPROACH (Failed):
═════════════════════════════════════════════════════════════════

os.rename(staging_dir, final_dir)  ← PermissionError!
   │
   └─ Fails if final_dir is locked by dev server
      (Windows: can't rename locked directories)

When error occurred:
  • Dev server holding file handle open
  • Preventing atomic rename
  • Script crashed → Sync incomplete


NEW APPROACH (Graceful):
═════════════════════════════════════════════════════════════════

try:
    shutil.move(staging_dir, final_dir)  ← Attempt clean move
    │
    └─ Works if unlocked ✅
       │
       └─→ Done! New assets in place

except PermissionError:  ← Directory still in use
    │
    ├─ Detected: target is locked
    │
    ├─ Fallback: Copy each file individually
    │
    ├─ Result: New content synced into existing dir ✅
    │
    └─ Dev server continues running (no interrupt)
       │
       └─→ Done! Assets updated without crash


OUTCOME:
═════════════════════════════════════════════════════════════════

Before: ❌ Script crashes if dev server running
After:  ✅ Script works even with dev server active
        ✅ Assets sync correctly in both cases
        ✅ No user action needed (automatic fallback)
```

---

## 5. CSS to Features Mapping

```
FEATURE COVERAGE:

Game Board Display
  ├─ layout.css (structure, grid)
  ├─ cards.css (card rendering, zones)
  └─ base.css (colors, spacing)

Header & Navigation
  ├─ layout.css (header utilities)
  ├─ components.css (buttons, icons)
  └─ base.css (coloring)

Action Buttons & Sidebar
  ├─ layout.css (positioning)
  ├─ components.css (button styling)
  └─ base.css (colors)

Modal System (Lobby, Setup, etc.)
  ├─ modals_base.css (structure)
  ├─ modals.css (variants)
  ├─ components.css (buttons inside modals)
  └─ base.css (overlay color)

Logger / Debug Console
  ├─ log_viewer_modal.css (container)
  ├─ unified_log_styles.css (text colors)
  └─ modals_base.css (if shown in modal)

Performance / Stats Display
  ├─ performance.css (layout)
  ├─ performance_stats.css (stat cells)
  ├─ modals.css (if in modal)
  └─ base.css (colors)

Deck Validation Messages
  ├─ deck_validation.css (UI)
  ├─ components.css (buttons)
  └─ base.css (error color)

Developer Tools
  ├─ debug.css (overlay, positioning)
  └─ base.css (font, colors)
```

---

## 6. When to Edit Each File

```
EDIT THIS              IF YOU NEED TO CHANGE
─────────────────     ─────────────────────────────────────────────
base.css              • App color scheme / theme
                      • Spacing/size scale
                      • Typography or shadows
                      • Z-index layering strategy

layout.css            • Page structure or grid
                      • Header/sidebar positioning
                      • Responsive breakpoints (mobile/desktop)
                      • Main content area sizing

cards.css             • Card size or appearance
                      • Card hover/animation effects
                      • Zone background colors
                      • Card rotation/orientation

components.css        • Button/input styling
                      • Form appearance
                      • General UI consistency
                      • Button states (hover, active, disabled)

modals_base.css       • Modal window base styling (border, shadow)
                      • Modal overlay darkness
                      • Modal title/body sections
                      • MUST change before modals.css takes effect

modals.css            • Specific modal animations
                      • Individual modal positioning
                      • Modal-specific color schemes
                      • Victory/defeat screen appearance

log_viewer_modal.css  • Logger appearance/layout
                      • Log entry formatting
                      • Logger container size/position
                      • Log buttons/controls

performance.css       • Performance panel layout
                      • Stat grid appearance
                      • Victory/defeat banner styling

unified_log_styles.css • Log text colors
                      • Log entry type coloring (action/error/info)
                      • Player/card name highlighting

debug.css             • Developer overlay appearance
                      • Debug info positioning
                      • Development tool styling
```

---

## 7. CSS Modification Impact Analysis

```
                  RISK LEVEL (what breaks if I change this?)
FILE              ┌─────────────────────────────────────────┐
                  │ LOW    │ MEDIUM    │ HIGH    │ CRITICAL│
─────────────────┼──────────────────────────────────────────┤
base.css          │       │           │  ✓✓✓   │
  (colors)        │       │           │ Everything! ✓✓✓
                  │
layout.css        │       │    ✓✓    │         │
  (grid)          │       │ Page layout │       │
                  │
cards.css         │    ✓ │           │         │
  (card render)   │ Only cards     │     │
                  │
components.css    │    ✓ │           │         │
  (buttons)       │ Only UI         │     │
                  │
modals_*.css      │    ✓ │           │         │
  (modals)        │ Only modals     │     │
                  │
log_viewer_...css │  ✓   │           │         │
  (logger)        │ Only logger     │     │
                  │
debug.css         │  ✓   │           │         │
  (dev tools)     │ Only debug      │     │
                  │
main.css          │  ✓   │           │         │
  (overrides)     │ Localized       │     │

LEGEND:
✓   = Affects only that feature
✓✓  = Affects multiple features
✓✓✓ = Affects everything (use with caution!)
```

---
