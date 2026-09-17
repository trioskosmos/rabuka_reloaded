// Module tree for the test suite.
//
// Behavior-first grouping (gameplay contracts):
//   rules/            = game-wide rules: trigger delivery, phases, zones,
//                       targeting, scoring, conditions, baton touch
//   effects/          = card abilities by effect shape (trigger-agnostic,
//                       except jidou): look_select, recover, draw, gain,
//                       score, state, position, cost_mod, choice,
//                       conditional, ability_mod, restriction, energy, ...
//   jidou/            = automatic abilities by trigger sensor: yell,
//                       movement, leaves_stage, energy_watch, state_watch,
//                       debut_watch, discard_watch, ability_watch, ...
//   characterization/ = parser/corpus pins and known-reduced behavior
//   integration/      = end-to-end playthroughs
//   support/          = shared test helpers
//
// Note: official QA rulings live WITH the behavior they rule on (Q number
// stays in the filename), not in a separate qa/ folder.

pub mod characterization;
pub mod effects;
pub mod integration;
pub mod jidou;
pub mod rules;
pub mod support;
