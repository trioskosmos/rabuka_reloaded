// Module tree for the test suite.
//
// Behavior-first grouping (gameplay contracts):
//   card_flow/  = deck inspection, search/recover, deck-bottom placement
//   mechanics/  = game-wide rules (attacker order, phase machine, hearts)
//   jidou/      = automatic (self-triggering) abilities, by complexity
//   abilities/  = other (activated / live / constant) abilities, by complexity
// Other folders group engine subsystems, QA rulings, coverage batches,
// edge cases, integration, and shared helpers.

pub mod card_flow;
pub mod jidou;
pub mod abilities;
pub mod qa;
pub mod batches;
pub mod edge_cases;
pub mod mechanics;
pub mod integration;
pub mod support;
