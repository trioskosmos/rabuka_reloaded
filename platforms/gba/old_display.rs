commit 62d019f8d1bd035f1581b6a9e5400aca6eedc787
Author: trioskosmos <trioskosmos@users.noreply.huggingface.co>
Date:   Fri Sep 4 00:14:47 2026 +1000

    GBA display: fix VRAM leaks, ghost cards, transition OOM
    
    - Replace TileSetting::new(0,...) clears with TileSetting::BLANK for 8bpp transparency
    - Clear all 7 backgrounds on every screen transition (board/detail/menu)
    - Remove bogus tile_set_cache reference
    - Fix &TileSet borrow issues in draw_slot and render_card_detail
    - Fix unused variable warnings
    
    ROM builds successfully.

diff --git a/platforms/gba/src/display.rs b/platforms/gba/src/display.rs
index 40245b13..e980aa76 100644
--- a/platforms/gba/src/display.rs
+++ b/platforms/gba/src/display.rs
@@ -1,5 +1,4 @@
-・ｿuse core::cell::RefCell;
-use alloc::collections::BTreeMap;
+・ｿ
 use alloc::string::String;
 use alloc::vec::Vec;
 
@@ -79,9 +78,6 @@ pub struct Display<'a> {
     action_bg: RegularBackground,
     detail_art_bg: RegularBackground,
     detail_text_bg: RegularBackground,
-    /// Cached TileSet objects to prevent VRAM leak from repeated TileSet::new() calls.
-    /// Key: "front_type:card_no" (e.g., "hand:LL-001", "stage:LL-001", etc.)
-    tile_set_cache: RefCell<BTreeMap<alloc::string::String, TileSet>>,
 }
 
 /// One queued card image for the next [`Display::swap_buffers`].
@@ -194,17 +190,9 @@ impl<'a> Display<'a> {
                 RegularBackgroundSize::Background32x32,
                 TileFormat::FourBpp,
             ),
-            tile_set_cache: RefCell::new(BTreeMap::new()),
         }
     }
 
-    /// Get or create a cached TileSet for the given front type and card number.
-    /// Prevents VRAM leak from repeated TileSet::new() calls with same data.
-    fn get_tile_set(&self, front_type: &str, card_no: &str, tiles: &'static [u8]) -> &TileSet {
-        let key = alloc::format!("{}:{}", front_type, card_no);
-        self.tile_set_cache.borrow_mut().entry(key).or_insert_with(|| unsafe { TileSet::new(tiles, TileFormat::EightBpp) })
-    }
-
     pub fn clear(&mut self) {
         self.buf.clear();
         self.pending_art.clear();
@@ -402,13 +390,19 @@ impl<'a> Display<'a> {
         let ui_ts = unsafe { TileSet::new(BOARD_UI, TileFormat::FourBpp) };
         let e0 = TileEffect::new(false, false, 15);
 
-        // Clear art_bg (8bpp) to transparent (index 0) each frame to prevent ghost cards
-        let clear_8bpp = TileSetting::new(0, TileEffect::new(false, false, 0));
+        // Clear art_bg (8bpp) to transparent each frame to prevent ghost cards
+        let clear_8bpp = TileSetting::BLANK;
         let back_ts = unsafe { TileSet::new(BACK_FRONT, TileFormat::EightBpp) };
         for ty in 0..ROWS {
             for tx in 0..COLS {
                 self.board_art_bg.set_tile((tx, ty), &back_ts, clear_8bpp);
-                ui_bg.set_tile((tx, ty), &ui_ts, TileSetting::new(UI_EMPTY, e0));
+                self.board_ui_bg.set_tile((tx, ty), &ui_ts, TileSetting::new(UI_EMPTY, e0));
+                // Clear detail backgrounds to free VRAM from previous detail view
+                self.detail_art_bg.set_tile((tx, ty), &back_ts, clear_8bpp);
+                self.detail_text_bg.set_tile((tx, ty), &ui_ts, TileSetting::new(UI_EMPTY, e0));
+                // Clear menu backgrounds to free VRAM from previous menu view
+                self.menu_art_bg.set_tile((tx, ty), &back_ts, clear_8bpp);
+                self.menu_bg.set_tile((tx, ty), &ui_ts, TileSetting::new(UI_EMPTY, e0));
             }
         }
         Self::blit_line(&mut self.board_ui_bg, &font_ts, &icon_ts, e0, &frame.header, 0, 0);
@@ -426,7 +420,7 @@ impl<'a> Display<'a> {
             for (i, slot) in stage.iter().enumerate() {
                 let xi = if is_opp { 2 - i } else { i };
                 let x = 1 + STAGE_PITCH * xi as i32;
-                self.draw_slot(
+                Self::draw_slot(
                     &mut self.board_art_bg,
                     &mut self.board_ui_bg,
                     &ui_ts,
@@ -448,7 +442,7 @@ impl<'a> Display<'a> {
             for (i, slot) in live.iter().enumerate() {
                 let xi = if is_opp { 2 - i } else { i };
                 let x = INFO_X + LIVE_PITCH * xi as i32;
-                self.draw_slot(
+                Self::draw_slot(
                     &mut self.board_art_bg,
                     &mut self.board_ui_bg,
                     &ui_ts,
@@ -470,7 +464,7 @@ impl<'a> Display<'a> {
             for (i, slot) in live_set.iter().enumerate() {
                 let xi = if is_opp { 2 - i } else { i };
                 let x = INFO_X + LIVE_PITCH * xi as i32;
-                self.draw_slot(
+                Self::draw_slot(
                     &mut self.board_art_bg,
                     &mut self.board_ui_bg,
                     &ui_ts,
@@ -486,14 +480,14 @@ impl<'a> Display<'a> {
                     BACK_FRONT,
                     flipped,
                     "live",
-);
+                );
             }
         }
 
         // Hand window; a gold badge marks more cards off-screen right.
-        for (i, slot) in frame.hand.iter().enumerate() {
+for (i, slot) in frame.hand.iter().enumerate() {
             let x = HAND_PITCH * i as i32;
-            self.draw_slot(
+            Self::draw_slot(
                 &mut self.board_art_bg,
                 &mut self.board_ui_bg,
                 &ui_ts,
@@ -611,17 +605,29 @@ impl<'a> Display<'a> {
         let icon_ts = unsafe { TileSet::new(&TEXTICON_TILES.0, TileFormat::FourBpp) };
         let e_text = TileEffect::new(false, false, 15);
 
+        // Clear board backgrounds to free VRAM from previous board view
+        let clear_8bpp = TileSetting::BLANK;
+        let back_ts = unsafe { TileSet::new(BACK_FRONT, TileFormat::EightBpp) };
+        let ui_ts = unsafe { TileSet::new(BOARD_UI, TileFormat::FourBpp) };
+        let e_ui = TileEffect::new(false, false, 15);
+        for ty in 0..ROWS {
+            for tx in 0..COLS {
+                self.board_art_bg.set_tile((tx, ty), &back_ts, clear_8bpp);
+                self.board_ui_bg.set_tile((tx, ty), &ui_ts, TileSetting::new(UI_EMPTY, e_ui));
+            }
+        }
+
         let mut f = self.gfx.frame();
 
         if let Some(art) = art {
             // Baked 12x18 tiles address the portrait's own static bytes, so
             // every card uploads its own (pointer, tile) pairs 窶・always the
             // right pixels, never stale, and freed on close.
-            let art_ts = self.get_tile_set("detail", art.card_no, art.tiles);
+            let art_ts = unsafe { TileSet::new(art.tiles, TileFormat::EightBpp) };
             for i in 0..(DETAIL_DW * DETAIL_DH) {
                 let tx = (i % DETAIL_DW) as i32;
                 let ty = (i / DETAIL_DW) as i32 + DETAIL_Y0;
-                self.detail_art_bg.set_tile((tx, ty), art_ts, TileSetting::new(i as u16, TileEffect::new(false, false, 0)));
+                self.detail_art_bg.set_tile((tx, ty), &art_ts, TileSetting::new(i as u16, TileEffect::new(false, false, 0)));
             }
             self.detail_art_bg.show(&mut f);
         }
@@ -725,6 +731,31 @@ impl<'a> Display<'a> {
             }
         }
 
+        // Clear menu_art_bg (8bpp) each frame to prevent ghost cards
+        let clear_8bpp = TileSetting::BLANK;
+        let back_ts = unsafe { TileSet::new(BACK_FRONT, TileFormat::EightBpp) };
+        for ty in 0..ROWS {
+            for tx in 0..COLS {
+                self.menu_art_bg.set_tile((tx, ty), &back_ts, clear_8bpp);
+            }
+        }
+
+        // Clear board backgrounds to free VRAM from previous board view
+        for ty in 0..ROWS {
+            for tx in 0..COLS {
+                self.board_art_bg.set_tile((tx, ty), &back_ts, clear_8bpp);
+                self.board_ui_bg.set_tile((tx, ty), &ui_ts, TileSetting::new(UI_EMPTY, e_ui));
+            }
+        }
+
+        // Clear detail backgrounds to free VRAM from previous detail view
+        for ty in 0..ROWS {
+            for tx in 0..COLS {
+                self.detail_art_bg.set_tile((tx, ty), &back_ts, clear_8bpp);
+                self.detail_text_bg.set_tile((tx, ty), &ui_ts, TileSetting::new(UI_EMPTY, e_ui));
+            }
+        }
+
         let e = TileEffect::new(false, false, 15);
         let mut ty = 0i32;
 
@@ -768,8 +799,8 @@ impl<'a> Display<'a> {
                 .iter()
                 .find(|f| f.card_no == q.card_no.as_str())
             {
-                let front_type = if q.cols == 5 && q.rows == 6 { "stage" } else { "hand" };
-                let ts = self.get_tile_set(front_type, &q.card_no, front.tiles);
+                let _front_type = if q.cols == 5 && q.rows == 6 { "stage" } else { "hand" };
+                let ts = unsafe { TileSet::new(front.tiles, TileFormat::EightBpp) };
                 for ay in 0..q.rows {
                     for ax in 0..q.cols {
                         let sidx = (ay * q.cols + ax) as u16;
@@ -824,7 +855,6 @@ impl<'a> Display<'a> {
     /// solid gray slot on the text BG), plus the gold badge in the right gap
     /// column when the card has valid actions.
     fn draw_slot(
-        &self,
         art_bg: &mut RegularBackground,
         ui_bg: &mut RegularBackground,
         ui_ts: &TileSet,
@@ -839,7 +869,7 @@ impl<'a> Display<'a> {
         waited_fronts: &[crate::card_art_gen::CardFront],
         back: &'static [u8],
         flipped: bool,
-        front_type: &str, // "hand", "stage", "live", "waited"
+        _front_type: &str, // "hand", "stage", "live", "waited"
     ) {
         let (cols, rows) = if slot.waited {
             (4, 3) // wait grid: 4x3 tiles = 32x24
@@ -859,13 +889,13 @@ impl<'a> Display<'a> {
             // Face-down: card back at live-slot geometry. `hidden` is only set
             // on 3x2 live-set slots, matching BACK_FRONT's baked grid.
             Some(_) if slot.hidden => {
-                let ts = self.get_tile_set("back", "back", back);
+                let ts = unsafe { TileSet::new(back, TileFormat::EightBpp) };
                 for ty in 0..2 {
                     for tx in 0..3 {
                         let sidx = if flipped { (1 - ty) * 3 + (2 - tx) } else { ty * 3 + tx } as u16;
                         art_bg.set_tile(
                             (x + tx, y + ty),
-                            ts,
+                            &ts,
                             TileSetting::new(sidx, art_eff(flipped)),
                         );
                         ui_bg.set_tile((x + tx, y + ty), &ui_ts, TileSetting::BLANK);
@@ -874,13 +904,13 @@ impl<'a> Display<'a> {
             }
             Some(card_no) => match fronts.iter().find(|f| f.card_no == card_no.as_str()) {
                 Some(front) => {
-                    let ts = self.get_tile_set(front_type, card_no, front.tiles);
+                    let ts = unsafe { TileSet::new(front.tiles, TileFormat::EightBpp) };
                     for ty in 0..rows {
                         for tx in 0..cols {
                             let sidx = if flipped { (rows - 1 - ty) * cols + (cols - 1 - tx) } else { ty * cols + tx } as u16;
                             art_bg.set_tile(
                                 (x + tx, y + ty),
-                                ts,
+                                &ts,
                                 TileSetting::new(sidx, art_eff(flipped)),
                             );
                             // Clear ui_bg so card art shows through (ui is in front)
