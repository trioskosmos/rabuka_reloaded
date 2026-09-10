import sys

with open('src/bot/strategy_v7.rs', 'rb') as f:
    content_bytes = f.read()

# Remove BOM if present
if content_bytes.startswith(b'\xff\xfe') or content_bytes.startswith(b'\xfe\xff'):
    content_bytes = content_bytes[2:]
elif content_bytes.startswith(b'\xef\xbb\xbf'):
    content_bytes = content_bytes[3:]

content = content_bytes.decode('utf-8')

# 1. Update header
content = content.replace(
    '//! Strategy bot v6 \u2014 aggressive, tempo-first development with binomial-aware\n//! live sets.',
    '//! Strategy bot v7 \u2014 improved v6 with fixed pass logic and member dev bonus.'
)

# 2. Rename functions
content = content.replace('pub fn choose_action_v6', 'pub fn choose_action_v7')
content = content.replace('pub fn choose_live_set_v6', 'pub fn choose_live_set_v7')
content = content.replace('pub fn choose_mulligan_v6', 'pub fn choose_mulligan_v7')
content = content.replace('V6_DEBUG', 'V7_DEBUG')
content = content.replace('V6_TRACE', 'V7_TRACE')
content = content.replace('V6L ', 'V7L ')  # trace prefix

# 3. Reduce passable penalty for member plays from 60 to 35
old_passable = '''        // Doctrine 1: passable lives (placements-in-waiting).
        let d_pass = passable_count(&sim, me, db) as f64 - base_passable as f64;
        val += 60.0 * d_pass;
        if d_pass != 0.0 {
            parts.push(format!("pass{:+}", d_pass));
        }'''

new_passable = '''        // Doctrine 1: passable lives (placements-in-waiting).
        // When playing a member from hand, reduce the passable penalty since we're
        // developing the board. The hearts/blades gained compensate for the hand loss.
        let d_pass = passable_count(&sim, me, db) as f64 - base_passable as f64;
        let is_member_play = a.action_type == ActionType::PlayMemberToStage;
        let pass_weight = if is_member_play { 35.0 } else { 60.0 };
        val += pass_weight * d_pass;
        if d_pass != 0.0 {
            parts.push(format!("pass{:+}", d_pass));
        }

        // Bonus for productive member play (has hearts or blades)
        if is_member_play {
            if let Some(cid) = a.parameters.as_ref().and_then(|p| p.card_id) {
                if let Some(card) = db.get_card(cid) {
                    let blade = card.blade as i32;
                    let hearts = card.base_heart.as_ref().map(|bh| bh.hearts.values_sum() as i32).unwrap_or(0);
                    if blade > 0 || hearts > 0 {
                        val += 15.0;
                        parts.push("member_dev+15".into());
                    }
                }
            }
        }'''

content = content.replace(old_passable, new_passable)

# Write the modified file
with open('src/bot/strategy_v7.rs', 'wb') as f:
    f.write(content.encode('utf-8'))

print('Done')