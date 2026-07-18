// DEPRECATED — no longer used.
//
// The bytecode VM used to hand-decode a bespoke binary schema here, which
// required manual decoder edits for every new action type / field and silently
// diverged from the JSON loader (causing test failures). The bytecode path now
// stores each ability as its minified JSON slice and decodes it through the
// *same* serde `Ability` deserialization as the default loader (see
// `src/ability/vm.rs`). This is fully data-driven: new ability types need zero
// decoder changes.
//
// This file is retained only as a historical record; it is NOT included by any
// module and is not compiled. Regenerate the real artifacts with
// `python cards/compile_abilities.py`.
