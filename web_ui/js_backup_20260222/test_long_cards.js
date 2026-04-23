const { translateAbility } = require('./ability_translator.js');

const cards = [
    {
        name: 'PL!-sd1-003-SD (南 ことり/Kotori)',
        text: `TRIGGER: ON_PLAY
EFFECT: RECOVER_MEMBER(1) -> CARD_DISCARD {COST_MAX=4, GROUP="μ's"}

TRIGGER: ON_LIVE_START
COST: DISCARD_HAND(1) (Optional)
EFFECT: SELECT_MODE(1) -> SELF
    1: ADD_HEARTS(1) -> SELF {COLOR="PINK", UNTIL="live_end"}
    2: ADD_HEARTS(1) -> SELF {COLOR="RED", UNTIL="live_end"}
    3: ADD_HEARTS(1) -> SELF {COLOR="YELLOW", UNTIL="live_end"}
    4: ADD_HEARTS(1) -> SELF {COLOR="GREEN", UNTIL="live_end"}
    5: ADD_HEARTS(1) -> SELF {COLOR="BLUE", UNTIL="live_end"}
    6: ADD_HEARTS(1) -> SELF {COLOR="PURPLE", UNTIL="live_end"}`
    },
    {
        name: 'PL!SP-bp1-024-L (Tiny Stars)',
        text: `TRIGGER: ON_LIVE_START
EFFECT: ADD_HEARTS(1) -> MEMBER_NAMED {NAME="澁谷かのん", COLOR="BLUE", UNTIL="live_end"}; ADD_BLADES(1) -> MEMBER_NAMED {NAME="澁谷かのん", UNTIL="live_end"}
EFFECT: ADD_HEARTS(1) -> MEMBER_NAMED {NAME="唐可可", COLOR="PINK", UNTIL="live_end"}; ADD_BLADES(1) -> MEMBER_NAMED {NAME="唐可可", UNTIL="live_end"}`
    },
    {
        name: 'LL-bp3-001-R＋ (园田海未/Umi & 善子/Yoshiko & 璃奈/Rina)',
        text: `TRIGGER: ACTIVATED
(Once per turn)
COST: MOVE_TO_DECK(6) -> CARD_DISCARD {NAMES=["園田海未", "津島善子", "天王寺璃奈"], TO="bottom", SHUFFLE=TRUE}
EFFECT: ACTIVATE_MEMBER(6) -> PLAYER {FILTER="energy"}

TRIGGER: ON_LIVE_START
COST: ENERGY(6) (Optional)
EFFECT: ADD_BLADES(3) -> SELF {UNTIL="live_end"}`
    }
];

cards.forEach(c => {
    console.log(`\n==================================================`);
    console.log(`CARD: ${c.name}`);
    console.log(`==================================================`);
    console.log(`[JP VERSION]`);
    console.log(translateAbility(c.text, 'jp'));
    console.log(`\n[EN VERSION]`);
    console.log(translateAbility(c.text, 'en'));
});
