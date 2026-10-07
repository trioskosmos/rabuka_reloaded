use super::*;

// `AbilityEffect.kind` holds the decoded effect payload. It used to be a
// hand-rolled pool box (`core::pool::EkBox`) on desktop and a plain `Box` on
// `no_std`. The pool was worse on both counts: `Clone` took two `Mutex`
// lock/unlock pairs and then deep-copied the `EffectKind` (whose
// `EffectFilter` is ~1.2 KB) into a *fresh* pool slot, and `Deref`/`DerefMut`
// were `unsafe` dereferences of pool memory rather than a pointer chase.
//
// `Arc` is what the `no_std` side was already missing: `Clone` becomes a
// refcount bump, reads are an ordinary deref, and the mutation sites use
// `Arc::make_mut`, which is a no-op clone-free write whenever the effect is
// uniquely owned (which is the case for every build-time site) and a copy
// only when it is genuinely shared.
pub(crate) type EkBox = Arc<EffectKind>;

pub(crate) fn ek_box_new(val: EffectKind) -> EkBox {
    Arc::new(val)
}

#[derive(Debug, Clone, Default)]
#[cfg_attr(feature = "serde_support", derive(Serialize, Deserialize))]
pub struct Ability {
    #[cfg_attr(feature = "serde_support", serde(default = "default_empty_string"))]
    pub full_text: String,
    /// Trigger prefix stripped from `full_text` (e.g. "【自】"). Usually derived
    /// on demand from `full_text`; only populated directly when the source JSON
    /// carries an explicit, non-derivable value. `None` means "derive from
    /// `full_text`" — see `Ability::triggerless_text`.
    #[cfg_attr(
        feature = "serde_support",
        serde(default, skip_serializing_if = "Option::is_none")
    )]
    pub triggerless_text: Option<String>,
    pub triggers: Option<ArcStr>,
    pub use_limit: Option<u8>,
    #[cfg_attr(feature = "serde_support", serde(default))]
    pub is_null: bool,
    pub cost: Option<Box<AbilityCost>>,
    pub effect: Option<Box<AbilityEffect>>,
    pub keywords: Option<Vec<Keyword>>,
    /// Precomputed [`TriggerMask`] for `triggers`, parsed once on
    /// first use and cached here. Decoded abilities live behind
    /// process-global `OnceLock` slots (`ability_store`), so the
    /// parse runs at most once per ability index for the whole
    /// process — every trigger scan after the first tests a bit
    /// instead of re-splitting the `triggers` text
    /// (docs/PERF_ENGINE_OPTIMIZATION_CATALOG.md §4.11). A pure
    /// function of `triggers`, so it is serde-skipped: never a
    /// wire field.
    #[cfg(not(feature = "no_std"))]
    #[cfg_attr(feature = "serde_support", serde(skip))]
    pub(crate) trigger_mask: std::sync::OnceLock<crate::triggers::TriggerMask>,
    /// Precomputed [`TriggerPositionMask`] for the position
    /// keywords in `triggers` — same caching rationale as
    /// `trigger_mask`, for the action-generation position gate.
    #[cfg(not(feature = "no_std"))]
    #[cfg_attr(feature = "serde_support", serde(skip))]
    pub(crate) trigger_position_mask: std::sync::OnceLock<crate::zones::TriggerPositionMask>,
}

// PartialEq/Eq are manual: the two mask fields are caches of pure
// functions of `triggers`, and a derived impl would compare the
// OnceLock contents — making two otherwise-identical abilities
// unequal when only one has had its mask computed.
impl PartialEq for Ability {
    fn eq(&self, other: &Self) -> bool {
        self.full_text == other.full_text
            && self.triggerless_text == other.triggerless_text
            && self.triggers == other.triggers
            && self.use_limit == other.use_limit
            && self.is_null == other.is_null
            && self.cost == other.cost
            && self.effect == other.effect
            && self.keywords == other.keywords
    }
}
impl Eq for Ability {}

#[cfg(feature = "serde_support")]
fn default_empty_string() -> String {
    String::new()
}

impl Ability {
    /// The precomputed trigger mask for this ability's `triggers`
    /// text, parsed on first call and cached in the struct. On
    /// `no_std` builds there is no cache; `has_trigger` parses
    /// directly.
    #[cfg(not(feature = "no_std"))]
    pub fn trigger_mask(&self) -> crate::triggers::TriggerMask {
        *self.trigger_mask.get_or_init(|| {
            crate::triggers::TriggerMask::parse(self.triggers.as_deref().unwrap_or(""))
        })
    }

    /// The precomputed stage-position mask for the position
    /// keywords in this ability's `triggers` text — the cached
    /// form of the scan `check_trigger_position` performs.
    #[cfg(not(feature = "no_std"))]
    pub fn trigger_position_mask(&self) -> crate::zones::TriggerPositionMask {
        *self.trigger_position_mask.get_or_init(|| {
            crate::zones::TriggerPositionMask::parse(self.triggers.as_deref().unwrap_or(""))
        })
    }

    /// Does this ability carry the given trigger kind? The single
    /// source of truth for trigger matching across resolver,
    /// trigger scans, and UI. Replaces scattered substring
    /// `contains()` checks.
    ///
    /// Tests the precomputed [`TriggerMask`] (one bitwise AND)
    /// instead of re-parsing the `triggers` text on every call;
    /// the mask is parsed once per ability and cached, and
    /// decoded abilities are cached process-globally, so the
    /// parse happens at most once per ability index per process.
    pub fn has_trigger(&self, kind: crate::triggers::TriggerKind) -> bool {
        #[cfg(not(feature = "no_std"))]
        {
            self.trigger_mask().contains(kind)
        }
        #[cfg(feature = "no_std")]
        {
            self.triggers
                .as_deref()
                .is_some_and(|t| crate::triggers::parse_triggers(t).any(|k| k == kind))
        }
    }

    /// Whether a card in `area` may activate this ability under
    /// the position keywords embedded in its trigger text — the
    /// cached form of [`crate::zones::check_trigger_position`],
    /// which action generation called per stage card.
    pub fn trigger_position_allows(
        &self,
        area: crate::zones::MemberArea,
    ) -> bool {
        #[cfg(not(feature = "no_std"))]
        {
            self.trigger_position_mask().allows(area)
        }
        #[cfg(feature = "no_std")]
        {
            crate::zones::check_trigger_position(self.triggers.as_deref(), area)
        }
    }

    /// Return the text with any leading trigger clause (e.g. `【自】`) stripped.
    /// When `triggerless_text` was explicitly set it is returned directly;
    /// otherwise it is derived from `full_text` on demand.
    pub fn triggerless_text(&self) -> &str {
        match &self.triggerless_text {
            Some(t) => t.as_str(),
            None => {
                let ft = self.full_text.trim_start();
                if let Some(rest) = ft.strip_prefix("【") {
                    if let Some(idx) = rest.find('】') {
                        return &ft[idx + '】'.len_utf8()..];
                    }
                }
                ft
            }
        }
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
#[cfg_attr(feature = "serde_support", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde_support", serde(transparent))]
pub struct AbilityCost(pub AbilityEffect);

impl From<AbilityCost> for AbilityEffect {
    fn from(cost: AbilityCost) -> Self {
        cost.0
    }
}

impl From<AbilityEffect> for AbilityCost {
    fn from(effect: AbilityEffect) -> Self {
        AbilityCost(effect)
    }
}

impl core::ops::Deref for AbilityCost {
    type Target = AbilityEffect;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl core::ops::DerefMut for AbilityCost {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

/// True when an exclusion list is exactly the one group an inclusion names.
///
/// Both spellings can occur in the corpus, so the comparison is by the same
/// whitespace-and-micro-sign normalisation `card_matches_group_str` uses rather than by
/// string equality.
fn excluded_matches(excluded: &[String], included: &str) -> bool {
    excluded.len() == 1 && excluded[0] == included
}

impl AbilityCost {
    /// Borrow the compound branch of the wrapped effect, if any.
    ///
    /// `AbilityCost` is a newtype over `AbilityEffect`; this forwards
    /// [`AbilityEffect::compound_branch`] so cost handlers can read the branch
    /// without reaching through `.0`.
    pub fn compound_branch(&self) -> Option<&CompoundBranch> {
        self.0.compound_branch()
    }

    /// Mutably borrow the compound branch, creating it if absent.
    pub fn compound_branch_mut(&mut self) -> &mut CompoundBranch {
        self.0.compound_branch_mut()
    }

    /// Forward of [`AbilityEffect::actions`] for cost handlers.
    pub fn actions(&self) -> Option<&Vec<Box<AbilityEffect>>> {
        self.0.actions()
    }
    /// Build a `CardFilter` containing the same 7 base filter fields that
    /// `AbilityEffect::filter_subset` exposes. Mirrors that method so cost
    /// handlers can use the same consolidation pattern as effect handlers.
    pub fn filter_subset(&self) -> crate::ability::util::CardFilter<'_> {
        // Single filter builder: delegate to CardFilter::from_effect so the
        // full field set (characters, need_heart_*, cost_total, distinct, ...)
        // is always honored. Previously this was a hand-maintained minimal
        // subset that silently dropped fields (e.g. `characters`), causing
        // condition/effect mismatches (CLEAN-G15/D20).
        crate::ability::util::CardFilter::from_effect(self)
    }
}

/// Grouped sub-effect fields used by compound action handlers
/// (Sequential, ConditionalAlternative, ConditionalOnResult, ConditionalOnOptional, LookAndSelect).
/// Flattened into AbilityEffect via `#[serde(flatten)]` for JSON backward compat.
///
/// The 4 specialized compound shapes (look_and_select, conditional_alternative,
/// conditional_on_result, conditional_on_optional) are all normalized into the
/// unified `effect_steps` form by the engine on dispatch. The legacy fields
/// (look_action/select_action/primary_effect/...) are kept here for backward
/// compatibility with previously-generated `abilities.json` files; new
/// parsers should emit only `effect_steps`.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
#[cfg_attr(feature = "serde_support", derive(Serialize, Deserialize))]
pub struct CompoundBranch {
    #[cfg_attr(feature = "serde_support", serde(default))]
    pub look_action: Option<Box<AbilityEffect>>,
    #[cfg_attr(feature = "serde_support", serde(default))]
    pub select_action: Option<Box<AbilityEffect>>,
    #[cfg_attr(feature = "serde_support", serde(default))]
    pub actions: Option<Vec<Box<AbilityEffect>>>,
    #[cfg_attr(feature = "serde_support", serde(default))]
    pub primary_effect: Option<Box<AbilityEffect>>,
    #[cfg_attr(feature = "serde_support", serde(default))]
    pub alternative_condition: Option<Arc<Condition>>,
    #[cfg_attr(feature = "serde_support", serde(default))]
    pub result_condition: Option<Arc<Condition>>,
    #[cfg_attr(feature = "serde_support", serde(default))]
    pub followup_action: Option<Box<AbilityEffect>>,
    #[cfg_attr(feature = "serde_support", serde(default))]
    pub optional_action: Option<Box<AbilityEffect>>,
    #[cfg_attr(feature = "serde_support", serde(default))]
    pub conditional_action: Option<Box<AbilityEffect>>,
    #[cfg_attr(feature = "serde_support", serde(default))]
    pub conditional_negation: Option<bool>,
}

/// One branch of an OR'd ability filter. At least one branch must match
/// for the card to pass.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
#[cfg_attr(feature = "serde_support", derive(Serialize, Deserialize))]
pub struct AbilityFilterBranch {
    pub ability_filter: Option<AbilityFilter>,
    pub ability_filter_triggers: Option<Vec<String>>,
}

/// Shared filter/targeting fields extracted from EffectKind variants.
/// Boxed into each variant to reduce enum size from ~544 to ~140 bytes.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
#[cfg_attr(feature = "serde_support", derive(Serialize))]
pub struct EffectFilter {
    pub card_type: Option<CardType>,
    pub exclude_self: Option<bool>,
    pub same_name: Option<bool>,
    pub same_unit_name: Option<bool>,
    pub group_names: Option<Box<Vec<String>>>,
    /// The groups whose effects see a `count_as_multiple` weight
    /// (PL!-pb2-041-L: 「自分の『lily white』のカードの効果によって…」). This is the
    /// OPPOSITE direction to `group_names` and must never be used as a card filter.
    pub counted_by: Option<Box<Vec<String>>>,
    /// The weight itself for `count_as_multiple` — 「このカードは2枚として扱う」.
    pub count_as: Option<u8>,
    pub self_target: Option<bool>,
    /// "Under THIS member" for under_member placement (e.g. "…をこのメンバーの
    /// 下に置く"). Distinct from `self_target` (which has source-filtering
    /// semantics). When set, the player does NOT choose which stage member
    /// receives the card — it auto-places under the activating member.
    pub under_self: Option<bool>,
    pub location: Option<ArcStr>,
    pub heart_colors: Box<Vec<String>>,
    pub source: Option<Zone>,
    pub target: Option<ArcStr>,
    pub destination: Option<Zone>,
    pub characters: Option<Box<Vec<String>>>,
    pub exclude_characters: Option<Box<Vec<String>>>,
    pub exclude_group_names: Option<Box<Vec<String>>>,
    pub activation_position: Option<ArcStr>,
    pub original_value: Option<bool>,
    /// 「元々持つハートの数がNつ以下」 — bound on the SUM of a card's printed hearts.
    /// Static card data, so unlike a current-blade bound it is evaluated inside
    /// `CardFilter::matches`. Defaults to `<=` when the operator is absent, since
    /// 「Nつ以下」 is the phrasing this exists for.
    pub heart_total_limit: Option<u8>,
    pub heart_total_operator: Option<ArcStr>,
    pub target_count: Option<u8>,
    pub per_unit: Option<bool>,
    pub per_unit_count: Option<u8>,
    pub per_unit_type: Option<ArcStr>,
    /// When set, the per-unit count is an aggregate over the matching cards rather
    /// than a tally of them. `"score"` = the sum of the cards' printed scores, for
    /// 「…のスコアの合計Nにつき」. `per_unit_type` still names the zone; only the
    /// measured quantity changes, so the two are kept apart.
    pub per_unit_aggregate: Option<ArcStr>,
    /// What the per-unit count is measured against (e.g. "previous_moved_cards"
    /// = cards moved by the preceding action step). Without this the generic
    /// per-unit branch wrongly counts the default location instead.
    pub per_unit_source: Option<ArcStr>,
    pub group_reference: Option<ArcStr>,
    pub state: Option<Box<EffectState>>,
    pub distinct: Option<Box<DistinctType>>,
    pub position: Option<Box<PositionInfo>>,
    pub negation: Option<bool>,
    pub per_unit_heart_colors: Box<Vec<String>>,
    pub cost_limit: Option<u8>,
    pub cost_limit_operator: Option<Operator>,
    /// Discrete set of allowed cost values (OR) — e.g. "コストが10か20" → [10, 20].
    #[cfg_attr(feature = "serde_support", serde(default))]
    pub cost_values: Option<Vec<u8>>,
    /// Blade-count target filter (e.g. "ブレードを4つ以上持つ"). Whether it
    /// means the printed/original value or the CURRENT total is decided by the
    /// effect's `original_value` flag when building a CardFilter.
    pub blade_limit: Option<u8>,
    pub blade_limit_operator: Option<Operator>,
    /// Dynamic blade limit: when set, the effective blade_limit is
    /// (energy cards under the activating member) + blade_limit_offset (C5).
    pub blade_limit_from_energy_under: Option<bool>,
    /// Dynamic blade limit referenced to the member paid as a wait cost:
    /// effective blade_limit = (original blade of the member waited as the cost) + offset.
    pub blade_limit_from_cost_member: Option<bool>,
    /// The addend used when `blade_limit_from_energy_under` is set (the
    /// "1を足した数" +N in the text), so the JSON makes the +N explicit.
    pub blade_limit_offset: Option<u8>,
    /// C6 keep-N-shuffle-rest: both players each select up to `count` hand
    /// cards to keep; the OTHER hand cards shuffle under their own deck.
    pub keep_shuffle_under: Option<bool>,
    pub need_heart_color: Option<ArcStr>,
    pub need_heart_operator: Option<Operator>,
    pub need_heart_total: Option<u8>,
    pub cost_reference: Option<ArcStr>,
    pub cost_offset: Option<i8>,
    pub target_member: Option<ArcStr>,
    pub target_from_selection: Option<bool>,
    pub source_position: Option<ArcStr>,
    /// Cross-position comparison reference (e.g. "right_side"). The parser
    /// echoes the condition-level `position_compare` on effect objects; the
    /// condition side carries the evaluated value. Decoded here so bytecode
    /// stops warning; no executor reads the effect copy yet.
    pub position_compare: Option<ArcStr>,
    /// Origin of a gained ability (corpus: only "under_member"). The
    /// gain_ability_from_source executor hardcodes the under-member lookup,
    /// which matches every corpus value — stored for visibility so a future
    /// new value warns loudly instead of dropping silently.
    pub source_location: Option<ArcStr>,
    pub exclude_selected: Option<bool>,
    pub discard_remaining: Option<bool>,
    pub self_cost: Option<bool>,
    pub shuffle: Option<bool>,
    pub effect_constraint: Option<ArcStr>,
    pub score_floor: Option<u8>,
    pub sign: Option<ArcStr>,
    pub heart_type: Option<ArcStr>,
    pub alternative_count_type: Option<ArcStr>,
    pub blind: Option<bool>,
    pub picker: Option<ArcStr>,
    pub reveal: Option<bool>,
    pub choice_type: Option<ArcStr>,
    pub choice_options: Option<Box<Vec<String>>>,
    pub replaces_event: Option<ArcStr>,
    pub choice_based: Option<bool>,
    pub use_limit: Option<u8>,
    pub triggers: Option<ArcStr>,
    pub original_count: Option<u8>,
    pub original_operator: Option<Operator>,
    pub quoted_text: Option<Box<QuotedText>>,
    pub allow_occupied_stage: Option<bool>,
    pub state_change: Option<Box<EffectState>>,
    pub exclude_position: Option<ArcStr>,
    pub duration: Option<ArcStr>,
    pub dynamic_count: Option<Box<DynamicCount>>,
    pub filter_targets_by_heart_colors: Option<bool>,
    pub card_property: Option<ArcStr>,
    pub per_unit_location: Option<ArcStr>,
    pub card_names: Box<Vec<String>>,
    pub all: Option<bool>,
    pub requires_under_card: Option<bool>,
    pub optional: Option<bool>,
    pub cost_total: Option<u8>,
    pub cost_total_operator: Option<Operator>,
    pub activation_condition_parsed: Option<Box<Condition>>,
    pub action_by: Option<ArcStr>,
    pub trigger_type: Option<ArcStr>,
    pub repeat_limit: Option<u8>,
    pub ability_filter: Option<AbilityFilter>,
    pub multiple_targets: Option<bool>,
    pub operation: Option<Operation>,
    pub options: Option<Box<Vec<Box<AbilityEffect>>>>,
    pub name_constraint: Option<ArcStr>,
    pub name_constraint_source: Option<ArcStr>,
    pub ability_filter_triggers: Option<Box<Vec<String>>>,
    pub or_ability_filters: Option<Box<Vec<AbilityFilterBranch>>>,
    pub energy_count: Option<u8>,
    pub any_number: Option<bool>,
    pub custom_type: Option<ArcStr>,
    pub yell_source: Option<ArcStr>,
    pub require_all_heart_colors: Option<bool>,
    pub heart_color_count: Option<u8>,
    pub value: Option<u8>,
    pub per_group: Option<bool>,
    pub per_group_count: Option<u8>,
    /// Cost requires one card PER named character (e.g. LL-bp7-001's three
    /// names) rather than `count` cards matching any of them. The play-time
    /// alt-cost hook (phases.rs `play_time_alt_cost_chars`) enforces this via
    /// the `characters` list with distinct assignment.
    pub per_character: Option<bool>,
    pub placement_order: Option<PlacementOrder>,
    pub remainder_destination: Option<ArcStr>,
    pub remainder_placement_order: Option<PlacementOrder>,
    pub or_card_types: Option<Box<Vec<String>>>,
    pub exclude_heart_colors: Option<Box<Vec<String>>>,
    pub cost_from_revealed: Option<bool>,
    pub timing_condition: Option<ArcStr>,
    pub identities: Option<Box<Vec<String>>>,
    pub all_regions: Option<bool>,
    pub trigger_filter: Option<Box<Vec<String>>>,
    pub effect_type: Option<ArcStr>,
    pub timing: Option<ArcStr>,
    pub treat_as: Option<ArcStr>,
    pub question: Option<ArcStr>,
    pub answers: Option<Box<Vec<String>>>,
    pub choice_maker: Option<ArcStr>,
    pub cost_limit_min: Option<u8>,
    pub cost_limit_max: Option<u8>,
    pub count: Option<u8>,
    pub exclude_by_name_source: Option<ArcStr>,
    pub baton_touch_trigger: Option<bool>,
    pub is_reveal: Option<bool>,
    pub resource_on_select: Option<Box<AbilityEffect>>,
    pub replace_all: Option<bool>,
    pub resource: Option<ArcStr>,
    pub heart_colors_from_selected_card: Option<bool>,
    pub heart_color: Option<ArcStr>,
    pub ability_gain: Option<ArcStr>,
    pub ability_gain_trigger: Option<ArcStr>,
    /// Anaphora reference: "cost_waited" for 「これによってウェイト状態になった
    /// メンバーは…」 (points at members put to wait by this ability's cost).
    pub anaphora: Option<ArcStr>,
    pub gained_effect: Option<Box<AbilityEffect>>,
    pub ability_text: Option<ArcStr>,
    pub target_trigger: Option<ArcStr>,
    pub source_card: Option<ArcStr>,
    pub suppressed_trigger: Option<ArcStr>,
    pub option: Option<ArcStr>,
    pub alternative_effect: Option<Box<AbilityEffect>>,
    pub choice_condition: Option<Arc<Condition>>,
    pub alternative_condition: Option<Arc<Condition>>,
    pub restriction_type: Option<ArcStr>,
    pub restricted_destination: Option<ArcStr>,
    pub delayed: Option<bool>,
    pub phase: Option<ArcStr>,
    pub non_stackable: Option<bool>,
    pub heart_selection: Option<bool>,
    pub blade_type: Option<ArcStr>,
    pub choice: Option<bool>,
    pub lose_blade_hearts: Option<bool>,
    pub original_cost: Option<u8>,
    pub parenthetical: Option<Box<Vec<String>>>,
    pub resource_icon_count: Option<u8>,
    pub ref_value: Option<ArcStr>,
    pub ref_offset: Option<i8>,
    pub id: Option<ArcStr>,
    pub opponent_action: Option<Box<AbilityEffect>>,

    /// 「このターン中に…アクティブにしていた」 — a per-turn HISTORY clause, not a
    /// state filter. The counted set is GameState::cards_activated_this_turn (members
    /// that were in wait and have been activated this turn), filtered to location
    /// and group_names — NOT the set of members currently in wait.
    /// PL!-pb2-010-P＋ 綎滄絵針.
    pub activated_this_turn: Option<bool>,
    /// 「このメンバーが、自分のカードの能力のコストを支払うため」 — also a per-turn HISTORY
    /// clause. The set is GameState::cards_paid_cost_this_turn: the members that have PAID
    /// the cost of one of their own abilities this turn. It is not a comparison on cost, and
    /// not a state — which is why the parser emitting a bare `comparison_condition` here made
    /// it a tautology. PL!-pb2-021-N 南ことり.
    pub paid_cost_this_turn: Option<bool>,
}

/// Tagged union of effect action types. Every field lives on the shared
/// `EffectFilter`; each variant is just a marker carrying the filter box so the
/// action-type dispatch in `filter()`/`filter_mut()` keeps working.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
#[cfg_attr(feature = "serde_support", derive(Serialize))]
pub enum EffectKind {
    #[default]
    None,
    MoveCards {
        filter: Option<Box<EffectFilter>>,
    },
    DrawCards {
        filter: Option<Box<EffectFilter>>,
    },
    SelectTarget {
        filter: Option<Box<EffectFilter>>,
    },
    LookReveal {
        filter: Option<Box<EffectFilter>>,
    },
    ModifyScore {
        filter: Option<Box<EffectFilter>>,
    },
    ModifyHearts {
        filter: Option<Box<EffectFilter>>,
    },
    GainResource {
        filter: Option<Box<EffectFilter>>,
    },
    ChangeState {
        filter: Option<Box<EffectFilter>>,
    },
    AbilityOp {
        filter: Option<Box<EffectFilter>>,
    },
    CompoundEffect {
        filter: Option<Box<EffectFilter>>,
    },
    RestrictionOp {
        filter: Option<Box<EffectFilter>>,
    },
    PositionOp {
        filter: Option<Box<EffectFilter>>,
    },
    MiscOp {
        filter: Option<Box<EffectFilter>>,
    },
    CustomOp {
        filter: Option<Box<EffectFilter>>,
    },
}

impl EffectKind {
    pub(crate) fn filter(&self) -> Option<&EffectFilter> {
        match self {
            EffectKind::None => None,
            EffectKind::MoveCards { filter, .. }
            | EffectKind::DrawCards { filter, .. }
            | EffectKind::SelectTarget { filter, .. }
            | EffectKind::LookReveal { filter, .. }
            | EffectKind::ModifyScore { filter, .. }
            | EffectKind::ModifyHearts { filter, .. }
            | EffectKind::GainResource { filter, .. }
            | EffectKind::ChangeState { filter, .. }
            | EffectKind::AbilityOp { filter, .. }
            | EffectKind::CompoundEffect { filter, .. }
            | EffectKind::RestrictionOp { filter, .. }
            | EffectKind::PositionOp { filter, .. }
            | EffectKind::MiscOp { filter, .. }
            | EffectKind::CustomOp { filter, .. } => filter.as_deref(),
        }
    }

    pub(crate) fn filter_mut(&mut self) -> Option<&mut EffectFilter> {
        match self {
            EffectKind::None => None,
            EffectKind::MoveCards { filter, .. }
            | EffectKind::DrawCards { filter, .. }
            | EffectKind::SelectTarget { filter, .. }
            | EffectKind::LookReveal { filter, .. }
            | EffectKind::ModifyScore { filter, .. }
            | EffectKind::ModifyHearts { filter, .. }
            | EffectKind::GainResource { filter, .. }
            | EffectKind::ChangeState { filter, .. }
            | EffectKind::AbilityOp { filter, .. }
            | EffectKind::CompoundEffect { filter, .. }
            | EffectKind::RestrictionOp { filter, .. }
            | EffectKind::PositionOp { filter, .. }
            | EffectKind::MiscOp { filter, .. }
            | EffectKind::CustomOp { filter, .. } => filter.as_deref_mut(),
        }
    }

    /// Single source of truth for the action → `EffectKind` mapping. Both the
    /// bytecode decoder and the JSON deep-compare path build an `EffectKind`
    /// from an action string + a pre-built filter via this function, so the
    /// variant-selection logic lives in exactly one place.
    ///
    /// Returns `None` for action strings that don't map to a known variant
    /// (the JSON path treats these as a decode failure).
    pub(crate) fn from_action(
        action: &str,
        filter: Option<Box<EffectFilter>>,
    ) -> Option<EffectKind> {
        // Every real caller passes `ActionType::to_str()`, whose wire
        // strings are canonical lowercase snake_case, so the match below
        // hits without allocating. The lowercase copy is built only when
        // the input actually contains a character whose lowercase mapping
        // differs from itself — the exact condition under which the old
        // unconditional `to_lowercase()` changed the string — so which
        // inputs match is unchanged, but the decode path no longer pays
        // a `String` allocation per effect.
        let lowered;
        let a: &str = if action.chars().all(|c| c.to_lowercase().eq([c])) {
            action
        } else {
            lowered = action.to_lowercase();
            lowered.as_str()
        };
        Some(match a {
            "move_cards"
            | "discard_card"
            | "discard_until_count"
            | "place_energy_under_member"
            | "re_yell"
            | "shuffle"
            | "play_baton_touch"
            | "double_baton_touch" => EffectKind::MoveCards { filter },
            "draw" | "draw_card" | "draw_until_count" => EffectKind::DrawCards { filter },
            "select" | "select_cards" | "select_number" | "choose_target_player" => {
                EffectKind::SelectTarget { filter }
            }
            "look"
            | "look_at"
            | "reveal"
            | "reveal_effect"
            | "reveal_per_group"
            | "reveal_until_live_card"
            | "reveal_until_chosen_card"
            | "look_and_select" => EffectKind::LookReveal { filter },
            "modify_score" => EffectKind::ModifyScore { filter },
            "modify_required_hearts"
            | "modify_required_hearts_global"
            | "modify_required_hearts_success" => EffectKind::ModifyHearts { filter },
            "gain_resource" | "pay_energy" => EffectKind::GainResource { filter },
            "change_state"
            | "set_card_identity"
            | "set_card_identity_all_regions"
            // A replacement appearance rule is a state change TO WAIT carrying the
            // same filters (card_type, position, heart/blade bound), so it reuses the
            // ChangeState variant. Without an entry here the filter is dropped during
            // `populate_from_json`, and the rule silently matches nothing — the
            // ability still decodes and still reports its action, so nothing errors.
            | "replacement_appear_wait"
            // A counting weight carries the same filter shape (its own zone condition
            // and the `counted_by` group list), so it rides the same variant. Neither
            // action is dispatched — both are consumed elsewhere.
            | "count_as_multiple" => {
                EffectKind::ChangeState { filter }
            }
            "gain_ability"
            | "gain_ability_from_source"
            | "invalidate_ability"
            | "suppress_ability_trigger"
            | "activate_ability" => EffectKind::AbilityOp { filter },
            "sequential"
            | "sequential_cost"
            | "choice"
            | "choice_condition"
            | "repeat_procedure"
            | "conditional_alternative"
            | "conditional_on_optional"
            | "conditional_on_result" => EffectKind::CompoundEffect { filter },
            "restriction"
            | "activation_restriction"
            | "modify_limit"
            | "all_blade_timing"
            | "reduce_live_card_set_limit" => EffectKind::RestrictionOp { filter },
            "position_change" | "rotation" => EffectKind::PositionOp { filter },
            "set_cost"
            | "set_cost_to_use"
            | "modify_cost"
            | "activation_cost"
            | "set_blade_type"
            | "set_blade_count"
            | "set_heart_type"
            | "specify_heart_color"
            | "choose_required_hearts"
            | "perform_yell"
            | "modify_yell_count" => EffectKind::MiscOp { filter },
            "custom" | "do_nothing" | "action_by" | "opponent_action" | "modify_yell_source" => {
                EffectKind::CustomOp { filter }
            }
            "" => EffectKind::SelectTarget { filter },
            _ => return None,
        })
    }
}

/// Recursively re-populate EffectKind for a serialization-deserialized
/// AbilityEffect that lost its `kind` (because kind is #[serde(skip)]).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
#[cfg_attr(feature = "serde_support", derive(Serialize, Deserialize))]
pub struct AbilityEffect {
    #[cfg_attr(feature = "serde_support", serde(default))]
    pub text: ArcStr,
    #[cfg_attr(feature = "serde_support", serde(default))]
    pub action: ActionType,
    #[cfg_attr(feature = "serde_support", serde(default))]
    pub source: Option<Zone>,
    #[cfg_attr(feature = "serde_support", serde(default))]
    pub destination: Option<Zone>,
    #[cfg_attr(feature = "serde_support", serde(default))]
    pub count: Option<u8>,
    #[cfg_attr(feature = "serde_support", serde(default))]
    pub target: Option<ArcStr>,
    #[cfg_attr(feature = "serde_support", serde(default))]
    pub condition: Option<Arc<Condition>>,
    #[cfg_attr(feature = "serde_support", serde(default, flatten))]
    pub compound: Option<Box<CompoundBranch>>,
    #[cfg_attr(feature = "serde_support", serde(skip))]
    pub kind: Option<EkBox>,
    pub non_stackable: Option<bool>,
    #[cfg_attr(feature = "serde_support", serde(default))]
    pub conditional: Option<bool>,
    #[cfg_attr(feature = "serde_support", serde(default))]
    pub is_further: Option<bool>,
    #[cfg_attr(feature = "serde_support", serde(default))]
    pub optional: Option<bool>,
    #[cfg_attr(feature = "serde_support", serde(default))]
    pub max: Option<bool>,
    #[cfg_attr(feature = "serde_support", serde(default))]
    pub effect_steps: Option<Vec<Box<AbilityEffect>>>,
    /// Parsed cost-reduction clause on a pay_energy cost (e.g. 海未 bp5-004:
    /// "グループ名1種類につき、E減る"). Presence means the printed energy is
    /// an upper bound — affordability must not hard-block offers.
    #[cfg_attr(feature = "serde_support", serde(default))]
    pub cost_reduction_per_group: Option<u8>,
    /// Trigger clause watches an ability resolution
    /// (「…能力が解決したとき/解決するたび」), stamped by the parser.
    #[cfg_attr(feature = "serde_support", serde(default))]
    pub watches_ability_resolution: Option<bool>,
    /// 「(対戦相手のカードの効果でも発動する。)」 — stamped by the parser
    /// when the effect carries that parenthetical; the engine reads the
    /// bool instead of re-matching JP text.
    #[cfg_attr(feature = "serde_support", serde(default))]
    pub fires_on_opponent_effects: Option<bool>,
}

impl AbilityEffect {
    /// 「(対戦相手のカードの効果でも発動する。)」 — this watcher also arms on
    /// trigger events caused by the OPPOSING player's card effects. Without
    /// the marker, a movement-watching 自動 fires on own-side causes only.
    ///
    /// Value is stamped by the parser into `fires_on_opponent_effects`
    /// (see `GameState::fire_opponent_cause_watchers_for_move`).
    pub fn fires_on_opponent_effects(&self) -> bool {
        self.fires_on_opponent_effects.unwrap_or(false)
    }

    /// Total ACTIVE-ENERGY cost of this cost block.
    ///
    /// Single source of truth for affordability (generation filter and the
    /// execution pre-check must never diverge). Rules:
    ///   - Only PayEnergy steps carry energy. `count` on other step kinds is
    ///     a TARGET quantity ("mill 5", "look at 3") — reading it as energy
    ///     blocks free abilities like 桜坂しずく's mill-5 (bp7-003).
    ///   - Sequential/compound costs: sum the pay_energy sub-steps
    ///     (e.g. 村野さやか bp1-002: [PayEnergy 2, MoveCards self→waitroom]).
    pub fn energy_cost_total(&self) -> u8 {
        if self.action == crate::ability::enums::ActionType::PayEnergy {
            return self.energy_count_any().or(self.count).unwrap_or(0);
        }
        self.compound_branch()
            .and_then(|c| c.actions.as_deref())
            .unwrap_or(&[])
            .iter()
            .fold(0u8, |acc, s| acc.saturating_add(s.energy_cost_total()))
    }

    /// True when ANY pay_energy component is optional/skippable — those must
    /// stay offered even when energy is short (resolution skips just the
    /// effect, e.g. wakana bp2-008).
    pub fn has_optional_payment(&self) -> bool {
        if self.action == crate::ability::enums::ActionType::PayEnergy
            && self.optional == Some(true)
        {
            return true;
        }
        self.compound_branch()
            .and_then(|c| c.actions.as_deref())
            .unwrap_or(&[])
            .iter()
            .any(|s| s.has_optional_payment())
    }

    /// Effective ACTIVE-energy cost of this cost block given
    /// `groups_on_stage` distinct group names among the activator's stage
    /// members. Single source of truth for affordability AND display:
    /// each pay_energy step subtracts its own `cost_reduction_per_group`
    /// × groups, clamped at zero per step (海未 bp5-004 Q228: 4E − groups×1,
    /// can reach 0).
    pub fn effective_energy_cost_total(&self, groups_on_stage: u8) -> u8 {
        if self.action == crate::ability::enums::ActionType::PayEnergy {
            let printed = self.energy_count_any().or(self.count).unwrap_or(0);
            let reduction = self
                .cost_reduction_per_group
                .unwrap_or(0)
                .saturating_mul(groups_on_stage);
            return printed.saturating_sub(reduction);
        }
        self.compound_branch()
            .and_then(|c| c.actions.as_deref())
            .unwrap_or(&[])
            .iter()
            .fold(0u8, |acc, s| {
                acc.saturating_add(s.effective_energy_cost_total(groups_on_stage))
            })
    }

    /// Borrow the compound branch, if any.
    ///
    /// `compound` is now `Option<Box<CompoundBranch>>` (the unconditional `Box`
    /// was a guaranteed heap allocation on every `AbilityEffect` clone, and
    /// ~95% of effects have no compound branch at all). Callers that used to
    /// read `self.compound.<field>` go through this so the `Option` is unwound
    /// in exactly one place rather than at 14 call sites.
    pub fn compound_branch(&self) -> Option<&CompoundBranch> {
        self.compound.as_deref()
    }

    /// Mutably borrow the compound branch, creating it if absent.
    ///
    /// The mirror of [`Self::compound_branch`] for callers that need to write
    /// into the branch (e.g. clearing `primary_effect` mid-resolution). The `Box`
    /// allocation is unavoidable here — a compound branch that didn't exist
    /// before is being created — but it only happens on the rare write paths,
    /// never on the read-heavy hot loop.
    pub fn compound_branch_mut(&mut self) -> &mut CompoundBranch {
        self.compound.get_or_insert_with(|| Box::new(CompoundBranch::default()))
    }

    /// Read-only accessors for each compound branch field, unwinding the
    /// `Option<Box<CompoundBranch>>` once. Mirrors [`Self::compound_branch`] but
    /// returns the field directly so call sites stay one-liners instead of
    /// `compound_branch().and_then(|c| c.X)`. The `Box` allocation is skipped on the
    /// common case where the effect has no compound branch at all.
    #[inline]
    pub fn look_action(&self) -> Option<&Box<AbilityEffect>> {
        self.compound.as_deref().and_then(|c| c.look_action.as_ref())
    }
    #[inline]
    pub fn select_action(&self) -> Option<&Box<AbilityEffect>> {
        self.compound.as_deref().and_then(|c| c.select_action.as_ref())
    }
    #[inline]
    pub fn actions(&self) -> Option<&Vec<Box<AbilityEffect>>> {
        self.compound.as_deref().and_then(|c| c.actions.as_ref())
    }
    #[inline]
    pub fn primary_effect(&self) -> Option<&Box<AbilityEffect>> {
        self.compound.as_deref().and_then(|c| c.primary_effect.as_ref())
    }
    #[inline]
    pub fn alternative_condition(&self) -> Option<&Arc<Condition>> {
        self.compound.as_deref().and_then(|c| c.alternative_condition.as_ref())
    }
    #[inline]
    pub fn result_condition(&self) -> Option<&Arc<Condition>> {
        self.compound.as_deref().and_then(|c| c.result_condition.as_ref())
    }
    #[inline]
    pub fn followup_action(&self) -> Option<&Box<AbilityEffect>> {
        self.compound.as_deref().and_then(|c| c.followup_action.as_ref())
    }
    #[inline]
    pub fn optional_action(&self) -> Option<&Box<AbilityEffect>> {
        self.compound.as_deref().and_then(|c| c.optional_action.as_ref())
    }
    #[inline]
    pub fn conditional_action(&self) -> Option<&Box<AbilityEffect>> {
        self.compound.as_deref().and_then(|c| c.conditional_action.as_ref())
    }
    #[inline]
    pub fn conditional_negation(&self) -> Option<bool> {
        self.compound.as_deref().and_then(|c| c.conditional_negation)
    }

    /// Build EffectKind from an action string and the matching effect JSON.
    /// Constructs EffectKind directly without serde Deserialize.
    #[cfg(feature = "json_path_test")]
    pub(crate) fn kind_from_action(
        action: &str,
        effect_json: &serde_json::Value,
    ) -> Option<EffectKind> {
        let obj = effect_json.as_object()?;
        macro_rules! str_field {
            ($key:expr) => {
                obj.get($key)
                    .and_then(|v| v.as_str())
                    .map(|s| ArcStr::from(s))
            };
        }
        macro_rules! zone_field {
            ($key:expr) => {
                obj.get($key)
                    .and_then(|v| v.as_str())
                    .map(Zone::from_source_str)
            };
        }
        macro_rules! bool_field {
            ($key:expr) => {
                obj.get($key).and_then(|v| v.as_bool())
            };
        }
        macro_rules! u8_field {
            ($key:expr) => {
                obj.get($key)
                    .and_then(|v| v.as_u64().and_then(|n| u8::try_from(n).ok()))
            };
        }
        macro_rules! i8_field {
            ($key:expr) => {
                obj.get($key)
                    .and_then(|v| v.as_i64().and_then(|n| i8::try_from(n).ok()))
            };
        }
        macro_rules! str_vec_field {
            ($key:expr) => {
                obj.get($key).and_then(|v| v.as_array()).map(|arr| {
                    Box::new(
                        arr.iter()
                            .filter_map(|v| v.as_str().map(String::from))
                            .collect::<Vec<_>>(),
                    )
                })
            };
        }
        macro_rules! opt_str_vec_field {
            ($key:expr) => {
                str_vec_field!($key)
            };
        }
        macro_rules! effect_field {
            ($key:expr) => {
                obj.get($key).map(|v| {
                    Box::new(AbilityEffect {
                        text: v
                            .get("text")
                            .and_then(|t| t.as_str())
                            .map(|s| ArcStr::from(s))
                            .unwrap_or_default(),
                        ..Default::default()
                    })
                })
            };
        }

        let filter = || {
            let f = EffectFilter {
                card_type: None,
                counted_by: None,
                count_as: None,
                activated_this_turn: None,
            paid_cost_this_turn: None,
                exclude_self: bool_field!("exclude_self"),
                same_name: bool_field!("same_name"),
                same_unit_name: bool_field!("same_unit_name"),
                group_names: opt_str_vec_field!("group_names"),
                self_target: bool_field!("self_target"),
                under_self: bool_field!("under_self"),
                location: str_field!("location"),
                heart_colors: str_vec_field!("heart_colors").unwrap_or_default(),
                source: zone_field!("source"),
                target: str_field!("target"),
                destination: zone_field!("destination"),
                characters: opt_str_vec_field!("characters"),
                exclude_characters: opt_str_vec_field!("exclude_characters"),
                exclude_group_names: opt_str_vec_field!("exclude_group_names"),
                activation_position: str_field!("activation_position"),
                original_value: bool_field!("original_value"),
                heart_total_limit: u8_field!("heart_total_limit"),
                heart_total_operator: str_field!("heart_total_operator"),
                target_count: u8_field!("target_count"),
                per_unit: bool_field!("per_unit"),
                per_unit_count: u8_field!("per_unit_count"),
                per_unit_type: str_field!("per_unit_type"),
                per_unit_aggregate: str_field!("per_unit_aggregate"),
                per_unit_source: str_field!("per_unit_source"),
                group_reference: str_field!("group_reference"),
                state: obj
                    .get("state")
                    .and_then(|v| v.as_str())
                    .map(|s| Box::new(EffectState::from_str(s))),
                distinct: None,
                position: obj.get("position").and_then(|v| match v {
                    serde_json::Value::String(s) if !s.is_empty() => {
                        Some(Box::new(PositionInfo::String(s.clone())))
                    }
                    serde_json::Value::Object(m) => {
                        let pos = m.get("position").and_then(|p| p.as_str()).map(String::from);
                        let tgt = m.get("target").and_then(|t| t.as_str()).map(ArcStr::from);
                        Some(Box::new(PositionInfo::Struct {
                            position: pos.map(ArcStr::from),
                            target: tgt,
                        }))
                    }
                    _ => None,
                }),
                negation: bool_field!("negation"),
                per_unit_heart_colors: str_vec_field!("per_unit_heart_colors").unwrap_or_default(),
                cost_limit: u8_field!("cost_limit"),
                cost_limit_operator: obj
                    .get("cost_limit_operator")
                    .and_then(|v| v.as_str())
                    .and_then(parse_operator),
                cost_values: obj
                    .get("cost_values")
                    .and_then(|v| v.as_array())
                    .map(|arr| {
                        arr.iter()
                            .filter_map(|x| x.as_u64())
                            .map(|n| n.u8_count())
                            .collect()
                    }),
                blade_limit: u8_field!("blade_limit"),
                blade_limit_operator: obj
                    .get("blade_limit_operator")
                    .and_then(|v| v.as_str())
                    .and_then(parse_operator),
                blade_limit_from_energy_under: bool_field!("blade_limit_from_energy_under"),
                blade_limit_from_cost_member: bool_field!("blade_limit_from_cost_member"),
                blade_limit_offset: u8_field!("blade_limit_offset"),
                keep_shuffle_under: bool_field!("keep_shuffle_under"),
                need_heart_color: str_field!("need_heart_color"),
                need_heart_operator: obj
                    .get("need_heart_operator")
                    .and_then(|v| v.as_str())
                    .and_then(parse_operator),
                need_heart_total: u8_field!("need_heart_total"),
                cost_reference: str_field!("cost_reference"),
                cost_offset: i8_field!("cost_offset"),
                target_member: str_field!("target_member"),
                target_from_selection: bool_field!("target_from_selection"),
                source_position: str_field!("source_position"),
                position_compare: str_field!("position_compare"),
                source_location: str_field!("source_location"),
                exclude_selected: bool_field!("exclude_selected"),
                discard_remaining: bool_field!("discard_remaining"),
                self_cost: bool_field!("self_cost"),
                shuffle: bool_field!("shuffle"),
                effect_constraint: str_field!("effect_constraint"),
                score_floor: u8_field!("score_floor"),
                sign: str_field!("sign"),
                heart_type: str_field!("heart_type"),
                picker: str_field!("picker"),
                reveal: bool_field!("reveal"),
                replaces_event: str_field!("replaces_event"),
                use_limit: u8_field!("use_limit"),
                triggers: str_field!("triggers"),
                original_count: u8_field!("original_count"),
                alternative_count_type: str_field!("alternative_count_type"),
                blind: bool_field!("blind"),
                allow_occupied_stage: bool_field!("allow_occupied_stage"),
                state_change: obj
                    .get("state_change")
                    .and_then(|v| v.as_str())
                    .map(|s| Box::new(EffectState::from_str(s))),
                exclude_position: str_field!("exclude_position"),
                original_operator: obj
                    .get("original_operator")
                    .and_then(|v| v.as_str())
                    .and_then(parse_operator),
                quoted_text: None,
                duration: str_field!("duration"),
                dynamic_count: obj
                    .get("dynamic_count")
                    .and_then(|v| serde_json::from_value(v.clone()).ok())
                    .map(Box::new),
                filter_targets_by_heart_colors: bool_field!("filter_targets_by_heart_colors"),
                card_property: str_field!("card_property"),
                per_unit_location: str_field!("per_unit_location"),
                card_names: str_vec_field!("card_names").unwrap_or_default(),
                all: bool_field!("all"),
                requires_under_card: bool_field!("requires_under_card"),
                optional: bool_field!("optional"),
                cost_total: u8_field!("cost_total"),
                cost_total_operator: None,
                activation_condition_parsed: None,
                action_by: str_field!("action_by"),
                trigger_type: str_field!("trigger_type"),
                repeat_limit: u8_field!("repeat_limit"),
                ability_filter: None,
                multiple_targets: bool_field!("multiple_targets"),
                operation: obj
                    .get("operation")
                    .and_then(|v| v.as_str())
                    .and_then(parse_operation),
                options: None,
                name_constraint: str_field!("name_constraint"),
                name_constraint_source: str_field!("name_constraint_source"),
                ability_filter_triggers: opt_str_vec_field!("ability_filter_triggers"),
                or_ability_filters: None,
                energy_count: u8_field!("energy_count").or_else(|| u8_field!("energy")),
                any_number: bool_field!("any_number"),
                custom_type: str_field!("custom_type"),
                yell_source: str_field!("yell_source"),
                require_all_heart_colors: bool_field!("require_all_heart_colors"),
                heart_color_count: u8_field!("heart_color_count"),
                value: u8_field!("value"),
                per_group: bool_field!("per_group"),
                per_group_count: u8_field!("per_group_count"),
                per_character: bool_field!("per_character"),
                placement_order: None,
                remainder_destination: None,
                remainder_placement_order: None,
                or_card_types: opt_str_vec_field!("or_card_types"),
                exclude_heart_colors: opt_str_vec_field!("exclude_heart_colors"),
                cost_from_revealed: bool_field!("cost_from_revealed"),
                timing_condition: str_field!("timing_condition"),
                identities: opt_str_vec_field!("identities"),
                all_regions: bool_field!("all_regions"),
                trigger_filter: opt_str_vec_field!("trigger_filter"),
                effect_type: str_field!("effect_type"),
                timing: str_field!("timing"),
                treat_as: str_field!("treat_as"),
                question: str_field!("question"),
                answers: opt_str_vec_field!("answers"),
                choice_maker: str_field!("choice_maker"),
                choice_based: bool_field!("choice_based"),
                choice_options: opt_str_vec_field!("choice_options"),
                choice_type: str_field!("choice_type"),
                cost_limit_min: u8_field!("cost_limit_min"),
                cost_limit_max: u8_field!("cost_limit_max"),
                count: u8_field!("count"),
                exclude_by_name_source: str_field!("exclude_by_name_source"),
                baton_touch_trigger: bool_field!("baton_touch_trigger"),
                is_reveal: bool_field!("is_reveal"),
                resource_on_select: effect_field!("resource_on_select"),
                replace_all: bool_field!("replace_all"),
                resource: str_field!("resource"),
                heart_colors_from_selected_card: bool_field!("heart_colors_from_selected_card"),
                heart_color: str_field!("heart_color"),
                ability_gain: str_field!("ability_gain"),
                ability_gain_trigger: str_field!("ability_gain_trigger"),
                anaphora: str_field!("anaphora"),
                gained_effect: effect_field!("gained_effect"),
                ability_text: str_field!("ability_text"),
                target_trigger: str_field!("target_trigger"),
                source_card: str_field!("source_card"),
                suppressed_trigger: str_field!("suppressed_trigger"),
                option: str_field!("option"),
                alternative_effect: effect_field!("alternative_effect"),
                choice_condition: obj
                    .get("choice_condition")
                    .and_then(|v| serde_json::from_value(v.clone()).ok()),
                alternative_condition: obj
                    .get("alternative_condition")
                    .and_then(|v| serde_json::from_value(v.clone()).ok()),
                restriction_type: str_field!("restriction_type"),
                restricted_destination: str_field!("restricted_destination"),
                delayed: bool_field!("delayed"),
                phase: str_field!("phase"),
                non_stackable: bool_field!("non_stackable"),
                heart_selection: bool_field!("heart_selection"),
                blade_type: str_field!("blade_type"),
                choice: bool_field!("choice"),
                lose_blade_hearts: bool_field!("lose_blade_hearts"),
                original_cost: u8_field!("original_cost"),
                parenthetical: opt_str_vec_field!("parenthetical"),
                resource_icon_count: u8_field!("resource_icon_count"),
                ref_value: str_field!("ref_value"),
                ref_offset: i8_field!("ref_offset"),
                id: str_field!("id"),
                opponent_action: effect_field!("opponent_action"),
            };
            if f == EffectFilter::default() {
                None
            } else {
                Some(Box::new(f))
            }
        };

        EffectKind::from_action(action, filter())
    }
}

// Macro-generated getters for EffectKind fields — all fields now live on the
// shared EffectFilter, so only the filter_* getter macros are used.
macro_rules! filter_str_getter {
    ($name:ident, $field:ident) => {
        pub fn $name(&self) -> Option<&str> {
            self.kind
                .as_deref()?
                .filter()?
                .$field
                .as_ref()
                .map(|s| -> &str { s })
        }
    };
}

macro_rules! filter_u8_getter {
    ($name:ident, $field:ident) => {
        pub fn $name(&self) -> Option<u8> {
            self.kind.as_deref()?.filter()?.$field
        }
    };
}

macro_rules! filter_bool_getter {
    ($name:ident, $field:ident) => {
        pub fn $name(&self) -> Option<bool> {
            self.kind.as_deref()?.filter()?.$field
        }
    };
}

macro_rules! filter_copy_getter {
    ($name:ident, $ty:ident, $field:ident) => {
        pub fn $name(&self) -> Option<$ty> {
            self.kind.as_deref()?.filter()?.$field
        }
    };
}

macro_rules! filter_box_vec_ref_getter {
    ($name:ident, $field:ident) => {
        pub fn $name(&self) -> Option<&Vec<String>> {
            self.kind.as_deref()?.filter()?.$field.as_deref()
        }
    };
}

macro_rules! filter_opt_vec_ref_getter {
    ($name:ident, $field:ident) => {
        pub fn $name(&self) -> Option<&Vec<String>> {
            self.kind.as_deref()?.filter()?.$field.as_deref()
        }
    };
}

macro_rules! filter_setter {
    ($fn:ident, $field:ident: $ty:ty) => {
        pub fn $fn(&mut self, val: Option<$ty>) {
            if let Some(f) = self.kind.as_mut().map(Arc::make_mut).and_then(|k| k.filter_mut()) {
                f.$field = val;
            }
        }
    };
}

impl AbilityEffect {
    pub fn ability_filter_any(&self) -> Option<AbilityFilter> {
        self.kind.as_deref()?.filter()?.ability_filter
    }

    filter_opt_vec_ref_getter!(ability_filter_triggers_any, ability_filter_triggers);

    filter_str_getter!(ability_gain_any, ability_gain);

    filter_str_getter!(ability_gain_trigger_any, ability_gain_trigger);

    filter_str_getter!(anaphora_any, anaphora);

    filter_str_getter!(ability_text_any, ability_text);

    pub fn activation_condition_parsed_any(&self) -> Option<&Condition> {
        self.kind
            .as_deref()?
            .filter()?
            .activation_condition_parsed
            .as_deref()
    }

    filter_str_getter!(activation_position_any, activation_position);

    filter_bool_getter!(all_any, all);
    filter_bool_getter!(requires_under_card_any, requires_under_card);

    filter_bool_getter!(all_regions_any, all_regions);

    filter_bool_getter!(allow_occupied_stage_any, allow_occupied_stage);

    filter_str_getter!(alternative_count_type_any, alternative_count_type);

    pub fn alternative_effect_any(&self) -> Option<&AbilityEffect> {
        self.kind
            .as_deref()
            .and_then(|k| k.filter())
            .and_then(|f| f.alternative_effect.as_deref())
    }

    filter_opt_vec_ref_getter!(answers_any, answers);

    filter_bool_getter!(any_number_any, any_number);

    filter_str_getter!(custom_type_any, custom_type);
    filter_str_getter!(yell_source_any, yell_source);

    filter_u8_getter!(blade_limit_any, blade_limit);
    filter_bool_getter!(
        blade_limit_from_energy_under_any,
        blade_limit_from_energy_under
    );
    filter_bool_getter!(
        blade_limit_from_cost_member_any,
        blade_limit_from_cost_member
    );
    filter_u8_getter!(blade_limit_offset_any, blade_limit_offset);
    filter_bool_getter!(keep_shuffle_under_any, keep_shuffle_under);

    filter_copy_getter!(blade_limit_operator_any, Operator, blade_limit_operator);

    filter_str_getter!(blade_type_any, blade_type);

    filter_bool_getter!(blind_any, blind);

    pub fn card_names_any(&self) -> Option<&Vec<String>> {
        Some(&*self.kind.as_deref()?.filter()?.card_names)
    }

    filter_str_getter!(card_property_any, card_property);

    pub fn card_type_any(&self) -> Option<&CardType> {
        self.kind.as_deref()?.filter()?.card_type.as_ref()
    }

    filter_opt_vec_ref_getter!(characters_any, characters);

    /// Discrete set of allowed cost values (OR) — e.g. "コストが10か20" → [10, 20].
    pub fn cost_values_any(&self) -> Option<&Vec<u8>> {
        self.kind.as_deref()?.filter()?.cost_values.as_ref()
    }

    filter_bool_getter!(choice_any, choice);

    filter_bool_getter!(choice_based_any, choice_based);

    filter_str_getter!(choice_maker_any, choice_maker);

    filter_box_vec_ref_getter!(choice_options_any, choice_options);

    filter_str_getter!(choice_type_any, choice_type);

    filter_bool_getter!(cost_from_revealed_any, cost_from_revealed);

    filter_u8_getter!(cost_limit_any, cost_limit);

    filter_u8_getter!(cost_limit_min_any, cost_limit_min);

    filter_copy_getter!(cost_limit_operator_any, Operator, cost_limit_operator);

    pub fn cost_offset_any(&self) -> Option<i8> {
        self.kind
            .as_deref()
            .and_then(|k| k.filter())
            .and_then(|f| f.cost_offset)
    }

    filter_str_getter!(cost_reference_any, cost_reference);

    filter_u8_getter!(cost_total_any, cost_total);

    filter_copy_getter!(cost_total_operator_any, Operator, cost_total_operator);

    filter_bool_getter!(delayed_any, delayed);

    filter_bool_getter!(discard_remaining_any, discard_remaining);

    pub fn distinct_any(&self) -> Option<DistinctType> {
        self.kind.as_deref()?.filter()?.distinct.as_deref().copied()
    }

    filter_str_getter!(duration_any, duration);

    pub fn dynamic_count_any(&self) -> Option<&DynamicCount> {
        self.kind.as_deref()?.filter()?.dynamic_count.as_deref()
    }

    filter_str_getter!(effect_constraint_any, effect_constraint);

    filter_u8_getter!(score_floor_any, score_floor);

    filter_u8_getter!(energy_count_any, energy_count);

    filter_str_getter!(exclude_by_name_source_any, exclude_by_name_source);

    filter_opt_vec_ref_getter!(exclude_characters_any, exclude_characters);

    filter_opt_vec_ref_getter!(exclude_group_names_any, exclude_group_names);

    pub fn exclude_heart_colors_any(&self) -> &[String] {
        self.kind
            .as_deref()
            .and_then(|k| k.filter())
            .and_then(|f| f.exclude_heart_colors.as_ref())
            .map(|b| b.as_slice())
            .unwrap_or(&[])
    }

    pub fn exclude_position_any(&self) -> Option<&str> {
        self.kind
            .as_deref()
            .and_then(|k| k.filter())
            .and_then(|f| f.exclude_position.as_deref())
    }

    filter_bool_getter!(exclude_selected_any, exclude_selected);

    filter_bool_getter!(exclude_self_any, exclude_self);

    filter_bool_getter!(
        filter_targets_by_heart_colors_any,
        filter_targets_by_heart_colors
    );

    pub fn gained_effect_any(&self) -> Option<&AbilityEffect> {
        self.kind
            .as_deref()
            .and_then(|k| k.filter())
            .and_then(|f| f.gained_effect.as_deref())
    }

    filter_opt_vec_ref_getter!(group_names_any, group_names);

    filter_str_getter!(group_reference_any, group_reference);

    filter_u8_getter!(heart_color_count_any, heart_color_count);

    pub fn heart_colors_any(&self) -> &[String] {
        self.kind
            .as_deref()
            .and_then(|k| k.filter())
            .map(|f| f.heart_colors.as_slice())
            .unwrap_or(&[])
    }

    filter_bool_getter!(
        heart_colors_from_selected_card_any,
        heart_colors_from_selected_card
    );

    filter_str_getter!(heart_color_any, heart_color);

    filter_bool_getter!(heart_selection_any, heart_selection);

    filter_str_getter!(heart_type_any, heart_type);

    filter_str_getter!(ref_value_any, ref_value);

    filter_str_getter!(id_any, id);

    filter_opt_vec_ref_getter!(identities_any, identities);

    filter_str_getter!(location_any, location);

    filter_bool_getter!(lose_blade_hearts_any, lose_blade_hearts);

    filter_bool_getter!(multiple_targets_any, multiple_targets);

    filter_str_getter!(name_constraint_any, name_constraint);

    filter_str_getter!(name_constraint_source_any, name_constraint_source);

    filter_str_getter!(need_heart_color_any, need_heart_color);

    filter_copy_getter!(need_heart_operator_any, Operator, need_heart_operator);

    filter_u8_getter!(need_heart_total_any, need_heart_total);

    filter_bool_getter!(negation_any, negation);

    pub fn operation_any(&self) -> Option<&'static str> {
        self.kind
            .as_deref()?
            .filter()?
            .operation
            .map(|op| op.as_str())
    }

    pub fn optional_any(&self) -> Option<bool> {
        self.optional.or_else(|| {
            self.kind
                .as_deref()
                .and_then(|k| k.filter())
                .and_then(|f| f.optional)
        })
    }

    pub fn options_any(&self) -> Option<&Vec<Box<AbilityEffect>>> {
        self.kind.as_deref()?.filter()?.options.as_deref()
    }

    pub fn or_ability_filters_any(&self) -> Option<&Vec<AbilityFilterBranch>> {
        self.kind
            .as_deref()?
            .filter()?
            .or_ability_filters
            .as_deref()
    }

    filter_opt_vec_ref_getter!(or_card_types_any, or_card_types);

    filter_u8_getter!(original_count_any, original_count);

    filter_copy_getter!(original_operator_any, Operator, original_operator);

    filter_bool_getter!(original_value_any, original_value);
    filter_u8_getter!(heart_total_limit_any, heart_total_limit);
    filter_str_getter!(heart_total_operator_any, heart_total_operator);

    filter_u8_getter!(count_as_any, count_as);
    filter_bool_getter!(activated_this_turn_any, activated_this_turn);
    filter_bool_getter!(paid_cost_this_turn_any, paid_cost_this_turn);
    filter_box_vec_ref_getter!(counted_by_any, counted_by);

    filter_u8_getter!(per_group_count_any, per_group_count);

    filter_bool_getter!(per_unit_any, per_unit);

    filter_u8_getter!(per_unit_count_any, per_unit_count);

    pub fn per_unit_heart_colors_any(&self) -> &[String] {
        self.kind
            .as_deref()
            .and_then(|k| k.filter())
            .map(|f| f.per_unit_heart_colors.as_slice())
            .unwrap_or(&[])
    }

    filter_str_getter!(per_unit_location_any, per_unit_location);

    filter_str_getter!(per_unit_type_any, per_unit_type);

    filter_str_getter!(per_unit_aggregate_any, per_unit_aggregate);

    filter_str_getter!(per_unit_source_any, per_unit_source);

    filter_copy_getter!(placement_order_any, PlacementOrder, placement_order);
    filter_str_getter!(remainder_destination_any, remainder_destination);
    filter_copy_getter!(
        remainder_placement_order_any,
        PlacementOrder,
        remainder_placement_order
    );

    pub fn position_any(&self) -> Option<&PositionInfo> {
        self.kind.as_deref()?.filter()?.position.as_deref()
    }

    pub fn repeat_limit_any(&self) -> Option<u8> {
        self.kind.as_deref()?.filter()?.repeat_limit
    }

    filter_str_getter!(replaces_event_any, replaces_event);

    filter_bool_getter!(require_all_heart_colors_any, require_all_heart_colors);

    filter_str_getter!(resource_any, resource);

    filter_u8_getter!(resource_icon_count_any, resource_icon_count);

    pub fn resource_on_select_any(&self) -> Option<&AbilityEffect> {
        self.kind
            .as_deref()
            .and_then(|k| k.filter())
            .and_then(|f| f.resource_on_select.as_deref())
    }

    filter_str_getter!(restricted_destination_any, restricted_destination);

    filter_str_getter!(restriction_type_any, restriction_type);

    filter_bool_getter!(reveal_any, reveal);

    filter_bool_getter!(same_unit_name_any, same_unit_name);
    filter_bool_getter!(same_name_any, same_name);

    filter_bool_getter!(self_cost_any, self_cost);

    filter_bool_getter!(self_target_any, self_target);
    filter_bool_getter!(under_self_any, under_self);

    filter_bool_getter!(shuffle_any, shuffle);

    filter_str_getter!(sign_any, sign);

    filter_str_getter!(source_card_any, source_card);

    filter_str_getter!(source_position_any, source_position);

    filter_str_getter!(position_compare_any, position_compare);

    filter_str_getter!(source_location_any, source_location);

    filter_bool_getter!(per_character_any, per_character);

    /// String form of the filter-level source zone (mirrors the pre-refactor
    /// `Option<ArcStr>.as_deref()`). The typed form is `source_zone()`.
    pub fn source_any(&self) -> Option<&str> {
        self.source
            .map(|z| z.as_str())
            .or_else(|| self.kind.as_deref()?.filter()?.source.map(|z| z.as_str()))
    }

    /// String form of the filter-level destination zone.
    pub fn destination_any(&self) -> Option<&str> {
        self.destination.map(|z| z.as_str()).or_else(|| {
            self.kind
                .as_deref()?
                .filter()?
                .destination
                .map(|z| z.as_str())
        })
    }

    pub fn count_any(&self) -> Option<u8> {
        self.count.or_else(|| {
            self.kind
                .as_deref()
                .and_then(|k| k.filter())
                .and_then(|f| f.count)
        })
    }

    pub fn target_any(&self) -> Option<&str> {
        self.target
            .as_deref()
            .or_else(|| self.kind.as_deref()?.filter()?.target.as_deref())
    }

    /// Typed player-target subset of `target`. Returns `None` when the merged
    /// target string is not a player ("deck", ability-reference strings, etc.),
    /// matching `TargetPlayer::from_str`.
    pub fn target_player(&self) -> Option<TargetPlayer> {
        self.target_any().and_then(TargetPlayer::from_str)
    }

    /// Derived `PlacementTarget` from the filter-level `self_target`/`under_self`
    /// booleans. `self_target` takes precedence (source-filtering semantics);
    /// `under_self` indicates auto-placing under the activating member.
    pub fn placement_target(&self) -> Option<PlacementTarget> {
        let filter = self.kind.as_deref()?.filter()?;
        if filter.under_self == Some(true) {
            Some(PlacementTarget::UnderThisMember)
        } else if filter.self_target == Some(true) {
            Some(PlacementTarget::FilterSelfAsSource)
        } else {
            None
        }
    }

    /// Boolean convenience: whether the effect targets the activating card as a
    /// source-filter (the legacy `self_target` boolean). Derived from
    /// `placement_target()`; equivalent to `self_target_any().unwrap_or(false)`.
    pub fn is_self_target(&self) -> bool {
        self.placement_target() == Some(PlacementTarget::FilterSelfAsSource)
    }

    /// Boolean convenience: whether the placed card auto-places under the
    /// activating member (the legacy `under_self` boolean).
    pub fn is_under_self(&self) -> bool {
        self.placement_target() == Some(PlacementTarget::UnderThisMember)
    }

    /// Typed player-target subset of the *top-level* `self.target` only,
    /// mirroring `target_name()`'s read source (does not consult the filter).
    /// Use this where the original code compared against `target_name()`.
    pub fn target_name_player(&self) -> Option<TargetPlayer> {
        self.target.as_deref().and_then(TargetPlayer::from_str)
    }

    pub fn state_any(&self) -> Option<&str> {
        self.kind
            .as_deref()?
            .filter()?
            .state
            .as_ref()
            .map(|s| s.as_str())
    }

    pub fn state_change_any(&self) -> Option<&str> {
        self.kind
            .as_deref()
            .and_then(|k| k.filter())
            .and_then(|f| f.state_change.as_ref())
            .map(|s| s.as_str())
    }

    filter_str_getter!(suppressed_trigger_any, suppressed_trigger);

    filter_u8_getter!(target_count_any, target_count);

    filter_bool_getter!(target_from_selection_any, target_from_selection);

    filter_str_getter!(target_member_any, target_member);

    filter_str_getter!(target_trigger_any, target_trigger);

    filter_str_getter!(timing_any, timing);

    filter_str_getter!(timing_condition_any, timing_condition);

    filter_str_getter!(treat_as_any, treat_as);

    filter_opt_vec_ref_getter!(trigger_filter_any, trigger_filter);

    filter_str_getter!(trigger_type_any, trigger_type);

    filter_u8_getter!(value_any, value);

    pub fn action_by_any(&self) -> Option<&str> {
        self.kind
            .as_deref()?
            .filter()?
            .action_by
            .as_ref()
            .map(|s| -> &str { s })
    }

    pub fn picker_any(&self) -> Option<&str> {
        self.kind
            .as_deref()?
            .filter()?
            .picker
            .as_ref()
            .map(|s| -> &str { s })
    }

    filter_str_getter!(question_any, question);

    filter_u8_getter!(cost_limit_max_any, cost_limit_max);

    pub fn set_card_names(&mut self, val: Vec<String>) {
        if let Some(f) = self.kind.as_mut().map(Arc::make_mut).and_then(|k| k.filter_mut()) {
            *f.card_names = val;
        }
    }
    pub fn set_group_names(&mut self, val: Option<Box<Vec<String>>>) {
        if let Some(f) = self.kind.as_mut().map(Arc::make_mut).and_then(|k| k.filter_mut()) {
            f.group_names = val;
        }
    }
    pub fn set_action_by(&mut self, val: Option<ArcStr>) {
        if let Some(f) = self.kind.as_mut().map(Arc::make_mut).and_then(|k| k.filter_mut()) {
            f.action_by = val;
        }
    }
    pub fn set_optional(&mut self, val: Option<bool>) {
        self.optional = val;
        if let Some(f) = self.kind.as_mut().map(Arc::make_mut).and_then(|k| k.filter_mut()) {
            f.optional = val;
        }
    }
    filter_setter!(set_energy_count, energy_count: u8);
    filter_setter!(set_per_unit, per_unit: bool);
    filter_setter!(set_per_unit_count, per_unit_count: u8);
    filter_setter!(set_per_unit_type, per_unit_type: ArcStr);
    filter_setter!(set_per_unit_aggregate, per_unit_aggregate: ArcStr);
    filter_setter!(set_self_target, self_target: bool);
    filter_setter!(set_target_count, target_count: u8);
    filter_setter!(set_target_member, target_member: ArcStr);

    pub fn target_name(&self) -> &str {
        self.target.as_deref().unwrap_or("self")
    }

    /// Returns the source zone string with a static default.
    pub fn source_or(&self, default: &'static str) -> &str {
        self.source_any().unwrap_or(default)
    }

    /// Typed source zone. `self.source` is `Option<Zone>`; this exposes it directly.
    pub fn source_zone(&self) -> Option<Zone> {
        self.source
    }

    /// String form of the source zone (mirrors the pre-refactor `Option<ArcStr>.as_deref()`).
    pub fn source_str(&self) -> Option<&str> {
        self.source.map(|z| z.as_str())
    }

    /// Build a `CardFilter` containing the 7 base filter fields (card_type,
    /// group, cost_limit, cost_operator, characters, exclude_characters,
    /// exclude_self) that effect handlers most commonly need. Also carries the
    /// current-blade filter ("ブレードをNつ以上持つ", no 元々) which the caller
    /// applies post-`matches()` via util::filter_current_blade (matches() has no
    /// modifier access). heart_colors are only treated as a target filter when
    /// `filter_targets_by_heart_colors` is set (a gained resource color must not
    /// become a target filter).
    pub fn filter_subset(&self) -> crate::ability::util::CardFilter<'_> {
        let original = self.original_value_any().unwrap_or(false);
        // Same redundancy `CardFilter::from_effect` drops: an inclusion and an exclusion
        // of the same group cannot both hold, and the extractor produces exactly that when
        // it hoists a clause's group onto the effect carrying that clause's exclusion.
        // 百生 吟子 (PL!HS-bp5-004) parses with both set to スリーズブーケ, so this
        // builder demanded a card be スリーズブケット AND not, the candidate set was
        // always empty, and 「『スリーズブーケ』以外のメンバー1人につきブレードを2得る」
        // could never pay out. The exclusion is the reading the text asks for.
        let excluded = self.exclude_group_names_any();
        let group = match (self.group_name(), excluded) {
            (Some(included), Some(ex)) if excluded_matches(ex, included) => None,
            (group, _) => group,
        };
        crate::ability::util::CardFilter {
            card_type: self.card_type_any().map(|ct| ct.as_card_str()),
            group,
            cost_limit: self.cost_limit_any(),
            cost_operator: self.cost_limit_operator_any().map(Operator::as_str),
            cost_values: self.cost_values_any(),
            characters: self.characters_any(),
            exclude_characters: self.exclude_characters_any(),
            exclude_self: if self.exclude_self_any().unwrap_or(false) {
                Some(-1)
            } else {
                None
            },
            exclude_group_names: excluded.map(Vec::as_slice),
            name_fragments: match self.card_names_any() {
                Some(names) if !names.is_empty() => Some(names),
                _ => None,
            },
            card_property: self.card_property_any(),
            negation: self.negation_any().unwrap_or(false),
            heart_colors: if self.filter_targets_by_heart_colors_any().unwrap_or(false)
                && !self.heart_colors_any().is_empty()
            {
                self.heart_colors_any()
            } else {
                &[]
            },
            distinct: self.distinct_any(),
            original_blade_limit: if original {
                self.blade_limit_any()
            } else {
                None
            },
            original_blade_operator: if original {
                self.blade_limit_operator_any().map(Operator::as_str)
            } else {
                None
            },
            // 「元々持つハート」 can only ever mean the printed total, so this one is
            // not gated on `original` — there is no current-heart variant of the
            // phrase to disambiguate from.
            original_heart_total: self.heart_total_limit_any(),
            original_heart_total_operator: self.heart_total_operator_any().or(Some("<=")),
            current_blade_limit: if original {
                None
            } else {
                self.blade_limit_any()
            },
            current_blade_operator: if original {
                None
            } else {
                self.blade_limit_operator_any().map(Operator::as_str)
            },
            ..Default::default()
        }
    }

    /// Returns the count with a caller-provided default.
    pub fn count_or(&self, n: u8) -> u8 {
        self.count.unwrap_or(n)
    }

    pub fn value_or_count(&self, default: u8) -> u8 {
        self.value_any().or(self.count).unwrap_or(default)
    }

    /// Returns the first group name, if any.
    pub fn group_name(&self) -> Option<&str> {
        self.group_names_any()
            .and_then(|gn| gn.first().map(|s| s.as_str()))
    }

    /// Borrowing form of [`Self::normalized_steps`].
    ///
    /// `normalized_steps()` returns an owned `Vec<Box<AbilityEffect>>`, and each
    /// `Box<AbilityEffect>` clone is a DEEP clone — `AbilityEffect` carries an
    /// `Option<Arc<EffectKind>>` wrapping a ~1.2 KB `EffectFilter` with 26
    /// `Box`es, so one element costs ~4 allocations plus ~1.7 KB of memcpy. For
    /// a 3-step sequential ability that is ~128 allocations, spent before a
    /// single step runs.
    ///
    /// The two arms that actually dominate — a card carrying explicit
    /// `effect_steps`, and a plain `Sequential` whose steps are already in
    /// `compound.actions` — need no construction at all, so they borrow. The
    /// remaining arms genuinely build a new list (they patch a condition onto a
    /// step, or synthesise a wrapper) and keep allocating exactly as before.
    ///
    /// Both paths are semantically identical to `normalized_steps()`; the
    /// existing test suite covers every routing arm.
    pub fn normalized_steps_cow(&self) -> Cow<'_, [Box<AbilityEffect>]> {
        if let Some(ref steps) = self.effect_steps {
            return Cow::Borrowed(&steps[..]);
        }
        match self.action {
            ActionType::Sequential => match self.compound_branch().and_then(|c| c.actions.as_ref()) {
                Some(ref actions) => Cow::Borrowed(&actions[..]),
                None => Cow::Owned(Vec::new()),
            },
            _ => Cow::Owned(self.normalized_steps()),
        }
    }

    /// Normalized sub-effect steps
    pub fn normalized_steps(&self) -> Vec<Box<AbilityEffect>> {
        if let Some(ref steps) = self.effect_steps {
            return steps.clone();
        }
        match self.action {
            ActionType::Sequential => self
                .compound_branch()
                .and_then(|c| c.actions.clone())
                .unwrap_or_default(),
            ActionType::LookAndSelect => {
                let mut out: Vec<Box<AbilityEffect>> = Vec::new();
                if let Some(c) = self.compound_branch() {
                    if let Some(ref la) = c.look_action {
                        out.push(la.clone());
                    }
                    if let Some(ref sa) = c.select_action {
                        out.push(sa.clone());
                    }
                    if let Some(ref fu) = c.followup_action {
                        out.push(fu.clone());
                    }
                }
                out
            }
            ActionType::ConditionalAlternative => {
                let mut out: Vec<Box<AbilityEffect>> = Vec::new();
                let c = self.compound_branch();
                let mut primary = c.and_then(|c| c.primary_effect.clone());
                let mut alternative = self.alternative_effect_any().cloned();
                let condition = c.and_then(|c| c.alternative_condition.clone());
                if let Some(mut alt) = alternative.take() {
                    if let Some(cond) = condition {
                        alt.condition = Some(cond);
                    }
                    out.push(Box::new(alt));
                }
                if let Some(pri) = primary.take() {
                    out.push(pri);
                }
                out
            }
            ActionType::ConditionalOnResult => {
                let mut out: Vec<Box<AbilityEffect>> = Vec::new();
                if let Some(c) = self.compound_branch() {
                    if let Some(ref pri) = c.primary_effect {
                        out.push(pri.clone());
                    }
                    if let Some(ref follow) = c.followup_action {
                        let mut f = follow.clone();
                        if let Some(rc) = c.result_condition.clone() {
                            f.condition = Some(rc);
                        }
                        out.push(f);
                    }
                }
                out
            }
            ActionType::ConditionalOnOptional => {
                let mut step = AbilityEffect {
                    action: ActionType::ConditionalOptional,
                    ..Default::default()
                };
                if let Some(c) = self.compound_branch() {
                    if let Some(ref oa) = c.optional_action {
                        step.text = oa.text.clone();
                    }
                    let step_c = step.compound_branch_mut();
                    step_c.optional_action = c.optional_action.clone();
                    step_c.conditional_action = c.conditional_action.clone();
                    step_c.conditional_negation = c.conditional_negation;
                }
                vec![Box::new(step)]
            }
            _ => Vec::new(),
        }
    }

    pub fn action_by(&self) -> Option<&str> {
        self.kind
            .as_deref()?
            .filter()?
            .action_by
            .as_ref()
            .map(|s| -> &str { s })
    }

    pub fn opponent_action(&self) -> Option<&AbilityEffect> {
        self.kind
            .as_deref()
            .and_then(|k| k.filter())
            .and_then(|f| f.opponent_action.as_deref())
    }

    pub fn effect_type(&self) -> Option<&str> {
        self.kind
            .as_deref()?
            .filter()?
            .effect_type
            .as_ref()
            .map(|s| -> &str { s })
    }
}

