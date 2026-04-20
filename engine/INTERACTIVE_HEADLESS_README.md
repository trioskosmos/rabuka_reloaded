# Interactive Headless Game Mode

A CLI-based headless interface for playing the Rabuka card game without the web UI.

## Features

- **Interactive Gameplay**: Choose actions manually through a command-line interface
- **Detailed Game State Display**: Shows all game information including:
  - Turn number and phase
  - Player hands, energy zones, stages
  - Live cards and success cards
  - Deck and waitroom status
- **Action List**: Displays all available actions with descriptive icons
- **Rule Validation**: Automatically checks game state against rules:
  - Victory conditions (3+ success cards vs opponent's 2 or fewer)
  - Deck composition (48 member cards + 12 live cards)
  - Stage limits (max 3 members per stage)
  - Duplicate card detection
- **Auto-advance**: Automatically advances automatic phases (Active, Energy, Draw, Live phases)

## Usage

### Running the Interactive Mode

```bash
cd engine
cargo run --bin rabuka_engine interactive
```

Or if already built:

```bash
cd engine/target/debug
.\rabuka_engine.exe interactive
```

### Commands

During gameplay, you can use:

- **Number (0-N)**: Execute the corresponding action
- **q** or **quit**: Quit the game
- **v** or **validate**: Run validation checks on current game state

### Game Flow

1. **Deck Selection**: Choose decks for both players from the available deck lists
2. **Game Initialization**: Game sets up automatically (RPS, mulligan, initial draws)
3. **Main Loop**:
   - View current game state
   - See validation warnings (if any)
   - View available actions
   - Choose an action to execute
   - Repeat until game over

## Game State Display

The game state shows:

```
=== GAME STATE ===
Turn: 1 | Phase: Main | Turn Phase: FirstAttackerNormal

--- PLAYER 1 (Player 1) ---
Hand (5 cards):
  [0] 👤 Chika Takami - LLD-001 (Cost: Some(1), Hearts: ...)
Energy Zone (3 cards):
  [0] ⚡ Energy Card - LLD-E001 [✓]
Stage:
  Left: 👤 Chika Takami - LLD-001 [✓] (Blades: 1)
  Center: (empty)
  Right: (empty)
Live Card Zone: 2 cards
  [0] 🎤 Live Card - LLD-L001 (Score: Some(10), Need: ...)
Success Live Card Zone: 0 cards
Waitroom: 0 cards
Main Deck: 43 cards
Energy Deck: 9 cards

--- VICTORY STATUS ---
Game in progress...
P1 Success Cards: 0 | P2 Success Cards: 0
==================
```

## Action List

Actions are displayed with icons for quick identification:

- ⏭ **pass**: Pass turn
- 👤 **play_member**: Play member card to stage
- ⚡ **activate_energy**: Activate energy cards
- 🎤 **play_live**: Play live card
- ✨ **activate_ability**: Activate card ability
- 💖 **cheer**: Cheer performance
- 🔄 **baton_touch**: Baton touch

## Validation

The tool validates game state against rules from `rules_1_05.txt`:

- **Rule 1.2.1.1**: Victory condition check
- **Rule 6.1.1**: Deck composition (48 members + 12 live)
- **Rule 4.5**: Stage member limits (max 3)
- **Duplicate detection**: Checks for duplicate cards in hand

## Comparison with Other Modes

### vs. Web UI
- No browser required
- Faster interaction (no network latency)
- More detailed state information
- Built-in validation

### vs. Auto Headless (`headless` command)
- Manual control instead of AI
- Interactive decision making
- Better for testing and learning
- Can validate at any point with 'v' command

## Development

The interactive headless mode is implemented in `src/bot/interactive_headless.rs`.

Key functions:
- `run_interactive_headless()`: Main game loop
- `print_game_state()`: Display current state
- `print_player_state()`: Display individual player info
- `print_actions()`: Show available actions
- `validate_game_state()`: Check against rules
- `count_stage_members()`: Helper for stage validation

## Troubleshooting

If you encounter issues:

1. **Build errors**: Run `cargo build` to compile
2. **Missing cards/decks**: Ensure `../cards/cards.json` and `../game/decks/` exist
3. **Stuck in phase**: Use 'v' to validate and check for state issues
