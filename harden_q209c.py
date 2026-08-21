p = 'engine/tests/test_modules/qa_new_tests209.rs'
src = open(p, encoding='utf-8').read()
DQ = chr(34)

replacements = []

# q209_kasumi_no_niji_in_discard_skips (line ~315-322)
replacements.append((
    "    if game.has_pending_choice() {\n        game.select_generated(0); // pay 2 energy\n    }\n\n    if game.has_pending_choice() {\n        game.select_indices(&[0]); // discard filler\n    }",
    "    assert!(game.has_pending_choice(), " + DQ + "Kasumi activation must offer energy payment" + DQ + ");\n    game.select_generated(0); // pay 2 energy\n\n    assert!(game.has_pending_choice(), " + DQ + "Kasumi activation must offer hand discard" + DQ + ");\n    game.select_indices(&[0]); // discard filler"
))

# q209_kasumi_energy_available_no_target_in_discard (line ~365-372)
replacements.append((
    "    // Pay 2 energy\n    if game.has_pending_choice() {\n        game.select_generated(0);\n    }\n\n    // Discard 1 card\n    if game.has_pending_choice() {\n        game.select_indices(&[0]);\n    }",
    "    // Pay 2 energy\n    assert!(game.has_pending_choice(), " + DQ + "Kasumi activation must offer energy payment" + DQ + ");\n    game.select_generated(0);\n\n    // Discard 1 card\n    assert!(game.has_pending_choice(), " + DQ + "Kasumi activation must offer hand discard" + DQ + ");\n    game.select_indices(&[0]);"
))

# q209_kasumi_use_limit_blocks_second (line ~413-421)
replacements.append((
    "    if game.has_pending_choice() {\n        game.select_generated(0); // pay energy\n    }\n    if game.has_pending_choice() {\n        game.select_indices(&[0]); // discard\n    }\n    if game.has_pending_choice() {\n        game.select_indices(&[0]); // retrieve\n    }",
    "    assert!(game.has_pending_choice(), " + DQ + "first activation must offer energy payment" + DQ + ");\n    game.select_generated(0); // pay energy\n    assert!(game.has_pending_choice(), " + DQ + "first activation must offer hand discard" + DQ + ");\n    game.select_indices(&[0]); // discard\n    assert!(game.has_pending_choice(), " + DQ + "first activation must offer retrieval" + DQ + ");\n    game.select_indices(&[0]); // retrieve"
))

# q209_kasumi_retrieve_different_niji_live (line ~473-484)
replacements.append((
    "    if game.has_pending_choice() {\n        game.select_generated(0); // pay energy\n    }\n\n    if game.has_pending_choice() {\n        game.select_indices(&[0]); // discard niji_b\n    }\n\n    // Now waitroom has [niji_a, niji_b]. Choose niji_a (index 0).\n    if game.has_pending_choice() {\n        game.select_indices(&[0]); // retrieve niji_a\n    }",
    "    assert!(game.has_pending_choice(), " + DQ + "Kasumi activation must offer energy payment" + DQ + ");\n    game.select_generated(0); // pay energy\n\n    assert!(game.has_pending_choice(), " + DQ + "Kasumi activation must offer hand discard" + DQ + ");\n    game.select_indices(&[0]); // discard niji_b\n\n    // Now waitroom has [niji_a, niji_b]. Choose niji_a (index 0).\n    assert!(game.has_pending_choice(), " + DQ + "Kasumi retrieval select must be offered" + DQ + ");\n    game.select_indices(&[0]); // retrieve niji_a"
))

count = 0
for old, new in replacements:
    if old in src:
        src = src.replace(old, new)
        count += 1
open(p, 'w', encoding='utf-8').write(src)
print('replaced', count)
