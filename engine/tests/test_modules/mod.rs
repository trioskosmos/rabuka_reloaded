pub mod ability_engine_fixes_test;
pub mod ability_from_source_test;
pub mod abundant_test;
pub mod action_coverage_test;
pub mod angelic_angel_test;
pub mod aurora_flower_identity_test;
pub mod aurora_flower_test;
pub mod awake_test;
pub mod awaken_the_power_test;
pub mod ayumu_azuna_test;
pub mod ayumu_pb1_test;
pub mod b7_constant_ability_test;
pub mod b8_live_timing_test;
pub mod b9_more_test;
pub mod batch5_test;
pub mod batch_nico_bp4_hanayo_test;
pub mod baton_touch_restriction_combination_test;
pub mod baton_touch_test;
pub mod blade_heart_colorless_test;
pub mod bp7_q276_cheer_mode_return_hand_test;
pub mod bp7_q278_q279_joint_blade_test;
pub mod bp7_q280_energy_do_not_activate_test;
pub mod bp7_ai_energy_under_member_optional_test;
pub mod bp7_kanata_look_select_test;
pub mod bp7_parser_gap_cards_test;
pub mod bp7_sd2_gap_test;
pub mod blade_heart_types_test;
pub mod blade_per_discard_test;
pub mod bloom_hs_test;
pub mod bp7_character_name_condition_test;
pub mod bp7_deck_bottom_source_test;
pub mod bp7_ai_choice_under_member_test;
pub mod bp7_dia_both_hand_reorder_test;
pub mod bp7_fire_bird_blade_gain_test;
pub mod bp7_heart_copy_test;
pub mod bp7_kanon_baton_touch_replace_test;
pub mod bp7_kanon_under_member_blade_test;
pub mod bp7_karin_wait_blade_limit_test;
pub mod bp7_dia_look_bottom_select_test;
pub mod bp7_mari_look_top_split_test;
pub mod bp7_ginko_select_discard_deck_bottom_test;
pub mod bp7_watanabe_under_card_blade_test;
pub mod bp7_audrey_blade_max_test;
pub mod bp7_aquarium_yell_source_test;
pub mod bp7_setsuna_abilities_test;
pub mod bp7_watanabe_select_self_and_other_test;
pub mod bp7_like_a_treasure_optional_test;
pub mod bp7_mirai_no_oto_optional_test;
pub mod bp7_mia_optional_recover_test;
pub mod bp7_auto_gap_test;
pub mod bp7_constant_edge_case_test;
pub mod bp7_mia_deck_to_discard_test;
pub mod bp7_q269_mia_yell_no_trigger_test;
pub mod bp7_mia_play_cost_reduction_test;
pub mod bp7_q271_colorful_dreams_test;
pub mod bp7_q272_just_believe_test;
pub mod bp7_q273_watanabe_cost_test;
pub mod bp7_we_will_energy_score_test;
pub mod bp7_tang_keke_cost_test;
pub mod bp7_tomari_discard_all_hand_test;
pub mod bp7_q274_immune_still_selectable_test;
pub mod bp7_q275_forcepick_wait_test;
pub mod bp7_kanan_formation_change_test;
pub mod bp7_kanan_wait_immunity_test;
pub mod bp7_wait_immunity_helpers;
pub mod bp7_q270_emma_color_diversity_test;pub mod bp7_kanata_choice_test;
pub mod bp7_cooking_with_love_test;
pub mod bp7_karin_dynamic_blade_wait_test;
pub mod bp7_under_member_per_unit_blade_test;
pub mod bp7_q266_natsumi_blade_wait_test;
pub mod bp7_q267_rinna_mill_refresh_test;
pub mod bp7_q268_shioriko_empty_area_deploy_test;
pub mod bp7_ren_both_trigger_test;
pub mod bp7_ren_energy_placed_gain_blade_test;
pub mod bp7_ruby_front_blade_test;
pub mod bring_love_test;
pub mod butterfly_wing_suppress_test;
#[cfg(feature = "bytecode_abilities")]
pub mod bytecode_deep_compare_test;
#[cfg(feature = "bytecode_abilities")]
pub mod bytecode_validation_test;
pub mod cannot_baton_touch_test;
pub mod card_ability_tests;
pub mod card_count_state_test;
pub mod character_condition_fix_test;
pub mod cheer_pipeline_test;
pub mod chika_test;
pub mod chisato_bp5_test;
pub mod chisato_jidou_move_analysis_test;
pub mod chisato_live_success_test;
pub mod chisato_move_test;
pub mod chisato_natsumi_test;
pub mod chisato_test;
pub mod choice_bullet_test;
pub mod condition_negative_test;
pub mod constant_edge_case_test;
pub mod daisuki_and_dia_test;
pub mod daisuki_test;
pub mod dazzling_test;
pub mod draw_one_put_bottom_debut_test;
pub mod draw_phase_fix;
pub mod dream_believers_test;
pub mod dream_with_you_test;
pub mod duplicate_id_mulligan_test;
pub mod e2e_basic_game_test;
pub mod edelnote_test;
pub mod eli_bp4_test;
pub mod eli_sequential_cost_test;
pub mod eli_test;
pub mod emma_bp5_test;
pub mod emma_test;
pub mod empty_energy_deck_test;
pub mod energy_and_member_under_test;
pub mod eri_bp3_test;
pub mod eternalize_love_test;
pub mod formation_change_test;
pub mod fuyumari_debut_test;
pub mod fuyumari_test;
pub mod gameplay_test;
pub mod genki_zenkai_test;
pub mod hanabiko_discard_test;
pub mod hanamaru_test;
pub mod kidou_softlock_test;
pub mod kurosawa_dia_re_yell_test;
pub mod size_budget_test;
pub mod tang_keke_test;

pub mod hanamusubi_exclude_self_test;
pub mod hanamusubi_test;
pub mod hanano_test;
pub mod hanayo_bp4_constant_test;
pub mod hanayo_bp6_test;
pub mod hanayo_test;
pub mod hasunosora_bp6_test;
pub mod hasunosora_pb1_test;
pub mod hazuki_test;
pub mod heart_override_test;
pub mod himeno_bp5_live_start_test;
pub mod himeno_bp_versions_test;
pub mod himeno_front_test;
pub mod himeno_position_change_single_test;
pub mod himeno_test;
pub mod hinoshita_test;
pub mod honoka_bp5_live_score_test;
pub mod honoka_test;
pub mod izumi_bp5_test;
pub mod izumi_bp6_test;
pub mod izumi_pb1_test;
pub mod jellyfish_test;
pub mod jimo_ai_dash_test;
pub mod joint_card_live_start_test;
pub mod kagayaiteru_test;
pub mod kanan_bp7_debut_queue_test;
pub mod kanata_bp1_test;
pub mod kanata_restrict_test;
pub mod kanon_invalidate_test;
pub mod kanon_pb2_test;
pub mod kanon_test;
pub mod kasumi_energy_under_test;
pub mod kasumi_test;
pub mod karin_bp4_004_live_start_test;
pub mod keke_bp5_test;
pub mod kinako_bp5_test;
pub mod kinako_each_time_blade_test;
pub mod kinako_hs_test;
pub mod kinako_live_success_or_move_test;
pub mod kinako_sakurakoji_test;
pub mod kinako_test;
pub mod konata_bp4_test;
pub mod konata_test;
pub mod kotori_bp5_003_test;
pub mod kotori_test;
pub mod kuroe_dia_bp6_test;
pub mod ladybug_test;
pub mod link_to_future_test;
pub mod live_success_rules_test;
pub mod live_success_sequential_test;
pub mod ll_bp7_001_triple_member_test;
pub mod ll_joint_test;
pub mod look_and_select_test;
pub mod looked_at_discard_test;
pub mod love_u_test;
pub mod love_wing_bell_test;
pub mod maki_appear_test;
pub mod maki_test;
pub mod mari_bp2_test;
pub mod mari_test;
pub mod mebius_loop_test;
pub mod mei_bp5_test;
pub mod mia_q190_test;
pub mod mia_test;
pub mod mifune_test;
pub mod miracle_stay_tune_test;
pub mod miracle_wave_test;
pub mod mirai_ticket_test;
pub mod miyashita_ai_bp3_test;
pub mod miyashita_ai_bp5_test;
pub mod miyashita_ai_pb1_test;
pub mod miyashita_ai_pr_test;
pub mod miyashita_ai_test;
pub mod modify_required_hearts_global_test;
pub mod movement_condition_test;
pub mod multiname_card_test;
pub mod mute_kibiriver_test;
pub mod mymai_tonight_test;
pub mod nagi_live_card_draw_test;
pub mod nahone_live_start_same_name_test;
pub mod natsumi_bp5_test;
pub mod natsumi_test;
pub mod neutral_live_success_test;
pub mod nico_cannot_activate_test;
pub mod nico_recover_test;
pub mod nozomi_test;
pub mod opponent_choice_tests;
pub mod parser_issues_e2e_test;
pub mod parser_issues_e2e_test_part2;
pub mod parser_issues_e2e_test_part3;
pub mod pb2_under_member_test;
pub mod per_unit_discard_fix;
pub mod performance_phase_rules_test;
pub mod performance_pipeline_test;
pub mod performance_snapshot_audit_test;
pub mod poppin_test;
pub mod position_ability_test;
pub mod position_change_condition_test;
pub mod position_change_multi_test;
pub mod position_change_non_optional_test;
pub mod position_change_triggers_jidou_move_test;
pub mod q127_heart_set_plus_global_test;
pub mod q137_already_waited_cost_test;
pub mod q146_per_member_draw_test;
pub mod q148_blade_total_waited_test;
pub mod q159_debut_from_discard_self_cost_test;
pub mod q38_live_card_zone_test;
pub mod q46_kanako_all_heart_timing_test;
pub mod qa_new_tests;
pub mod qa_new_tests198;
pub mod qa_new_tests202;
pub mod qa_new_tests209;
pub mod qa_new_tests238;
pub mod qa_new_tests246;
pub mod qa_new_tests250;
pub mod qa_new_tests251;
pub mod qa_new_tests252;
pub mod qa_new_tests253;
pub mod qa_new_tests254;
pub mod qa_new_tests255;
pub mod qa_new_tests256;
pub mod qa_new_tests257;
pub mod qa_remaining_tests2;

pub mod remaining_quick_test;
pub mod ren_bp4_test;
pub mod ren_test;
pub mod riko_test;
pub mod rin_bp6_test;
pub mod rin_test;
pub mod rina_bp3_debut_test;
pub mod rina_bp3_test;
pub mod rina_test;
pub mod rurino_bp5_test;
pub mod rurino_pb1_test;
pub mod rurino_test;
pub mod sayaka_bp6_test;
pub mod sayaka_test;
pub mod score_condition_integration_test;
pub mod sd2_002_basic_test;
pub mod setsuna_bp5_test;
pub mod setsuna_pb1_heart_constant_test;
pub mod setsuna_pb1_test;
pub mod setsuna_test;
pub mod shioriko_bp4_swap_test;
pub mod shizuku_bp5_test;
pub mod shizuku_pb1_test;
pub mod shodo_rin_energy_test;
pub mod smile_test;
pub mod solitude_test;
pub mod special_color_test;
pub mod stage_to_discard_ability_test;
pub mod start_true_dreams_test;
pub mod state_condition_test;
pub mod strawberry_test;
pub mod strawberry_trapper_test;
pub mod sumire_auto_test;
pub mod sumire_bp4_test;
pub mod sumire_bp5_test;
pub mod sunny_test;
pub mod surplus_heart_test;
pub mod takaramono_test;
pub mod tokimeki_test;
pub mod totemari_test;
pub mod toubatsu_test;
pub mod tsunagaru_connect_test;
pub mod turn_number_condition_test;
pub mod umi_bp3_test;
pub mod umi_q228_test;
pub mod untested_abilities_test;
pub mod victory_road_test;
pub mod vitamin_test;
pub mod wao_wao_test;
pub mod wien_bp5_test;
pub mod wien_cost_mod_test;
pub mod wien_n_test;
pub mod wien_pb2_test;
pub mod yoshiko_card_check_test;
pub mod yoshiko_center_ability_test;
pub mod yoshiko_debug_move_test;
pub mod yoshiko_debug_test;
pub mod yoshiko_debut_test;
pub mod yoshiko_detailed_test;
pub mod yoshiko_edge_cases_test;
pub mod yoshiko_filter_test;
pub mod yoshiko_fixed_test;
pub mod yoshiko_group_check_test;
pub mod yoshiko_main_effect_only_test;
pub mod yoshiko_single_target_test;
pub mod yoshiko_test;
pub mod you_debut_test;
pub mod zero_tested_action_types_test;

// PVP room / web server integration tests (excluded from main test suite 窶・runs slowly)
// pub mod pvp_room_test;

// Playthrough of coverage-gap test cards
pub mod untested_abilities_playthrough_test;
pub mod untested_abilities_batch2_test;
pub mod either_or_state_change_test;
pub mod untested_abilities_batch4_test;
pub mod untested_abilities_batch5_test;
pub mod untested_abilities_batch6_test;
pub mod untested_abilities_batch7_test;
pub mod untested_abilities_batch8_test;
pub mod untested_abilities_batch9_test;
pub mod untested_secondary_abilities_test;
pub mod restriction_mechanics_test;
pub mod untested_abilities_batch3_test;

// Unique / edge-case ability tests
pub mod unique_abilities_test;

// New test files for coverage gaps
pub mod bp4_live_start_change_state_gain_test;
pub mod bp6_004_002_audit_test;
pub mod cards_6_thru_13_test;
pub mod chika_center_cost_test;
pub mod chika_bp7_conditional_result_test;
pub mod conditional_alternative_test;
pub mod daydream_mermaid_test;
pub mod ll_bp1_001_test;
pub mod modify_required_hearts_13key_test;
pub mod parser_fixes_e2e_test;
pub mod pl_bp5_012_test;
pub mod pl_bp6_003_test;
pub mod pl_bp6_006_test;
pub mod pl_n_bp1_008_test;
pub mod pl_n_bp1_027_test;
pub mod pl_s_bp5_010_test;
pub mod pl_s_bp6_006_test;
pub mod pl_sp_sd1_002_test;
pub mod pr_energy_place_cost_test;
pub mod sp_bp5_choice_energy_test;
pub mod sp_bp5_leftside_cost_test;
pub mod stellar_phoenix_test;
pub mod target_selection_test;

// New test files for parser/engine fix coverage
pub mod ability_trigger_fix_test;
pub mod ally_appear_each_time_test;
pub mod ayumu_pb1_constant_test;
pub mod card_filter_test;
pub mod condition_evaluation_test;
pub mod deep_resonance_bp3_test;
pub mod dive_auto_trigger_test;
pub mod dive_edge_test;
pub mod dive_live_card_test;
pub mod emotion_bp4_test;
pub mod hs_bp2_018_live_card_zone_test;
pub mod kanon_bp5_constant_test;
pub mod live_card_zone_movement_test;
pub mod ll_bp2_001_cost_reduction_test;
pub mod maki_pb1_006_debut_test;
pub mod maki_bp6_006_reveal_test;
pub mod maki_pb1_test;
pub mod nozomi_bp4_aggregate_test;
pub mod on_energy_placed_test;
pub mod on_hand_to_discard_test;
pub mod pl_hs_bp1_003_test;
pub mod pl_hs_bp6_004_test;
pub mod pl_hs_sd1_008_test;
pub mod pl_sp_pb2_005_test;
pub mod trigger_card_integration_test;
pub mod upper_batch_on_yell_test;

// Otherwise-condition routing (reveal 竊・conditional 竊・otherwise)
pub mod otherwise_condition_flow_test;

// Multi-color heart gain (PL!S-PR-040-PR / PL!N-PR-023-PR)
pub mod multi_color_heart_test;

// modify_required_hearts + exclude_heart_colors (PL!-bp5-023-L)
pub mod modify_required_hearts_exclude_heart_test;

// modify_score + per_unit + need_heart_total filtering (PL!SP-pb2-045-L)
pub mod wakana_shiki_test;
pub mod zettai_lover_test;

// KALEIDOSCORE debut: discard 竊・energy wait + conditional draw (PL!SP-pb2-013-R)
pub mod keke_pb2_013_debut_test;

// CatChu! per-unit energy activation (PL!SP-pb2-018-R)
pub mod catchu_energy_activation_test;
pub mod live_cards_disappear_test;
// §5.5: ディストーション per-unit need-heart modification + score gate (PL!SP-pb2-048-L)
pub mod distortion_need_hearts_test;

// Q176: 蝨堤伐豬ｷ譛ｪ pb1-013 窶・opponent picks from your hand blind, reveal live 竊・+1 score
pub mod umi_pb1_013_test;
// Q176 companion: 蝨堤伐豬ｷ譛ｪ PR-014 窶・you pick from opponent's hand blind, reveal 竊・draw
pub mod umi_pr014_test;

// 繝弱Φ繝輔ぅ繧ｯ繧ｷ繝ｧ繝ｳ!! (PL!SP-bp4-024) 窶・LiveStart cost comparison (ab#0)
pub mod nonfiction_cost_comparison_test;

// SELF CONTROL!! + 鮖ｿ隗定＊濶ｯ: position_change triggers moved-this-turn blade grant
pub mod self_control_position_change_test;

// 黒澤ルビィ (PL!S-bp7-018-N) 登場: move a chosen stage member to the center area.
pub mod ruby_bp7_center_position_test;

// 嵐 千砂都 (PL!SP-bp7-003) ab#2 起動: reveal a cost-10-or-20 member, place under, draw 2.
pub mod chika_bp7_reveal_cost_test;

// 譯懷・譴ｨ蟄・(PL!S-bp5-002) 窶・LiveStart center: left_cost == right_cost 竊・wait opponent low-blade
// Verifies require_position_cards: both positions must have cards (empty = no trigger)
pub mod riko_bp5_center_cost_equal_test;

// Wonder zone (PL!-bp5-020-L) 窶・modify_required_hearts per-unit with per_unit_heart_colors + max_repeats
pub mod auto_system_stress_test;
pub mod baton_touch_order_test;
pub mod wonder_zone_max_repeats_test;

// 蟷ｳ螳牙錐縺吶∩繧・(PL!SP-bp2-004-R) 窶・constant: center has highest cost 竊・heart03
pub mod sumire_bp2_center_cost_test;

// Rule/ability structured logging: bounded buffers, no "pending" residue after
// resolution, and ChoiceOffered/ChoiceResolved capture.
pub mod burn_energy_under_test;
pub mod hanamaru_bp3_choose_player_test;
pub mod logging_test;
pub mod pl_s_bp7_007_test;
pub mod untested_choice_change_target_test;
pub mod fixed_customs_test;
pub mod energy_state_condition_test;
pub mod ginko_baton_touch_hasu_search_test;
pub mod miyata_draw_until_test;
pub mod wien_yell_count_test;
pub mod kasumi_turn_limit_test;
pub mod sayaka_loop_test;
pub mod max_distinct_names_test;

