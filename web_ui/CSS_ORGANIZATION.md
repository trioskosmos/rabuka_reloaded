# Frontend CSS Organization & Dependency Map

**Last Updated:** 2026-03-16
**Status:** Vite-ready, modular, dependency-tracked

---

## I. CSS Load Order (Index in `style.css`)

All CSS is loaded through **`style.css`** as a unified entry point. Load order matters for cascade behavior.

```css
/* Level 1: Foundations (must load first) */
@import './base.css';           /* CSS variables, color tokens, resets */
@import './layout.css';         /* Grid, flexbox, spacing, positioning */

/* Level 2: Component Base Layers */
@import './cards.css';          /* Card rendering, zones, orientations */
@import './components.css';     /* Buttons, inputs, selects, general UI */
@import './ui_components.css';  /* Additional UI utilities */

/* Level 3: Modal Architecture */
@import './modals_base.css';    /* Base modal structure, overlay */
@import './modals.css';         /* Modal variants, positioning, animations */

/* Level 4: Feature-Specific Styling */
@import './log_viewer_modal.css';      /* Logger UI in modal form */
@import './performance.css';           /* Performance summary panels */
@import './performance_stats.css';     /* Stats grid, metric displays */
@import './unified_log_styles.css';    /* Log colors, formatting */
@import './deck_validation.css';       /* Validation UI, error states */

/* Level 5: Entry Points & Overrides */
@import './main.css';           /* Page-specific, application-level */
@import './debug.css';          /* Dev tools, debugging overlays (last) */
```

---

## II. CSS File Effect Map

### **`base.css`** (137 lines)
**Scope:** Global theme & design tokens  
**Loaded:** First  
**Affects:** Everything (all other files depend on these variables)

**Key Elements:**
- `:root` CSS variables defining:
  - Color palette: `--bg-primary`, `--accent-pink`, `--accent-blue`, etc.
  - Spacing scale: `--space-xs` through `--space-xxl` (fluid, responsive)
  - Typography: `--font-size-xs` through `--font-size-lg`
  - Shadows, borders, glass effects
  - Easing functions for animations
  - Z-index scale: `--z-tooltip`, `--z-modal`, `--z-top`

**Impact:**
- 🎨 All colors in the app derive from these tokens
- 📏 All spacing is fluid and scales with viewport
- ✅ Change here = change everywhere

**Note:** Highly coupled to other files—modifying color tokens affects all dependent CSS.

---

### **`layout.css`** (315 lines)
**Scope:** Page structure, grid systems, responsive layout  
**Loaded:** 2nd  
**Affects:** Page layout, header, main content area, sidebars

**Key Elements:**
- `.header-utilities`, `.pills-player-group` → Header UI buttons
- `.game-board`, `.zone-area` → Main game grid and playable areas
- `.sidebar`, `.mobile-sidebar` → Side panels and mobile menu
- `.action-bar`, `.mobile-action-bar` → Action buttons
- Flexbox/grid rules for responsive stacking
- Media queries for `@media (max-width: 768px)`

**Impact:**
- 🎯 Controls overall page structure and responsiveness
- 📱 Handles mobile vs. desktop layout differences
- 🔧 Modify when changing page layout or adding new sections

**Dependents:** `cards.css` (zones), `components.css` (buttons in layout)

---

### **`cards.css`** (489 lines)
**Scope:** Card rendering, zones, transformations  
**Loaded:** 3rd  
**Affects:** All `.card` elements, zone backgrounds, card states

**Key Selectors:**
- `.card` → Base card styling (size, transform, hover)
- `.card.type-live` → Live cards (rotated orientation)
- `.card.type-support` → Support card styling
- `.card.orientation-landscape` → Rotated display
- `.zone`, `.zone-deck`, `.zone-hand`, `.zone-field` → Play zones
- `.zone-bg` → Zone background colors
- `.card-hover-scale` → Hover animations

**Impact:**
- 🎴 Card appearance, size, rotation
- 🏑 Zone background colors and highlighting
- ✨ Card interaction animations (hover, select)

**Dependencies:** 
- `base.css` (colors, spacing, easing)
- `layout.css` (positioning structures)

---

### **`components.css`** (337 lines)
**Scope:** Reusable UI components (buttons, inputs, dropdowns)  
**Loaded:** 4th  
**Affects:** All interactive elements

**Key Selectors:**
- `.btn` → Base button styling
- `.btn.btn-primary`, `.btn.btn-secondary` → Button variants
- `.btn:hover`, `.btn:active`, `.btn:disabled` → Button states
- `.input-field`, `.select-dropdown` → Form inputs
- `.badge`, `.tag` → Labels and tags

**Impact:**
- 🔘 All buttons in the UI
- 📝 Input field appearance
- 🎯 Consistency across interactive elements

**Dependencies:**
- `base.css` (colors, spacing)

---

### **`ui_components.css`** (119 lines)
**Scope:** Additional utility components  
**Loaded:** 5th  
**Affects:** Specific component variants

**Contents:**
- Toggle switches
- Tabs and tab panels
- Spinners/loaders
- Tooltips (passive, shown via JS)
- Badges and status indicators

**Impact:**
- 🔀 Custom UI patterns not in base components
- 📊 Status indicators and badges

**Dependencies:**
- `base.css`, `components.css`

---

### **`modals_base.css`** (75 lines)
**Scope:** Shared modal structure, overlay, base positioning  
**Loaded:** 6th (FIRST modal file)  
**Affects:** `.modal-overlay`, `.modal-content`, common modal structure

**Key Elements:**
- `.modal-overlay` → Full-screen semi-transparent backdrop (fixed)
- `.modal-content` → Modal box (padding, max-width, shadow)
- `.modal-header`, `.modal-title` → Modal title bar
- `.modal-body`, `.modal-footer` → Content sections
- `.modal-close-btn` → Common close button styling

**Impact:**
- 🪟 All modals share this base structure
- 🎨 Overlay appearance (dark background)
- 📦 Modal box dimensions and shadows

**Note:** This is intentionally minimal—only shared properties. Variants go in `modals.css`.

**Dependencies:**
- `base.css` (colors, shadows, z-index)

---

### **`modals.css`** (118 lines)
**Scope:** Modal variants, specific modal styling  
**Loaded:** 7th (AFTER modals_base.css)  
**Affects:** Specific modal types (Lobby, Setup, Debug, etc.)

**Key Elements:**
- `.modal.lobby-setup` → Lobby configuration modal
- `.modal.game-setup` → Game setup modal
- `.modal.debug-modal` → Developer modal
- `.modal.victory-modal`, `.modal.defeat-modal` → Game end screens
- Animation rules, positioning overrides

**Impact:**
- 🎮 Appearance of specific modal screens
- ✨ Modal entrance/exit animations

**Overrides:** Builds on `modals_base.css`

**Dependencies:**
- `modals_base.css` (base structure)
- `base.css` (colors, animations)

---

### **`log_viewer_modal.css`** (441 lines)
**Scope:** Logger UI, log entry rendering  
**Loaded:** 8th  
**Affects:** Log viewer modal and log entry formatting

**Key Elements:**
- `.log-modal-btn` → Button to open log viewer
- `#log-viewer-modal` → Main log container
- `.log-entry` → Individual log line styling
- `.log-entry.action`, `.log-entry.error`, `.log-entry.info` → Log types
- `.log-timestamp`, `.log-text`, `.log-metadata` → Log components
- `.log-filter-bar` → Filter controls

**Impact:**
- 📋 Log display formatting
- 🎨 Color coding for different log types (action = green, error = red, etc.)
- 🔍 Log filter/search UI

**Complex:** This is the largest feature-specific CSS (441 lines).

**Dependencies:**
- `base.css`, `modals_base.css`, `components.css`

---

### **`performance.css`** (220 lines)
**Scope:** Performance summary panels, victory/defeat screens  
**Loaded:** 9th  
**Affects:** Game outcome presentation, stats panels

**Key Elements:**
- `.perf-overview-shell` → Performance summary container
- `.perf-comparison-banner` → Win/loss header
- `.perf-player-grid` → 2-column player stats layout
- `.perf-panel` → Individual player performance box
- `.perf-panel.success`, `.perf-panel.failure` → Success/failure styling
- `.perf-metric-grid` → Grid for stats display

**Impact:**
- 📊 Victory/defeat screen appearance
- 🎯 Performance metrics display layout
- 🏆 Winner/loser highlighting

**Dependencies:**
- `base.css`, `modals.css` (often shown in modals)

---

### **`performance_stats.css`** (96 lines)
**Scope:** Detailed stat rendering within performance panels  
**Loaded:** 10th  
**Affects:** Individual stat cells and formatting

**Key Elements:**
- `.perf-stat-cell` → Single stat box
- `.stat-label`, `.stat-value` → Text components
- `.stat-bar` → Progress bar for normalized stats
- `.stat-icon` → Icon next to stat name

**Impact:**
- 📈 Formatting of individual performance metrics
- 🎨 Stat cells color and layout

**Isolation:** Typically doesn't conflict because it's highly specific to perf panels.

**Dependencies:**
- `performance.css` (parent structure)

---

### **`unified_log_styles.css`** (239 lines)
**Scope:** Log message color and text styling  
**Loaded:** 11th  
**Affects:** Log entry text appearance, color coding

**Key Elements:**
- `.log-action-*` classes → Color coding for action types
- `.log-player-name` → Player name highlighting
- `.log-card-name` → Card name formatting
- `.log-error-text` → Error styling
- `.log-highlight` → Emphasis/highlight colors

**Impact:**
- 🎨 Log text colors (action = green, card = blue, error = red, etc.)
- 📝 Log text formatting (bold, italics, backgrounds)

**Note:** Heavily intertwined with `log_viewer_modal.css`.

**Dependencies:**
- `base.css` (color tokens)
- `log_viewer_modal.css` (parent log structure)

---

### **`deck_validation.css`** (119 lines)
**Scope:** Deck validation UI, error states, warnings  
**Loaded:** 12th  
**Affects:** Deck builder validation, error messages

**Key Elements:**
- `.validation-check` → Single validation rule
- `.validation-check.pass`, `.validation-check.fail` → Pass/fail styling
- `.validation-banner` → Top-level validation summary
- `.validation-error-msg` → Error text
- `.deck-count-indicator` → Card count display

**Impact:**
- ✅ "Deck is valid" UI
- ❌ Red error highlighting for invalid decks
- ⚠️ Warning messages

**Isolation:** Mostly self-contained to deck builder pages.

**Dependencies:**
- `base.css`, `components.css`

---

### **`main.css`** (149 lines)
**Scope:** Application-level overrides, page-specific rules  
**Loaded:** 13th (Second-to-last)  
**Affects:** Application-level tweaks, entry-point specifics

**Key Elements:**
- `.panel-container` → Containers on main page
- `.log-container` → Game log panel styling
- `.action-btn` → Action button variant
- Application-wide overrides not specific to any feature
- Comments documenting the module load order

**Impact:**
- 🔧 Fine-tuning of component appearance in context
- 📋 Application-specific layout adjustments

**Typically Minimal:** Added rules to refine component behavior without changing components themselves.

**Dependencies:**
- All previous files (this is the override layer)

---

### **`debug.css`** (37 lines)
**Scope:** Developer tools, debug overlays  
**Loaded:** 14th (LAST)  
**Affects:** Debug UI only (not visible in production)

**Key Elements:**
- `.debug-overlay` → Floating debug panel (top-left corner)
- `.debug-badge` → Debug info display
- `.debug-error-panel` → Error messages in debug panel
- `.floating-tooltip` → Developer tooltips

**Impact:**
- 🧪 Developer tools and debugging overlays
- 📍 Overlay positioning and appearance

**Can be safely removed:** DEBUG_MODE guards in JS prevent this from affecting production.

**Dependencies:**
- `base.css` (colors)

---

## III. CSS Dependency Graph

```
base.css
  ├─→ layout.css
  │    ├─→ cards.css
  │    └─→ components.css
  │         ├─→ ui_components.css
  │         └─→ modals_base.css
  │              ├─→ modals.css
  │              ├─→ log_viewer_modal.css
  │              ├─→ performance.css
  │              │    └─→ performance_stats.css
  │              └─→ deck_validation.css
  │
  ├─→ unified_log_styles.css  (heavy cross-dependency with log_viewer_modal.css)
  ├─→ main.css               (overrides everything)
  └─→ debug.css              (overrides for dev mode)
```

---

## IV. Common Editing Scenarios

### Scenario 1: Change all button colors
**File to edit:** `base.css`  
**Variables:** `--accent-blue`, `--accent-pink`, etc.  
**Effect:** Propagates to all buttons automatically (via `components.css`)  
**Risk:** Low (variables are intentional)

### Scenario 2: Change modals appearance
**Files to check/edit:**
1. `modals_base.css` → border, shadow, background
2. `modals.css` → specific modal animations
3. `main.css` → application-level modal tweaks  
**Risk:** Medium (modals appear in many contexts)

### Scenario 3: Change card rendering
**Files to check/edit:**
1. `cards.css` → card size, interactions
2. `base.css` → colors (for zone backgrounds)
3. `layout.css` → zone positioning  
**Risk:** Medium-High (affects core game UI)

### Scenario 4: Change log appearance
**Files to check/edit:**
1. `log_viewer_modal.css` → modal structure
2. `unified_log_styles.css` → text colors
3. `performance.css` → if performance data shown alongside logs  
**Risk:** Low (isolated to log viewer)

### Scenario 5: Add new component
**Steps:**
1. Create new CSS file: `css/mynewcomponent.css`
2. Add `@import './mynewcomponent.css';` to `style.css` **after** dependencies loaded
3. Use variables from `base.css` (colors, spacing, shadows)  
**Risk:** Low if done properly

---

## V. Vite Integration & Asset Sync

### Why Vite?
- **Build optimization:** Bundles CSS + JS + images
- **Dev mode:** CSS hot module replacement (instant reload on save)
- **Production:** Minified output (`dist/` folder)

### Asset Sync Process
```
frontend/web_ui/[HTML/CSS/JS]
    ↓
[Vite Build (if dist/ exists)] OR [Raw source (fallback)]
    ↓
launcher/.static_content_staging/ [Temporary]
    ↓
launcher/static_content/ [Final, served by backend]
```

**Key Files:**
- `frontend/web_ui/vite.config.js` → Vite configuration
- `frontend/web_ui/package.json` → Build scripts
- `tools/sync_launcher_assets.py` → Sync script (recently fixed)

### When to use Vite build vs. raw source
- **Development:** Use raw source (`npm run dev` for live reload)
- **Testing/CI:** Use Vite build (`npm run build`, then run sync)
- **Production:** Always use Vite build

---

## VI. Troubleshooting CSS Issues

### Problem: "My CSS change isn't taking effect"
**Checklist:**
- [ ] Did you edit the right file? (Check load order)
- [ ] Is Vite rebuilding? (`npm run dev` terminal should show update)
- [ ] Is browser cache stale? (Hard refresh: Ctrl+Shift+R)
- [ ] Check browser DevTools → Inspector → verify CSS is loaded
- [ ] Check precedence—is another rule overriding it? (Later load order wins)

### Problem: "CSS works in Dev but not in Production"
- [ ] Did you run `npm run build`?
- [ ] Did you run `python tools/sync_launcher_assets.py`?
- [ ] Check that `launcher/static_content/` has updated CSS files

### Problem: "Two CSS rules conflict"
**Resolution:**
1. Check load order in `style.css`
2. Later files override earlier files
3. If same file, use CSS specificity (class > element, ID > class)
4. Avoid `!important` unless absolutely necessary

### Problem: "CSS file is too large or hard to maintain"
**Solution:** Split and refactor
1. Identify concern (e.g., "all modal fixes")
2. Create new component file: `css/mynewcomponent.css`
3. Move rules there
4. Add import to `style.css` in appropriate order
5. Update this document (Section II)

---

## VII. Next Steps for Consolidation

### Current State ✅
- Single `style.css` entry point
- Clear load order with comments
- CSS mostly organized by concern
- Vite integration ready

### Recommended Next Steps
1. **Index CSS rules** → Create mapping of `.classname` → `filename` for quick lookup
2. **Remove dead code** → Check if `ui_components.css` classes are actually used
3. **Consolidate modals** → `modals.css` + `modals_base.css` might merge well
4. **Performance audit** → Run CSS through PurgeCSS to find unused rules

### DO NOT Consolidate Into Single File
- **Reason:** No benefit; Vite already bundles
- **Risk:** Lose organization, hard to debug
- **Current approach:** Keep modular, let build tool combine

---

