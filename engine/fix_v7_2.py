with open('src/bot/strategy_v7.rs', 'r') as f:
    content = f.read()

old = """        // Development in HEARTS and BLADES. Blades are the engine of the yell
        // flip: every extra blade is a fresh Binomial trial that can supply the
        // hearts a check needs, so board growth compounds into live-set power.
        let d_stage = stage_hearts_of(my_sim, db) - base_stage;
        val += 3.0 * d_stage as f64;
        if d_stage != 0 {
            parts.push(format!("hearts{d_stage:+}"));
        }
        let d_blades = total_blades_of(my_sim, &sim, db) - base_blades;
        val += 6.0 * d_blades as f64;
        if d_blades != 0 {
            parts.push(format!("blades{d_blades:+}"));"""

new = """        // Development in HEARTS and BLADES. Blades are the engine of the yell
        // flip: every extra blade is a fresh Binomial trial that can supply the
        // hearts a check needs, so board growth compounds into live-set power.
        let d_stage = stage_hearts_of(my_sim, db) - base_stage;
        val += 15.0 * d_stage as f64;
        if d_stage != 0 {
            parts.push(format!("hearts{d_stage:+}"));
        }
        let d_blades = total_blades_of(my_sim, &sim, db) - base_blades;
        val += 20.0 * d_blades as f64;
        if d_blades != 0 {
            parts.push(format!("blades{d_blades:+}"));"""

if old in content:
    content = content.replace(old, new)
    with open('src/bot/strategy_v7.rs', 'w') as f:
        f.write(content)
    print('Replaced successfully')
else:
    print('NOT FOUND')
    idx = content.find('Development in HEARTS')
    if idx >= 0:
        print(repr(content[idx:idx+500]))