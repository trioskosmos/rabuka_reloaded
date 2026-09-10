with open('src/bot/strategy_v7.rs', 'r') as f:
    content = f.read()

old = """        // Doctrine 1: passable lives (placements-in-waiting).
        // When playing a member from hand, reduce the passable penalty since we're
        // developing the board. The hearts/blades gained compensate for the hand loss.
        let is_member_play = a.action_type == ActionType::PlayMemberToStage;
        let pass_weight = if is_member_play { 30.0 } else { 60.0 };
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
        }

        // Doctrine 2: ammo \u2014 lives in hand are future placements."""

new = """        // Doctrine 1: passable lives (placements-in-waiting).
        // When playing a member from hand, reduce the passable penalty since we're
        // developing the board. The hearts/blades gained compensate for the hand loss.
        let d_pass = passable_count(&sim, me, db) as f64 - base_passable as f64;
        let is_member_play = a.action_type == ActionType::PlayMemberToStage;
        let pass_weight = if is_member_play { 30.0 } else { 60.0 };
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
        }

        // Doctrine 2: ammo \u2014 lives in hand are future placements."""

if old in content:
    content = content.replace(old, new)
    with open('src/bot/strategy_v7.rs', 'w') as f:
        f.write(content)
    print('Replaced successfully')
else:
    print('NOT FOUND')
    idx = content.find('passable lives (placements')
    if idx >= 0:
        print(repr(content[idx:idx+600]))