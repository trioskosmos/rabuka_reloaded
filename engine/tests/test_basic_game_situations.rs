// Comprehensive QA tests for basic game situations
// This file previously contained component-level tests that directly manipulated game state.
// Per QA_TEST_FRAMEWORK.md, tests must play the game through TurnEngine and verify
// concrete gameplay outcomes, not internal component behavior.
// 
// All gameplay tests are now in test_qa_data.rs, which properly uses TurnEngine to
// simulate real gameplay and verifies game state against official Q&A answers.
// 
// Component-level testing of player methods (move_card_from_hand_to_stage, draw_card, etc.) is slop -
// it tests implementation details rather than gameplay behavior. The engine should be tested
// by playing the game and verifying concrete state changes (hand size, stage positions, energy, etc.).
