#!/bin/bash
COMMIT=$GIT_COMMIT
case "$COMMIT" in
  "0cf145719619c2239655d9678ea6619293fa6ba7") echo "Implement conditional_on_result for Maki bp6 and fix move_cards selection bleed" ;;
  "940ad337047dbd0825ec1d305d64329f7ed37fe3") echo "Implement AllRevealedMatchHeartColor condition and update baton touch cost calculations" ;;
  "bccff77d0878b000644d6d93daa695a477342584") echo "Fix unique group count to use card group instead of unit" ;;
  "cc7cc7842bbeee476dfe0de17d1a7e0726f609e8") echo "Improve SelectCard action resolution for compound effects" ;;
  "e568c9ff6fa0e489a958a57e1aee3d4eb687fc46") echo "Implement place_energy_under_member action and parser support" ;;
  "209917fc28a593f134e3354557b4a0d94584fc49") echo "Implement same_group_name cost filtering and source_ability in action params" ;;
  "d1bc039b3b646ce1246c0076c2e6227e8124fd24") echo "Normalize heart notation in abilities and refactor CardRenderer" ;;
  "6d42f9e84316c779de52f31f8d83a7264d086d73") echo "Remove redundant group filters from abilities.json" ;;
  "5b8e3c045f4014a65682854d0c760336155f1d32") echo "Fix movement tracking for batch movements and ability transitions" ;;
  "26cc51bb34dfdeaa6d052122357067c85bc5f885") echo "Implement looked_at source for card movements and discard_remaining logic" ;;
  "1a416a62597908e70bf32da23e3268380ef9aefd") echo "Refine ability filters with card properties and negation" ;;
  "39def397212738ffa305d94676fd324d5578ca89") echo "Remove auto_abilities_report.txt" ;;
  "cec8a954093c24550fda4f8fbf729d0fb743aaf1") echo "Refine group_names propagation in ability parser" ;;
  "865a6543e9720f81299feba0dad21c0a8ac60cbd") echo "Regenerate extracted abilities in abilities.json" ;;
  "3e3f252de5dd0f8ceb3718b22f53c295e5699a03") echo "Regenerate extracted abilities in abilities.json" ;;
  "06fac5032773d3556366141062c405c74c95b6c4") echo "Regenerate extracted abilities in abilities.json" ;;
  "31822c87c8098af7510ca02bd9c77752dafde2bd") echo "Regenerate extracted abilities in abilities.json" ;;
  "083de51fb5bfeb7ffdae297cdf6a4e6c134152f5") echo "Regenerate extracted abilities in abilities.json" ;;
  "6001c8b9b6348a48775f571c76da736b943d8b79") echo "Regenerate extracted abilities in abilities.json" ;;
  "3a7ee38c91fdd856f17aabf94e9db3fc17968d0e") echo "Regenerate extracted abilities in abilities.json" ;;
  "50a27aec9b69a29363674840d79f703cf8de218e") echo "Regenerate extracted abilities in abilities.json" ;;
  "fc20a000920dd4a6cc6275ae50f910a8d9f28cb4") echo "Regenerate extracted abilities in abilities.json" ;;
  "36c04a09709e8eb6d9ab0be254abb9a8a58743b3") echo "Fix awakening promise logic" ;;
  "687c7b66601308159e3d231011d43ecfa5c3c7cd") echo "Fix optional action auto-skip tracking in sequential loops" ;;
  *) echo "$GIT_COMMIT_MESSAGE" ;;
esac
