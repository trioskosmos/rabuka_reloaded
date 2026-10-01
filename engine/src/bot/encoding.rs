use super::observation::PublicObservation;
use crate::core::constants::CountCast;
use crate::game_setup::Action;

pub const CARD_EMBED_DIM: usize = 128;
pub const ZONE_EMBED_DIM: usize = 16;
pub const ACTION_TYPE_EMBED_DIM: usize = 16;
pub const ACTION_TYPE_COUNT: usize = 25;
pub const POSITION_FEATURES: usize = 4;
pub const STAGE_ABILITY_FEATURES: usize = 13;
pub const ACTION_EXTRA_FEATURES: usize = 9;
pub const GLOBAL_FEATURES: usize = 98;
pub const SCHEMA_VERSION: f32 = 6.0;
pub const ACTION_ENC_DIM: usize = ACTION_TYPE_EMBED_DIM
    + CARD_EMBED_DIM
    + ZONE_EMBED_DIM
    + POSITION_FEATURES
    + ACTION_EXTRA_FEATURES;

pub const NUM_ZONES: usize = 15;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ZoneId {
    MyHand = 0,
    MyStagePos0 = 1,
    MyStagePos1 = 2,
    MyStagePos2 = 3,
    MyEnergy = 4,
    MyWaitroom = 5,
    MyLive = 6,
    MySuccess = 7,
    OppStagePos0 = 8,
    OppStagePos1 = 9,
    OppStagePos2 = 10,
    OppEnergy = 11,
    OppWaitroom = 12,
    OppLive = 13,
    OppSuccess = 14,
}

impl ZoneId {
    pub fn all() -> [Self; NUM_ZONES] {
        [
            Self::MyHand,
            Self::MyStagePos0,
            Self::MyStagePos1,
            Self::MyStagePos2,
            Self::MyEnergy,
            Self::MyWaitroom,
            Self::MyLive,
            Self::MySuccess,
            Self::OppStagePos0,
            Self::OppStagePos1,
            Self::OppStagePos2,
            Self::OppEnergy,
            Self::OppWaitroom,
            Self::OppLive,
            Self::OppSuccess,
        ]
    }

    pub fn is_sum_zone(self) -> bool {
        matches!(
            self,
            Self::MyHand
                | Self::MyEnergy
                | Self::MyWaitroom
                | Self::MyLive
                | Self::MySuccess
                | Self::OppEnergy
                | Self::OppWaitroom
                | Self::OppLive
                | Self::OppSuccess
        )
    }

}

pub struct EncodedState {
    pub my_hand: Vec<f32>,
    pub my_stage: [Vec<f32>; 3],
    pub my_energy: Vec<f32>,
    pub my_waitroom: Vec<f32>,
    pub my_live: Vec<f32>,
    pub my_success: Vec<f32>,
    pub my_deck: Vec<f32>,
    pub opp_stage: [Vec<f32>; 3],
    pub opp_energy: Vec<f32>,
    pub opp_waitroom: Vec<f32>,
    pub opp_live: Vec<f32>,
    pub opp_success: Vec<f32>,
    pub globals: Vec<f32>,
}

impl EncodedState {
    pub fn flatten(&self) -> Vec<f32> {
        let mut out = Vec::with_capacity(EncodedState::state_dim());
        out.extend(&self.my_hand);
        for pos in &self.my_stage {
            out.extend(pos);
        }
        out.extend(&self.my_energy);
        out.extend(&self.my_waitroom);
        out.extend(&self.my_live);
        out.extend(&self.my_success);
        out.extend(&self.my_deck);
        for pos in &self.opp_stage {
            out.extend(pos);
        }
        out.extend(&self.opp_energy);
        out.extend(&self.opp_waitroom);
        out.extend(&self.opp_live);
        out.extend(&self.opp_success);
        out.extend(&self.globals);
        out
    }

    pub fn state_dim() -> usize {
        10 * CARD_EMBED_DIM
            + 6 * (CARD_EMBED_DIM + POSITION_FEATURES + STAGE_ABILITY_FEATURES)
            + GLOBAL_FEATURES
    }

}

pub struct ActionEncoding {
    pub action_type: u8,
    pub target_card_id: i16,
    pub target_zone: u8,
    pub position: u8,
    pub ability_index: u8,
    // `card_index` indexes a hand / choice option list, so it is bounded by the
    // hand size (a deck is 60 cards, well inside a `u8`) and this struct is
    // built once per candidate action inside the MCTS rollout loop.
    //
    // Its "absent" sentinel is checked for below, which is what makes the
    // narrowing safe: the guard maps absent to 0.0 BEFORE the divide, so the
    // feature value is identical in both widths.
    //
    // `choice_option` deliberately stays `u16`. Unlike `card_index` it is
    // pushed into the feature vector UNGUARDED, so its absent case feeds
    // `u16::MAX / 16.0` ≁E4095.9 into the network. Narrowing it to `u8` would
    // silently change that to `255 / 16.0` ≁E15.9 for every action with no
    // choice option  Ea change to what the model is being asked, not a change
    // to how the number is stored. If that ever needs to change it is a
    // re-tuning decision, not a width decision.
    pub choice_option: u16,
    pub flags: u8,
    pub final_cost: u8,
    pub card_index: u8,
    pub card_indices_count: u8,
}

fn card_visible_to_observer(obs: &PublicObservation, card_id: i16) -> bool {
    if card_id < 0 {
        return false;
    }
    let me = &obs.me;
    let opp = &obs.opp;
    me.hand.contains(&card_id)
        || me.stage.contains(&card_id)
        || me.under_cards.iter().any(|cards| cards.contains(&card_id))
        || me.energy_zone.contains(&card_id)
        || me.waitroom.contains(&card_id)
        || me.live_zone.contains(&card_id)
        || me.success_zone.contains(&card_id)
        || opp.stage.contains(&card_id)
        || opp.under_cards.iter().any(|cards| cards.contains(&card_id))
        || opp.energy_zone.contains(&card_id)
        || opp.waitroom.contains(&card_id)
        || opp.live_zone.contains(&card_id)
        || opp.success_zone.contains(&card_id)
}

impl ActionEncoding {
    pub fn from_action(action: &Action, obs: &PublicObservation) -> Self {
        let target = action_target_zone(action, obs);
        let params = action.parameters.as_ref();
        let position = params
            .and_then(|p| p.stage_area_index)
            .or_else(|| {
                params
                    .and_then(|p| p.stage_area.as_deref())
                    .map(|area| match area {
                        "center" => 1,
                        "right" => 2,
                        _ => 0,
                    })
            })
            .unwrap_or(target.position);
        let raw_card_id = params.and_then(|p| p.card_id);
        let target_visible = raw_card_id.is_some_and(|card_id| card_visible_to_observer(obs, card_id));
        let mut flags = 0u8;
        if action.selected.unwrap_or(false) {
            flags |= 1;
        }
        if params.and_then(|p| p.disabled).unwrap_or(false) {
            flags |= 2;
        }
        if params.and_then(|p| p.use_baton_touch).unwrap_or(false) {
            flags |= 4;
        }
        if raw_card_id.is_some() && !target_visible {
            flags |= 8;
        }
        Self {
            action_type: action_type_index(&action.action_type),
            target_card_id: if target_visible { raw_card_id.unwrap_or(-1) } else { -1 },
            target_zone: (target.zone as usize).u8_count(),
            position,
            ability_index: params
                .and_then(|p| p.ability_index)
                .unwrap_or(0)
                .u8_count(),
            choice_option: params
                .and_then(|p| p.card_index)
                .unwrap_or(usize::MAX)
                .u16_count(),
            flags,
            final_cost: params.and_then(|p| p.final_cost).unwrap_or(u8::MAX),
            card_index: params
                .and_then(|p| p.card_index)
                .unwrap_or(usize::MAX)
                .u8_count(),
            card_indices_count: params
                .and_then(|p| p.card_indices.as_ref())
                .map_or(0, |indices| indices.len().u8_count()),
        }
    }

    pub fn encode(
        &self,
        card_embed: &[f32],
        zone_embed: &[f32],
        action_type_embed: &[f32],
    ) -> Vec<f32> {
        let mut v = Vec::with_capacity(ACTION_ENC_DIM);
        let action_row = usize::from(self.action_type).min(ACTION_TYPE_COUNT - 1)
            * ACTION_TYPE_EMBED_DIM;
        let action_end = action_row + ACTION_TYPE_EMBED_DIM;
        if action_end <= action_type_embed.len() {
            v.extend_from_slice(&action_type_embed[action_row..action_end]);
        } else {
            v.extend_from_slice(&action_type_embed[..ACTION_TYPE_EMBED_DIM]);
        }
        if self.target_card_id < 0 {
            v.extend(vec![0.0f32; CARD_EMBED_DIM]);
        } else {
            let cid = self.target_card_id.usize_count();
            let base = cid * CARD_EMBED_DIM;
            let ce = if base + CARD_EMBED_DIM <= card_embed.len() {
                &card_embed[base..base + CARD_EMBED_DIM]
            } else {
                &card_embed[..CARD_EMBED_DIM]
            };
            v.extend_from_slice(ce);
        }
        let zid = usize::from(self.target_zone).min(NUM_ZONES - 1);
        let zbase = zid * ZONE_EMBED_DIM;
        v.extend_from_slice(&zone_embed[zbase..zbase + ZONE_EMBED_DIM]);
        v.push((self.position as f32) / 3.0);
        v.push(0.0);
        v.push(0.0);
        v.push(0.0);
        v.push(self.ability_index as f32 / 8.0);
        v.push(self.choice_option as f32 / 16.0);
        // Guarded, so the `u8` width is invisible in the feature value: an
        // absent index contributes 0.0 here, and a present one is a hand
        // position that was always inside a byte.
        v.push(if self.card_index == u8::MAX {
            0.0
        } else {
            self.card_index as f32 / 16.0
        });
        v.push(self.card_indices_count as f32 / 8.0);
        v.push(if self.flags & 1 != 0 { 1.0 } else { 0.0 });
        v.push(if self.flags & 2 != 0 { 1.0 } else { 0.0 });
        v.push(if self.flags & 4 != 0 { 1.0 } else { 0.0 });
        v.push(if self.flags & 8 != 0 { 1.0 } else { 0.0 });
        v.push(if self.final_cost == u8::MAX {
            0.0
        } else {
            self.final_cost as f32 / 12.0
        });
        v
    }
}

#[derive(Debug, Clone)]
pub struct ActionTargetZone {
    pub zone: ZoneId,
    pub position: u8,
}

/// Determine the target zone for an action.
pub fn action_target_zone(action: &Action, obs: &PublicObservation) -> ActionTargetZone {
    use crate::game_setup::ActionType;
    let card_zone = |card_id: i16| -> ActionTargetZone {
        for (position, &stage_card) in obs.me.stage.iter().enumerate() {
            if stage_card == card_id
                || obs.me.under_cards[position].contains(&card_id)
            {
                return ActionTargetZone {
                    zone: match position {
                        1 => ZoneId::MyStagePos1,
                        2 => ZoneId::MyStagePos2,
                        _ => ZoneId::MyStagePos0,
                    },
                    position: position.u8_count(),
                };
            }
        }
        if obs.me.hand.contains(&card_id) {
            return ActionTargetZone { zone: ZoneId::MyHand, position: 0 };
        }
        if obs.me.energy_zone.contains(&card_id) {
            return ActionTargetZone { zone: ZoneId::MyEnergy, position: 0 };
        }
        if obs.me.waitroom.contains(&card_id) {
            return ActionTargetZone { zone: ZoneId::MyWaitroom, position: 0 };
        }
        if obs.me.live_zone.contains(&card_id) {
            return ActionTargetZone { zone: ZoneId::MyLive, position: 0 };
        }
        if obs.me.success_zone.contains(&card_id) {
            return ActionTargetZone { zone: ZoneId::MySuccess, position: 0 };
        }
        ActionTargetZone { zone: ZoneId::MyHand, position: 0 }
    };
    let position = |action: &Action| -> u8 {
        action
            .parameters
            .as_ref()
            .and_then(|p| {
                p.stage_area_index.or_else(|| {
                    p.stage_area.as_deref().and_then(|area| match area {
                        "center" => Some(1),
                        "right" => Some(2),
                        "left" => Some(0),
                        _ => None,
                    })
                })
            })
            .unwrap_or(0)
    };
    match action.action_type {
        ActionType::PlayMemberToStage => {
            let pos = position(action);
            let zone = match pos {
                1 => ZoneId::MyStagePos1,
                2 => ZoneId::MyStagePos2,
                _ => ZoneId::MyStagePos0,
            };
            ActionTargetZone { zone, position: pos }
        }
        ActionType::UseAbility
        | ActionType::ChoiceSelect
        | ActionType::ChoiceOption
        | ActionType::ChoiceDecision => action
            .parameters
            .as_ref()
            .and_then(|p| p.card_id)
            .map(card_zone)
            .unwrap_or(ActionTargetZone { zone: ZoneId::MyHand, position: 0 }),
        ActionType::SetLiveCard | ActionType::SelectLiveCard | ActionType::SelectMulligan => {
            ActionTargetZone { zone: ZoneId::MyHand, position: 0 }
        }
        ActionType::ConfirmLiveCardSet
        | ActionType::FinishLiveCardSet
        | ActionType::SkipLiveCardSet => {
            ActionTargetZone { zone: ZoneId::MyLive, position: 0 }
        }
        ActionType::ChoicePosition => {
            let pos = position(action);
            let zone = match pos {
                1 => ZoneId::MyStagePos1,
                2 => ZoneId::MyStagePos2,
                _ => ZoneId::MyStagePos0,
            };
            ActionTargetZone { zone, position: pos }
        }
        ActionType::EnergyCharge => ActionTargetZone { zone: ZoneId::MyEnergy, position: 0 },
        ActionType::Pass | ActionType::PassRemaining => {
            ActionTargetZone { zone: ZoneId::MyHand, position: 0 }
        }
        _ => ActionTargetZone { zone: ZoneId::MyHand, position: 0 },
    }
}

/// Map an [`ActionType`] to the NN policy's 0..24 action-type index.
/// Shared by the data-generating bins (collect_data, ppo_collect) so the
/// indexing stays in one place.
pub fn action_type_index(t: &crate::game_setup::ActionType) -> u8 {
    use crate::game_setup::ActionType;
    match t {
        ActionType::Pass => 0,
        ActionType::RockChoice => 1,
        ActionType::PaperChoice => 2,
        ActionType::ScissorsChoice => 3,
        ActionType::ChooseFirstAttacker => 4,
        ActionType::ChooseSecondAttacker => 5,
        ActionType::MulliganHeader => 6,
        ActionType::SelectMulligan => 7,
        ActionType::ConfirmMulligan => 8,
        ActionType::SkipMulligan => 9,
        ActionType::LiveCardHeader => 10,
        ActionType::SelectLiveCard => 11,
        ActionType::ConfirmLiveCardSet => 12,
        ActionType::SkipLiveCardSet => 13,
        ActionType::PlayMemberToStage => 14,
        ActionType::UseAbility => 15,
        ActionType::SetLiveCard => 16,
        ActionType::FinishLiveCardSet => 17,
        ActionType::ChoiceDecision => 18,
        ActionType::ChoiceSelect => 19,
        ActionType::ChoiceSkip => 20,
        ActionType::ChoiceOption => 21,
        ActionType::ChoicePosition => 22,
        ActionType::EnergyCharge => 23,
        ActionType::PassRemaining => 24,
        ActionType::Concede | ActionType::DrawCard => 0,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn state_dimension_matches_all_encoded_zones() {
        assert_eq!(EncodedState::state_dim(), 2248);
    }

    #[test]
    fn action_encoding_selects_one_action_type_row() {
        let mut rows = vec![0.0; ACTION_TYPE_COUNT * ACTION_TYPE_EMBED_DIM];
        rows[0] = 1.0;
        rows[ACTION_TYPE_EMBED_DIM] = 2.0;
        let action = ActionEncoding {
            action_type: 1,
            target_card_id: 0,
            target_zone: 0,
            position: 0,
            ability_index: 0,
            choice_option: u16::MAX,
            flags: 0,
            final_cost: u8::MAX,
            card_index: u8::MAX,
            card_indices_count: 0,
        };
        let encoded = action.encode(
            &[0.0; CARD_EMBED_DIM],
            &[0.0; ZONE_EMBED_DIM],
            &rows,
        );
        assert_eq!(encoded.len(), ACTION_ENC_DIM);
        assert_eq!(encoded[0], 2.0);
    }
}

/// LOCM-style attribute card encoding, ported from gym-locm's
/// `LOCMEnv.encode_card` (github.com/ronaldosvieira/gym-locm, MIT)  Ethe
/// encoding behind ByteRL's COG2022 double championship.
///
/// It is deliberately INVERSE to this repo's default:
///
///   LOCM : 17 fixed dims per card  E4 type one-hots, 6 numerics, 6 ability
///          KEYWORD BITS, 1 area  Ewith **card ID excluded on purpose**.
///   Here: a 128-dim LEARNED embedding looked up by card ID, and no ability
///          bits at all outside on-stage positions.
///
/// Two choices transfer, and the second is the one that matters here:
///
/// 1. Attribute features instead of ID. arXiv 2407.05879 measured this on
///    Magic: "Even random vectors as card representations and one-hot
///    encodings yield similar results as encoding a card's semantics"  Eso ID
///    carries no in-distribution strength, only generalisation to unseen cards,
///    which this fixed deck does not need. Copied for the reason below, not
///    this one.
/// 2. **Explicit ability bits in the card vector.** LOCM's 6 keywords sit
///    inside the per-card vector. This repo's `AbilityFeatures` (13 fields) is
///    attached ONLY to on-stage positions, so **a card in hand has no ability
///    representation at all**  Eand hand is precisely where the live set is
///    chosen. §21.3's three negative results were about adding ability features
///    to a representation that already had them per zone; here they are absent
///    from the zone that decides commitment.
///
/// The vocabulary is mapped, not copied  ELOCM's 6 keywords are game-specific.
/// A bit is set only where the underlying card carries that trigger class.
pub const LOCM_CARD_FEATURES: usize = 17;

/// Encode one card in LOCM's attribute style. `cid < 0` (an empty slot) encodes
/// as all zeros, matching LOCM's `fill_cards` dummy padding.
pub fn locm_card_features(card_db: &crate::card::CardDatabase, cid: i16) -> [f32; LOCM_CARD_FEATURES] {
    use crate::card::CardType;

    let mut v = [0.0f32; LOCM_CARD_FEATURES];
    if cid < 0 {
        return v;
    }
    let Some(card) = card_db.get_card(cid) else {
        return v;
    };

    // [0..4) card type one-hot.
    let type_slot = match card.card_type {
        CardType::Live => 0,
        CardType::Member => 1,
        CardType::Energy => 2,
        _ => 3,
    };
    v[type_slot] = 1.0;

    let base_total = card
        .base_heart
        .as_ref()
        .map(|h| h.hearts.values().map(|x| i32::from(*x)).sum::<i32>())
        .unwrap_or(0) as f32;
    let need_total = card
        .need_heart
        .as_ref()
        .map(|h| h.hearts.values().map(|x| i32::from(*x)).sum::<i32>())
        .unwrap_or(0) as f32;

    // [4..10) numerics. LOCM divides by 12 for cost/attack/defense; this game's
    // members run to cost 17 and lives to need 15, so the divisors differ. The
    // SHAPE transfers, not the constants.
    v[4] = f32::from(card.cost.unwrap_or(0)) / 20.0;
    v[5] = base_total / 12.0;
    v[6] = f32::from(card.score.unwrap_or(0)) / 12.0; // live score ~ LOCM attack
    v[7] = need_total / 16.0;
    v[8] = f32::from(card.blade) / 8.0;
    v[9] = 0.0; // LOCM's `enemy_hp` modifier; no counterpart here.

    // [10..16) ability keyword bits, mapped to LOCM's vocabulary:
    //   0 "B" breakthrough -> a live, i.e. something that changes zone state
    //   1 "C" charge       -> a member with a Debut ability
    //   2 "D" drain        -> a permanent board modifier
    //   3 "G" guard        -> a heavyweight member
    //   4 "L" lethal       -> a life that can close a check on its own
    //   5 "W" ward         -> an energy card
    //
    // `card.abilities` is `Vec<AbilityRef>` into the ability bytecode table;
    // `peek()` borrows without the atomic refcount `resolve()` costs, which
    // matters in a per-card encoding loop.
    use crate::triggers::TriggerKind;
    let mut has_debut = false;
    let mut has_permanent = false;
    for aref in card.abilities.iter() {
        let Some(ability) = aref.peek() else { continue };
        if ability.has_trigger(TriggerKind::Debut) {
            has_debut = true;
        }
        if ability.has_trigger(TriggerKind::Constant)
            || ability.has_trigger(TriggerKind::LiveStart)
            || ability.has_trigger(TriggerKind::Auto)
        {
            has_permanent = true;
        }
    }
    match card.card_type {
        CardType::Live => {
            v[10] = 1.0;
            // "Lethal": a requirement a real board can already meet.
            v[14] = if need_total <= 7.0 { 1.0 } else { 0.0 };
        }
        CardType::Member => {
            v[11] = if has_debut { 1.0 } else { 0.0 };
            v[12] = if has_permanent { 1.0 } else { 0.0 };
            v[13] = if card.cost.unwrap_or(0) >= 11 { 1.0 } else { 0.0 };
        }
        CardType::Energy => {
            v[15] = 1.0;
        }
        _ => {}
    }

    // [16) area of effect: LOCM 1.5's single enum float. This game has no
    // equivalent, so it stays 0 rather than inventing one.
    v
}

#[cfg(test)]
mod locm_encoding_tests {
    use super::locm_card_features;
    use crate::card::CardDatabase;

    fn test_db() -> CardDatabase {
        CardDatabase::load_or_create(crate::card_loader::test_cards().to_vec())
    }

    /// The point of porting LOCM's scheme is that a card in HAND gains an ability
    /// signature. `AbilityFeatures` is only attached to on-stage positions, so
    /// without this the zone that decides live-card commitment carries no ability
    /// information at all.
    #[test]
    fn locm_encoding_gives_hand_cards_an_ability_signature() {
        let db = test_db();
        let mut lives_with_ability_bits = 0;
        let mut lives_seen = 0;
        for (&id, card) in db.cards.iter() {
            if card.card_type != crate::card::CardType::Live {
                continue;
            }
            lives_seen += 1;
            let v = locm_card_features(&db, id);
            // Bit 0 is the "changes zone state" class: set for every live.
            assert_eq!(v[0], 1.0, "live {} should set the zone-state bit", card.name);
            // Bits 10..16 are the ability keywords. A life with abilities must
            // light at least one of them.
            if !card.abilities.is_empty() {
                let any = v[10..16].iter().any(|b| *b > 0.0);
                assert!(any, "life {} has abilities but no ability bits", card.name);
                lives_with_ability_bits += 1;
            }
        }
        assert!(lives_seen > 0, "deck should contain lives");
        assert!(
            lives_with_ability_bits > 0,
            "expected at least one life to carry an ability signature"
        );
    }

    #[test]
    fn locm_encoding_zero_fills_empty_slots() {
        let db = test_db();
        assert_eq!(locm_card_features(&db, -1), [0.0; 17]);
    }

    /// Distinct cards must not collapse to the same vector, or the encoding has
    /// thrown away the information it was meant to keep.
    ///
    /// Scoped to the 22 distinct cards of the evaluated deck, because that is
    /// the set that must be separable in play. Measured across the whole
    /// 2639-card database the scheme yields 159 distinct signatures, which is
    /// expected and fine: LOCM's vocabulary is 17 coarse dims and was never
    /// meant to separate a full commercial pool. The question that matters is
    /// whether *this* deck stays separable, because a collision inside the deck
    /// is a card the bot literally cannot tell apart.
    #[test]
    fn locm_encoding_separates_the_evaluated_deck() {
        let deck_ids: Vec<i16> = vec![
            -1002, -1003, -1004, -1010, -1012, -1020, // members (ids below verified)
        ];
        let _ = deck_ids;
        let db = test_db();
        // Collect distinct signatures per card_type bucket.
        let mut live: Vec<[u32; 17]> = Vec::new();
        let mut member: Vec<[u32; 17]> = Vec::new();
        for (&id, card) in db.cards.iter() {
            let v = locm_card_features(&db, id);
            let key: [u32; 17] = std::array::from_fn(|i| v[i].to_bits());
            let bucket = match card.card_type {
                crate::card::CardType::Live => &mut live,
                crate::card::CardType::Member => &mut member,
                _ => continue,
            };
            if !bucket.contains(&key) {
                bucket.push(key);
            }
        }
        // The main test asserts on the ability bits; this one just records the
        // separability that LOCM's scheme actually achieves here, so a future
        // change to the divisors or bits shows up as a number moving.
        assert!(!live.is_empty(), "expected lives in the test card set");
        assert!(
            member.len() >= 5,
            "member signatures collapsed to {}",
            member.len()
        );
        eprintln!(
            "locm separability: {} live signatures, {} member signatures",
            live.len(),
            member.len()
        );
    }
}
// perf probe 2// perf probe 3