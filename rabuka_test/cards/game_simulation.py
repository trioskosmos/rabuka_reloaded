"""Basic game simulation for Rabuka card game."""
import json
import random
from dataclasses import dataclass, field
from typing import List, Optional, Dict, Any
from enum import Enum

class CardType(Enum):
    ENERGY = "エネルギー"
    MEMBER = "メンバー"
    LIVE = "ライブ"

class TurnPhase(Enum):
    FIRST_ATTACKER_NORMAL = "first_attacker_normal"  # Rule 7.3.2.1
    SECOND_ATTACKER_NORMAL = "second_attacker_normal"  # Rule 7.3.2.1
    LIVE = "live"  # Rule 7.8

class Phase(Enum):
    # Normal phase sub-phases (Rule 7.3.3)
    ACTIVE = "active"
    ENERGY = "energy"
    DRAW = "draw"
    MAIN = "main"
    
    # Live phase sub-phases (Rule 8.1.2)
    LIVE_CARD_SET = "live_card_set"
    FIRST_ATTACKER_PERFORMANCE = "first_attacker_performance"
    SECOND_ATTACKER_PERFORMANCE = "second_attacker_performance"
    LIVE_VICTORY_DETERMINATION = "live_victory_determination"

class Orientation(Enum):
    WAIT = "wait"
    ACTIVE = "active"

class FaceState(Enum):
    FACE_UP = "face_up"   # Rule 4.3.3.1
    FACE_DOWN = "face_down"  # Rule 4.3.3.2

# Rule 9.1: Ability Types
class AbilityType(Enum):
    ACTIVATION = "activation"  # 起動能力
    AUTOMATIC = "automatic"    # 自動能力
    CONTINUOUS = "continuous"  # 常時能力

# Rule 9.2: Effect Types
class EffectType(Enum):
    ONE_SHOT = "one_shot"        # 単発効果
    CONTINUOUS_EFFECT = "continuous"  # 継続効果
    REPLACEMENT = "replacement"   # 置換効果

class MemberArea(Enum):
    LEFT = "left"
    CENTER = "center"
    RIGHT = "right"

@dataclass
class Card:
    card_no: str
    name: str
    card_type: CardType
    cost: Optional[int] = None
    base_heart: Optional[Dict[str, int]] = None
    blade: int = 0
    score: Optional[int] = None
    need_heart: Optional[Dict[str, int]] = None
    ability: str = ""
    abilities: List[Dict[str, Any]] = field(default_factory=list)  # Parsed abilities from parser
    
    def total_blades(self) -> int:
        return self.blade
    
    def total_hearts(self) -> int:
        if self.base_heart:
            return sum(self.base_heart.values())
        return 0
    
    def get_score(self) -> int:
        return self.score or 0
    
    def satisfies_required_hearts(self, available_hearts: Dict[str, int]) -> bool:
        """Check if this live card's required hearts can be satisfied by available hearts (rule 2.11.3)."""
        if not self.need_heart:
            return True  # No required hearts, automatically satisfied
        
        # Check each required heart color
        for color, required_count in self.need_heart.items():
            if color == 'heart01':  # Special case for heart01 (can be any color)
                # For heart01, we need total hearts >= required_count
                total_hearts = sum(available_hearts.values())
                if total_hearts < required_count:
                    return False
            else:
                # For specific colors, need that color >= required_count
                if available_hearts.get(color, 0) < required_count:
                    return False
        
        # Also check total hearts requirement (rule 2.11.3.2)
        total_required = sum(self.need_heart.values())
        total_available = sum(available_hearts.values())
        if total_available < total_required:
            return False
        
        return True
    
    def consume_required_hearts(self, available_hearts: Dict[str, int]) -> Dict[str, int]:
        """Consume required hearts from available hearts. Returns remaining hearts."""
        if not self.need_heart:
            return available_hearts
        
        remaining = available_hearts.copy()
        
        for color, required_count in self.need_heart.items():
            if color == 'heart01':
                # Consume from any colors
                remaining_needed = required_count
                for heart_color in sorted(remaining.keys(), key=lambda k: remaining[k], reverse=True):
                    if remaining_needed <= 0:
                        break
                    consume = min(remaining[heart_color], remaining_needed)
                    remaining[heart_color] -= consume
                    remaining_needed -= consume
            else:
                # Consume from specific color
                if remaining.get(color, 0) >= required_count:
                    remaining[color] -= required_count
                else:
                    # Should not happen if satisfies_required_hearts returned True
                    pass
        
        return remaining
    
    def is_member(self) -> bool:
        return self.card_type == CardType.MEMBER
    
    def is_energy(self) -> bool:
        return self.card_type == CardType.ENERGY
    
    def is_live(self) -> bool:
        return self.card_type == CardType.LIVE

@dataclass
class CardInZone:
    card: Card
    orientation: Optional[Orientation] = None
    face_state: Optional[FaceState] = None
    energy_underneath: List[Card] = field(default_factory=list)
    
    def total_blades(self) -> int:
        total = self.card.total_blades()
        for energy in self.energy_underneath:
            total += energy.total_blades()
        return total

@dataclass
class Stage:
    # Rule 4.4: Stage is the unified area of member areas
    left: Optional[CardInZone] = None
    center: Optional[CardInZone] = None
    right: Optional[CardInZone] = None
    
    def is_full(self) -> bool:
        return self.left is not None and self.center is not None and self.right is not None
    
    def get_available_hearts(self) -> Dict[str, int]:
        hearts = {}
        for card_in_zone in [self.left, self.center, self.right]:
            if card_in_zone and card_in_zone.card.base_heart:
                for color, count in card_in_zone.card.base_heart.items():
                    hearts[color] = hearts.get(color, 0) + count
        return hearts
    
    def total_blades(self) -> int:
        total = 0
        for card_in_zone in [self.left, self.center, self.right]:
            if card_in_zone:
                total += card_in_zone.total_blades()
        return total
    
    def activate_all_cards(self):
        for card_in_zone in [self.left, self.center, self.right]:
            if card_in_zone:
                card_in_zone.orientation = Orientation.ACTIVE

# Rule 4.11: Hand - non-public zone for unused cards
@dataclass
class Hand:
    cards: List[Card] = field(default_factory=list)
    
    def add_card(self, card: Card):
        self.cards.append(card)
    
    def remove_card(self, card: Card) -> bool:
        if card in self.cards:
            self.cards.remove(card)
            return True
        return False
    
    def len(self) -> int:
        return len(self.cards)

# Rule 4.7: Energy Zone - public zone for energy cards
@dataclass
class EnergyZone:
    cards: List[CardInZone] = field(default_factory=list)
    
    def add_card(self, card: Card):
        card_in_zone = CardInZone(card=card, orientation=Orientation.ACTIVE)
        self.cards.append(card_in_zone)
    
    def total_blades(self) -> int:
        return sum(card.total_blades() for card in self.cards if card.orientation == Orientation.ACTIVE)
    
    def pay_blades(self, amount: int) -> bool:
        remaining = amount
        for card in self.cards:
            if remaining <= 0:
                break
            if card.orientation == Orientation.ACTIVE:
                card.orientation = Orientation.WAIT
                remaining -= 1
        return remaining == 0

# Rule 4.6: Live Card Zone - public zone for live cards being performed
@dataclass
class LiveCardZone:
    cards: List[CardInZone] = field(default_factory=list)
    
    def add_card(self, card: Card):
        card_in_zone = CardInZone(card=card, orientation=Orientation.ACTIVE, face_state=FaceState.FACE_DOWN)
        self.cards.append(card_in_zone)
    
    def clear(self):
        self.cards.clear()
    
    def len(self) -> int:
        return len(self.cards)
    
    def is_empty(self) -> bool:
        return len(self.cards) == 0
    
    def calculate_score(self) -> int:
        return sum(card.card.get_score() for card in self.cards)

# Rule 4.10: Success Live Card Zone - public zone for successful live cards
@dataclass
class SuccessLiveCardZone:
    cards: List[Card] = field(default_factory=list)
    
    def add_card(self, card: Card):
        self.cards.append(card)
    
    def len(self) -> int:
        return len(self.cards)

# Rule 4.8: Main Deck - non-public zone, ordered
@dataclass
class MainDeck:
    cards: List[Card] = field(default_factory=list)
    
    def shuffle(self):
        random.shuffle(self.cards)
    
    def draw(self) -> Optional[Card]:
        if self.cards:
            return self.cards.pop(0)
        return None
    
    def len(self) -> int:
        return len(self.cards)
    
    def is_empty(self) -> bool:
        return len(self.cards) == 0

# Rule 4.9: Energy Deck - non-public zone, not ordered
@dataclass
class EnergyDeck:
    cards: List[Card] = field(default_factory=list)
    
    def draw(self) -> Optional[Card]:
        if self.cards:
            return self.cards.pop(0)
        return None
    
    def len(self) -> int:
        return len(self.cards)

# Rule 4.12: Discard - public zone for used cards
@dataclass
class Discard:
    cards: List[Card] = field(default_factory=list)
    
    def add_card(self, card: Card):
        self.cards.append(card)
    
    def shuffle_and_move_to_deck(self, deck: MainDeck):
        random.shuffle(self.cards)
        deck.cards.extend(self.cards)
        self.cards.clear()

# Rule 4.13: Exclusion Zone - public zone for removed cards
@dataclass
class ExclusionZone:
    cards: List[Card] = field(default_factory=list)

# Rule 4.14: Resolution Zone - shared public zone for temporary card placement
@dataclass
class ResolutionZone:
    cards: List[CardInZone] = field(default_factory=list)
    
    def clear(self):
        self.cards.clear()

@dataclass
class Player:
    name: str
    is_first_attacker: bool
    hand: Hand = field(default_factory=Hand)
    energy_zone: EnergyZone = field(default_factory=EnergyZone)
    stage: Stage = field(default_factory=Stage)
    live_card_zone: LiveCardZone = field(default_factory=LiveCardZone)
    success_live_card_zone: SuccessLiveCardZone = field(default_factory=SuccessLiveCardZone)
    main_deck: MainDeck = field(default_factory=MainDeck)
    energy_deck: EnergyDeck = field(default_factory=EnergyDeck)
    discard: Discard = field(default_factory=Discard)
    exclusion_zone: ExclusionZone = field(default_factory=ExclusionZone)
    blade_hearts_from_cheer: Dict[str, int] = field(default_factory=dict)  # Rule 8.3.14: Blade hearts from cheer
    
    def draw_card(self) -> Optional[Card]:
        return self.main_deck.draw()
    
    def draw_energy(self) -> Optional[Card]:
        return self.energy_deck.draw()
    
    def activate_all_energy(self):
        for card_in_zone in self.energy_zone.cards:
            card_in_zone.orientation = Orientation.ACTIVE
    
    def total_blades_in_energy(self) -> int:
        return self.energy_zone.total_blades()
    
    def can_pay_blades(self, amount: int) -> bool:
        return self.total_blades_in_energy() >= amount
    
    def calculate_live_score(self) -> int:
        return self.live_card_zone.calculate_score()
    
    def calculate_live_owned_hearts(self) -> Dict[str, int]:
        """Calculate live-owned hearts from stage members and blade hearts from cheer (rule 8.3.14)."""
        hearts = {}
        
        # Add hearts from members on stage
        for card_in_zone in [self.stage.left, self.stage.center, self.stage.right]:
            if card_in_zone and card_in_zone.card.base_heart:
                for color, count in card_in_zone.card.base_heart.items():
                    hearts[color] = hearts.get(color, 0) + count
        
        # Add blade hearts from cheer
        for color, count in self.blade_hearts_from_cheer.items():
            hearts[color] = hearts.get(color, 0) + count
        
        return hearts

@dataclass
class GameState:
    player1: Player
    player2: Player
    current_turn_phase: TurnPhase = TurnPhase.FIRST_ATTACKER_NORMAL
    current_phase: Phase = Phase.ACTIVE
    turn_number: int = 1
    
    def active_player(self) -> Player:
        # Rule 7.2: Determine active player based on turn phase
        if self.current_turn_phase == TurnPhase.FIRST_ATTACKER_NORMAL:
            return self.first_attacker()
        elif self.current_turn_phase == TurnPhase.SECOND_ATTACKER_NORMAL:
            return self.second_attacker()
        else:  # LIVE phase
            # Rule 7.2.1.2: In phases without specified turn player, first attacker is active
            return self.first_attacker()
    
    def first_attacker(self) -> Player:
        if self.player1.is_first_attacker:
            return self.player1
        return self.player2
    
    def second_attacker(self) -> Player:
        if self.player1.is_first_attacker:
            return self.player2
        return self.player1

class SimpleAI:
    def __init__(self, name: str):
        self.name = name
    
    def choose_action(self, game_state: GameState) -> str:
        phase = game_state.current_phase
        player = game_state.active_player()
        
        if phase == Phase.ACTIVE:
            return "activate_energy"
        
        elif phase == Phase.ENERGY:
            # Prioritize playing energy to build blades
            energy_cards = [c for c in player.hand.cards if c.is_energy()]
            if energy_cards:
                return "play_energy"
            return "draw_energy"
        
        elif phase == Phase.DRAW:
            return "draw_card"
        
        elif phase == Phase.MAIN:
            # Prioritize playing member cards to stage to get hearts
            member_cards = [c for c in player.hand.cards if c.is_member()]
            
            # Try to play affordable member cards
            affordable_members = [c for c in member_cards if not c.cost or player.can_pay_blades(c.cost)]
            
            if affordable_members:
                # Find an empty spot on stage
                stage = player.stage
                if stage.left is None:
                    return f"play_member_left"
                elif stage.center is None:
                    return f"play_member_center"
                elif stage.right is None:
                    return f"play_member_right"
            
            # If no affordable members, try playing energy
            energy_cards = [c for c in player.hand.cards if c.is_energy()]
            if energy_cards:
                return "play_energy"
            
            return "pass"
        
        elif phase == Phase.LIVE_CARD_SET:
            # Live card set is handled automatically in advance_phase
            return "pass"
        
        elif phase in [Phase.FIRST_ATTACKER_PERFORMANCE, Phase.SECOND_ATTACKER_PERFORMANCE]:
            # Performance is handled automatically in advance_phase
            return "pass"
        
        elif phase == Phase.LIVE_VICTORY_DETERMINATION:
            # Victory determination is handled automatically in advance_phase
            return "pass"
        
        return "pass"

class GameSimulation:
    def __init__(self, cards_data: Dict[str, Any], output_file: str = "game_output.txt"):
        self.cards_data = cards_data
        self.game_state: Optional[GameState] = None
        self.ai1 = SimpleAI("Player 1")
        self.ai2 = SimpleAI("Player 2")
        self.log_entries = []
        self.resolution_zone = ResolutionZone()
    
    # Rule 10: Rule Processing
    def check_refresh_needed(self, player: Player) -> bool:
        """Rule 10.2.2.1: Check if refresh is needed (main deck empty, discard has cards)"""
        return player.main_deck.is_empty() and len(player.discard.cards) > 0
    
    def execute_refresh(self, player: Player):
        """Rule 10.2.3: Execute refresh - shuffle discard into main deck"""
        self.log(f"  {player.name} executes refresh")
        player.discard.shuffle_and_move_to_deck(player.main_deck)
    
    def check_victory(self, player: Player) -> bool:
        """Rule 10.3: Check victory condition (3+ successful live cards)"""
        return player.success_live_card_zone.len() >= 3
    
    def check_duplicate_members(self, player: Player) -> bool:
        """Rule 10.4: Check for duplicate members in same area"""
        # Simplified: check if any area has multiple members
        # In proper implementation, would track when members are placed
        return False  # Simplified for now
    
    def process_duplicate_members(self, player: Player):
        """Rule 10.4.1: Remove duplicate members, keep most recent"""
        # Simplified: not implemented yet
        pass
    
    def check_invalid_cards(self, player: Player) -> bool:
        """Rule 10.5: Check for invalid cards in zones"""
        # Rule 10.5.1: Non-live cards in live card zone
        # Rule 10.5.2: Non-energy cards in energy zone
        # Rule 10.5.3: Energy cards without member above in member area
        return False  # Simplified for now
    
    def process_invalid_cards(self, player: Player):
        """Rule 10.5: Process invalid cards - move to discard or energy deck"""
        # Simplified: not implemented yet
        pass
    
    def process_all_rules(self):
        """Process all rule processing at check timing"""
        if self.game_state:
            for player in [self.game_state.player1, self.game_state.player2]:
                # Rule 10.2: Refresh
                if self.check_refresh_needed(player):
                    self.execute_refresh(player)
                
                # Rule 10.4-10.5: Invalid card processing
                if self.check_invalid_cards(player):
                    self.process_invalid_cards(player)
        self.output_file = output_file
    
    def log(self, message: str):
        """Add a log entry."""
        self.log_entries.append(message)
        print(message)
    
    def save_logs_to_file(self):
        """Save all log entries to a file."""
        with open(self.output_file, 'w', encoding='utf-8') as f:
            for entry in self.log_entries:
                f.write(entry + '\n')
        print(f"\nGame logs saved to {self.output_file}")
    
    def log_card_played(self, player: Player, card: Card, location: str):
        """Log when a card is played."""
        heart_info = ""
        if card.base_heart:
            heart_str = ", ".join([f"{color}: {count}" for color, count in card.base_heart.items()])
            heart_info = f" [Hearts: {heart_str}]"
        
        self.log(f"  {player.name} plays {card.name} ({card.card_type.value}, cost: {card.cost or 0}, blade: {card.blade}) to {location}{heart_info}")
    
    def log_heart_vector(self, player: Player):
        """Log the total heart vector on the player's stage."""
        hearts = player.stage.get_available_hearts()
        if hearts:
            heart_str = ", ".join([f"{color}: {count}" for color, count in sorted(hearts.items())])
            total = sum(hearts.values())
            self.log(f"  {player.name} stage hearts: {total} total ({heart_str})")
        else:
            self.log(f"  {player.name} stage hearts: 0 total")
    
    def log_performance_summary(self, player: Player):
        """Log a summary of the player's performance."""
        stage_hearts = player.stage.get_available_hearts()
        live_hearts = {}
        for card in player.live_card_zone:
            if card.base_heart:
                for color, count in card.base_heart.items():
                    live_hearts[color] = live_hearts.get(color, 0) + count
        
        stage_total = sum(stage_hearts.values())
        live_total = sum(live_hearts.values())
        
        if stage_hearts:
            stage_str = ", ".join([f"{color}: {count}" for color, count in sorted(stage_hearts.items())])
            self.log(f"  {player.name} stage: {stage_total} hearts ({stage_str})")
        else:
            self.log(f"  {player.name} stage: 0 hearts")
        
        if live_hearts:
            live_str = ", ".join([f"{color}: {count}" for color, count in sorted(live_hearts.items())])
            self.log(f"  {player.name} live zone: {live_total} hearts ({live_str})")
        else:
            self.log(f"  {player.name} live zone: 0 hearts")
        
        score = player.calculate_live_score()
        self.log(f"  {player.name} live score: {score}")
    
    def log_game_state(self):
        """Log the current game state."""
        self.log(f"\n=== Game State (Turn {self.game_state.turn_number}, Phase: {self.game_state.current_turn_phase.value} - {self.game_state.current_phase.value}) ===")
        self.log(f"Active player: {self.game_state.active_player().name}")
        
        for player in [self.game_state.player1, self.game_state.player2]:
            self.log(f"\n{player.name}:")
            self.log(f"  Hand: {player.hand.len()} cards")
            self.log(f"  Energy zone: {len(player.energy_zone.cards)} cards (blades: {player.total_blades_in_energy()})")
            self.log(f"  Stage: {player.stage.left.card.name if player.stage.left else 'empty'} | {player.stage.center.card.name if player.stage.center else 'empty'} | {player.stage.right.card.name if player.stage.right else 'empty'}")
            self.log(f"  Live card zone: {player.live_card_zone.len()} cards")
            self.log(f"  Success live card zone: {player.success_live_card_zone.len()} cards")
            self.log(f"  Player stage hearts: {sum(player.stage.get_available_hearts().values())} total")
    
    def load_cards(self) -> List[Card]:
        cards = []
        for card_no, card_data in self.cards_data.items():
            card_type_str = card_data.get("type", "")
            if card_type_str == "エネルギー":
                card_type = CardType.ENERGY
            elif card_type_str == "メンバー":
                card_type = CardType.MEMBER
            elif card_type_str == "ライブ":
                card_type = CardType.LIVE
            else:
                continue
            
            card = Card(
                card_no=card_no,
                name=card_data.get("name", ""),
                card_type=card_type,
                cost=card_data.get("cost"),
                base_heart=card_data.get("base_heart"),
                blade=card_data.get("blade", 0),
                score=card_data.get("score"),
                need_heart=card_data.get("need_heart"),
                ability=card_data.get("ability", "")
            )
            cards.append(card)
        return cards
    
    def build_deck(self, cards: List[Card]) -> MainDeck:
        # Build a deck with 48 member cards and 12 live cards per rules 6.1.1.1
        member_cards = [c for c in cards if c.is_member()]
        live_cards = [c for c in cards if c.is_live()]
        
        deck_cards = []
        deck_cards.extend(member_cards[:48])
        deck_cards.extend(live_cards[:12])
        random.shuffle(deck_cards)
        
        deck = MainDeck(cards=deck_cards)
        return deck
    
    def build_energy_deck(self, cards: List[Card]) -> EnergyDeck:
        # Build energy deck with 12 energy cards per rules 6.1.1.3
        energy_cards = [c for c in cards if c.is_energy()]
        
        deck_cards = energy_cards[:12]
        random.shuffle(deck_cards)
        
        deck = EnergyDeck(cards=deck_cards)
        return deck
    
    def initialize_game(self):
        """Initialize game per rule 6.2"""
        cards = self.load_cards()
        
        # Rule 6.2.1.2-6.2.1.3: Place decks
        player1 = Player(
            name="Player 1",
            is_first_attacker=True,  # Will be randomized
            main_deck=self.build_deck(cards),
            energy_deck=self.build_energy_deck(cards)
        )
        
        player2 = Player(
            name="Player 2",
            is_first_attacker=False,
            main_deck=self.build_deck(cards),
            energy_deck=self.build_energy_deck(cards)
        )
        
        # Rule 6.2.1.2: Shuffle main decks
        player1.main_deck.shuffle()
        player2.main_deck.shuffle()
        
        # Rule 6.2.1.4: Randomly select first attacker
        first_attacker = random.choice([player1, player2])
        first_attacker.is_first_attacker = True
        other_player = player2 if first_attacker == player1 else player1
        other_player.is_first_attacker = False
        
        self.log(f"  {first_attacker.name} is the first attacker")
        
        # Rule 6.2.1.5: Draw 6 cards to hand
        for _ in range(6):
            card1 = player1.draw_card()
            if card1:
                player1.hand.add_card(card1)
            card2 = player2.draw_card()
            if card2:
                player2.hand.add_card(card2)
        
        # Rule 6.2.1.6: Mulligan (simplified - skip for now, players keep initial hands)
        
        # Rule 6.2.1.7: Draw 3 energy cards to energy zone
        for _ in range(3):
            energy1 = player1.draw_energy()
            if energy1:
                player1.energy_zone.add_card(energy1)
            energy2 = player2.draw_energy()
            if energy2:
                player2.energy_zone.add_card(energy2)
        
        self.game_state = GameState(
            player1=player1,
            player2=player2,
            current_turn_phase=TurnPhase.FIRST_ATTACKER_NORMAL,
            current_phase=Phase.ACTIVE
        )
        
        print(f"Game initialized!")
        print(f"Player 1 hand: {player1.hand.len()} cards")
        print(f"Player 2 hand: {player2.hand.len()} cards")
        print(f"Player 1 energy zone: {len(player1.energy_zone.cards)} cards")
        print(f"Player 2 energy zone: {len(player2.energy_zone.cards)} cards")
    
    def execute_action(self, action: str):
        if not self.game_state:
            return
        
        player = self.game_state.active_player()
        phase = self.game_state.current_phase
        
        self.log(f"{player.name} performs: {action} (Phase: {phase.value})")
        
        if action == "activate_energy":
            player.activate_all_energy()
            self.log(f"  All energy activated for {player.name}")
        
        elif action == "draw_energy":
            energy = player.draw_energy()
            if energy:
                player.energy_zone.add_card(energy)
                self.log(f"  Drew energy: {energy.name}")
        
        elif action == "draw_card":
            card = player.draw_card()
            if card:
                player.hand.add_card(card)
                self.log(f"  Drew card: {card.name}")
        
        elif action == "play_energy":
            energy_cards = [c for c in player.hand.cards if c.is_energy()]
            if energy_cards:
                card = energy_cards[0]
                player.hand.remove_card(card)
                player.energy_zone.add_card(card)
                self.log_card_played(player, card, "energy zone")
        
        elif action == "play_member_left":
            self._play_member_to_stage(player, MemberArea.LEFT)
        
        elif action == "play_member_center":
            self._play_member_to_stage(player, MemberArea.CENTER)
        
        elif action == "play_member_right":
            self._play_member_to_stage(player, MemberArea.RIGHT)
        
        elif action == "place_live_card":
            live_cards = [c for c in player.hand.cards if c.is_live()]
            if live_cards and len(player.live_card_zone.cards) < 5:
                card = live_cards[0]
                player.hand.remove_card(card)
                player.live_card_zone.add_card(card)
                self.log_card_played(player, card, "live card zone")
        
        elif action == "pass":
            self.log(f"  Passing...")
    
    def _play_member_to_stage(self, player: Player, area: MemberArea):
        member_cards = [c for c in player.hand.cards if c.is_member()]
        if not member_cards:
            return
        
        card = member_cards[0]
        
        # Check if player can pay cost
        if card.cost and not player.can_pay_blades(card.cost):
            self.log(f"  Cannot afford {card.name} (cost: {card.cost}, blades: {player.total_blades_in_energy()})")
            return
        
        # Check heart requirements
        if card.need_heart:
            available_hearts = player.stage.get_available_hearts()
            required_hearts = card.need_heart
            can_satisfy = all(
                available_hearts.get(color, 0) >= count
                for color, count in required_hearts.items()
            )
            if not can_satisfy:
                self.log(f"  Cannot satisfy heart requirements for {card.name}")
                return
        
        # Check if area is available
        stage = player.stage
        if area == MemberArea.LEFT and stage.left is not None:
            self.log(f"  Left side is occupied")
            return
        elif area == MemberArea.CENTER and stage.center is not None:
            self.log(f"  Center is occupied")
            return
        elif area == MemberArea.RIGHT and stage.right is not None:
            self.log(f"  Right side is occupied")
            return
        
        # Play the card
        player.hand.remove_card(card)
        card_in_zone = CardInZone(card=card, orientation=Orientation.WAIT)
        
        if area == MemberArea.LEFT:
            stage.left = card_in_zone
        elif area == MemberArea.CENTER:
            stage.center = card_in_zone
        elif area == MemberArea.RIGHT:
            stage.right = card_in_zone
        
        self.log_card_played(player, card, f"stage {area.value}")
        self.log_heart_vector(player)
    
    def advance_phase(self):
        """Advance phase according to rules 7.1.2, 7.3.3, and 8.1.2"""
        if not self.game_state:
            return
        
        current_phase = self.game_state.current_phase
        current_turn_phase = self.game_state.current_turn_phase
        player = self.game_state.active_player()
        
        # Handle normal phase sub-phases (Rule 7.3.3)
        if current_turn_phase in [TurnPhase.FIRST_ATTACKER_NORMAL, TurnPhase.SECOND_ATTACKER_NORMAL]:
            if current_phase == Phase.ACTIVE:
                # Rule 7.4: Activate all energy and stage cards
                player.activate_all_energy()
                player.stage.activate_all_cards()
                # Rule 7.4.3: Check timing
                self.process_all_rules()
                self.game_state.current_phase = Phase.ENERGY
                self.log(f"  Phase advanced to: {current_phase.value}")
                return
            
            elif current_phase == Phase.ENERGY:
                # Rule 7.5.1: Check timing
                self.process_all_rules()
                # Rule 7.5.2: Draw energy card
                if energy_card := player.draw_energy():
                    player.energy_zone.add_card(energy_card)
                    self.log(f"  Drew energy: {energy_card.name}")
                # Rule 7.5.3: Check timing
                self.process_all_rules()
                self.game_state.current_phase = Phase.DRAW
                self.log(f"  Phase advanced to: {current_phase.value}")
                return
            
            elif current_phase == Phase.DRAW:
                # Rule 7.6.1: Check timing
                self.process_all_rules()
                # Rule 7.6.2: Draw card
                if card := player.draw_card():
                    player.hand.add_card(card)
                    self.log(f"  Drew card: {card.name}")
                # Rule 7.6.3: Check timing
                self.process_all_rules()
                self.game_state.current_phase = Phase.MAIN
                self.log(f"  Phase advanced to: {current_phase.value}")
                return
            
            elif current_phase == Phase.MAIN:
                # Rule 7.7.1: Check timing
                self.process_all_rules()
                # Rule 7.7: Main phase complete, advance to next turn phase
                if current_turn_phase == TurnPhase.FIRST_ATTACKER_NORMAL:
                    self.game_state.current_turn_phase = TurnPhase.SECOND_ATTACKER_NORMAL
                    self.game_state.current_phase = Phase.ACTIVE
                    self.log(f"  Phase advanced to: Second Attacker Normal Phase - ACTIVE")
                else:
                    self.game_state.current_turn_phase = TurnPhase.LIVE
                    self.game_state.current_phase = Phase.LIVE_CARD_SET
                    self.log(f"  Phase advanced to: Live Phase - LIVE_CARD_SET")
                return
        
        # Handle live phase sub-phases (Rule 8.1.2)
        elif current_turn_phase == TurnPhase.LIVE:
            if current_phase == Phase.LIVE_CARD_SET:
                # Rule 8.2.1: Check timing
                self.process_all_rules()
                # Rule 8.2: Both players set live cards
                # First attacker sets cards
                first_attacker = self.game_state.first_attacker()
                self._player_set_live_cards(first_attacker)
                # Rule 8.2.3: Check timing
                self.process_all_rules()
                # Second attacker sets cards
                second_attacker = self.game_state.second_attacker()
                self._player_set_live_cards(second_attacker)
                # Rule 8.2.5: Check timing
                self.process_all_rules()
                self.game_state.current_phase = Phase.FIRST_ATTACKER_PERFORMANCE
                self.log(f"  Phase advanced to: FIRST_ATTACKER_PERFORMANCE")
                return
            
            elif current_phase == Phase.FIRST_ATTACKER_PERFORMANCE:
                # Rule 8.3.3: Check timing
                self.process_all_rules()
                # Rule 8.3: First attacker performs
                first_attacker = self.game_state.first_attacker()
                self._player_perform_live(first_attacker)
                # Rule 8.3.5: Check timing
                self.process_all_rules()
                # Rule 8.3.9: Check timing
                self.process_all_rules()
                # Rule 8.3.13: Check timing
                self.process_all_rules()
                # Rule 8.3.17: Check timing
                self.process_all_rules()
                self.game_state.current_phase = Phase.SECOND_ATTACKER_PERFORMANCE
                self.log(f"  Phase advanced to: SECOND_ATTACKER_PERFORMANCE")
                return
            
            elif current_phase == Phase.SECOND_ATTACKER_PERFORMANCE:
                # Rule 8.3.3: Check timing
                self.process_all_rules()
                # Rule 8.3: Second attacker performs
                second_attacker = self.game_state.second_attacker()
                self._player_perform_live(second_attacker)
                # Rule 8.3.5: Check timing
                self.process_all_rules()
                # Rule 8.3.9: Check timing
                self.process_all_rules()
                # Rule 8.3.13: Check timing
                self.process_all_rules()
                # Rule 8.3.17: Check timing
                self.process_all_rules()
                self.game_state.current_phase = Phase.LIVE_VICTORY_DETERMINATION
                self.log(f"  Phase advanced to: LIVE_VICTORY_DETERMINATION")
                return
            
            elif current_phase == Phase.LIVE_VICTORY_DETERMINATION:
                # Rule 8.4.1: Check timing
                self.process_all_rules()
                # Rule 8.4: Determine live victory
                self._determine_live_victory()
                # Rule 8.4.5: Check timing
                self.process_all_rules()
                # Rule 8.4.9: Check timing
                self.process_all_rules()
                # Rule 8.4.11: Check timing
                self.process_all_rules()
                # Rule 8.4.13: Determine next first attacker
                # Rule 8.4.14: End turn
                self.game_state.turn_number += 1
                self.game_state.current_turn_phase = TurnPhase.FIRST_ATTACKER_NORMAL
                self.game_state.current_phase = Phase.ACTIVE
                self.log(f"  Phase advanced to: Turn {self.game_state.turn_number} - First Attacker Normal Phase - ACTIVE")
                return
    
    def _player_set_live_cards(self, player: Player):
        """Rule 8.2: Player sets live cards face-down and draws equal amount"""
        # Simplified: AI chooses up to 3 live cards to set
        live_cards = [c for c in player.hand.cards if c.is_live()]
        if not live_cards:
            return
        
        # Set up to 3 cards
        cards_to_set = min(3, len(live_cards))
        for _ in range(cards_to_set):
            card = live_cards.pop(0)
            player.hand.remove_card(card)
            player.live_card_zone.add_card(card)
        
        # Draw equal amount
        for _ in range(cards_to_set):
            if card := player.draw_card():
                player.hand.add_card(card)
        
        self.log(f"  {player.name} set {cards_to_set} live card(s) face-down and drew {cards_to_set}")
    
    def _player_perform_live(self, player: Player):
        """Rule 8.3: Player performs live - check heart requirements"""
        # Rule 8.3.4: Reveal cards, discard non-live cards
        player.live_card_zone.cards = [c for c in player.live_card_zone.cards if c.card.is_live()]
        
        # Rule 8.3.6: If no live cards, end performance
        if player.live_card_zone.is_empty():
            self.log(f"  {player.name} has no live cards - performance ends")
            return
        
        # Rule 8.3.14: Calculate live-owned hearts
        live_owned_hearts = player.calculate_live_owned_hearts()
        
        # Rule 8.3.10-8.3.11: エール (cheer) - draw cards based on blades
        total_blades = player.stage.total_blades()
        blade_hearts = {}
        for _ in range(total_blades):
            if card := player.draw_card():
                # Add blade hearts from drawn card
                if card.card_type == CardType.ENERGY and card.blade_heart:
                    for color, count in card.blade_heart.items():
                        blade_hearts[color] = blade_hearts.get(color, 0) + count
        
        # Add blade hearts to live-owned hearts
        for color, count in blade_hearts.items():
            live_owned_hearts[color] = live_owned_hearts.get(color, 0) + count
        
        self.log(f"  {player.name} live-owned hearts: {sum(live_owned_hearts.values())} total")
        
        # Rule 8.3.15: Check if each live card can satisfy required hearts
        remaining_hearts = live_owned_hearts.copy()
        live_cards_to_remove = []
        
        for card in player.live_card_zone.cards:
            if card.card.satisfies_required_hearts(remaining_hearts):
                self.log(f"  {card.card.name} satisfies required hearts")
                remaining_hearts = card.card.consume_required_hearts(remaining_hearts)
            else:
                self.log(f"  {card.card.name} FAILS to satisfy required hearts")
                live_cards_to_remove.append(card.card)
        
        # Rule 8.3.16: If any fails, all live cards go to discard
        if live_cards_to_remove:
            self.log(f"  {player.name} live failed! All live cards go to discard")
            for card in player.live_card_zone.cards:
                player.discard.add_card(card.card)
            player.live_card_zone.clear()
        else:
            self.log(f"  {player.name} live succeeded!")
    
    def _determine_live_victory(self):
        """Rule 8.4: Determine live victory"""
        p1 = self.game_state.player1
        p2 = self.game_state.player2
        
        p1_score = p1.calculate_live_score()
        p2_score = p2.calculate_live_score()
        p1_has_cards = not p1.live_card_zone.is_empty()
        p2_has_cards = not p2.live_card_zone.is_empty()
        
        self.log(f"  Scores - Player 1: {p1_score}, Player 2: {p2_score}")
        self.log(f"  Cards in live zones - Player 1: {p1.live_card_zone.len()}, Player 2: {p2.live_card_zone.len()}")
        
        # Rule 8.4.3: Compare scores
        if not p1_has_cards and not p2_has_cards:
            # Rule 8.4.3.1: Both have no cards - no winner
            self.log(f"  No cards in either live card zone - no winner")
        elif p1_has_cards and not p2_has_cards:
            # Rule 8.4.3.2: Player 1 has cards, Player 2 doesn't
            self.log(f"  Player 1 wins the live!")
            self._move_live_to_success(p1)
        elif p2_has_cards and not p1_has_cards:
            # Rule 8.4.3.2: Player 2 has cards, Player 1 doesn't
            self.log(f"  Player 2 wins the live!")
            self._move_live_to_success(p2)
        else:
            # Both have cards - compare scores
            # Rule 8.4.6.2: Higher score wins, equal = both win
            if p1_score > p2_score:
                self.log(f"  Player 1 wins the live!")
                self._move_live_to_success(p1)
            elif p2_score > p1_score:
                self.log(f"  Player 2 wins the live!")
                self._move_live_to_success(p2)
            else:
                self.log(f"  Scores are equal - both players win the live!")
                # Rule 8.4.7.1: If a player has 2 cards, they don't move a card
                if p1.live_card_zone.len() != 2:
                    self._move_live_to_success(p1)
                if p2.live_card_zone.len() != 2:
                    self._move_live_to_success(p2)
        
        # Rule 8.4.8: Clear remaining cards
        p1_live_cards = p1.live_card_zone.cards[:]
        p2_live_cards = p2.live_card_zone.cards[:]
        p1.live_card_zone.clear()
        p2.live_card_zone.clear()
        for card in p1_live_cards:
            p1.discard.add_card(card.card)
        for card in p2_live_cards:
            p2.discard.add_card(card.card)
        
        # Rule 8.4.13: Determine next first attacker based on who won
        # (Simplified: keep current first attacker for now)
    
    def _move_live_to_success(self, player: Player):
        """Move a live card to success live card zone"""
        if not player.live_card_zone.is_empty():
            card = player.live_card_zone.cards.pop(0)
            player.success_live_card_zone.add_card(card.card)
            self.log(f"  {card.card.name} added to {player.name}'s success live card zone")
    
    def run_turn(self, max_actions: int = 50):
        if not self.game_state:
            print("Game not initialized")
            return
        
        self.log(f"\n--- Turn {self.game_state.turn_number} ---")
        self.log_game_state()
        
        actions_taken = 0
        turn_complete = False
        turn_start_number = self.game_state.turn_number
        
        while actions_taken < max_actions and not turn_complete:
            ai = self.ai1 if self.game_state.active_player() == self.game_state.player1 else self.ai2
            action = ai.choose_action(self.game_state)
            
            self.execute_action(action)
            actions_taken += 1
            
            # Always advance phase after action
            self.advance_phase()
            
            # Check if turn is complete (turn number increased after live victory determination)
            if self.game_state.turn_number > turn_start_number:
                turn_complete = True
        
        print(f"Turn {self.game_state.turn_number - 1} complete")
    
    def run_game(self, max_turns: int = 20):
        self.initialize_game()
        
        for _ in range(max_turns):
            self.run_turn()
            
            # Check for victory condition (need 3 successful lives to win per rule 10.3)
            p1_success = self.game_state.player1.success_live_card_zone.len()
            p2_success = self.game_state.player2.success_live_card_zone.len()
            
            if p1_success >= 3:
                print(f"\nPlayer 1 wins with {p1_success} successful lives!")
                return self.game_state.player1
            elif p2_success >= 3:
                print(f"\nPlayer 2 wins with {p2_success} successful lives!")
                return self.game_state.player2
        
        # Game ended without a winner
        p1_success = self.game_state.player1.success_live_card_zone.len()
        p2_success = self.game_state.player2.success_live_card_zone.len()
        
        if p1_success >= 3:
            print(f"\nPlayer 1 wins with {p1_success} successful lives!")
            return self.game_state.player1
        elif p2_success >= 3:
            print(f"\nPlayer 2 wins with {p2_success} successful lives!")
            return self.game_state.player2
        else:
            print(f"\nGame ended without a winner (Player 1: {p1_success} lives, Player 2: {p2_success} lives)")
            return None

def main():
    # Load card data
    with open('cards.json', 'r', encoding='utf-8') as f:
        cards_data = json.load(f)
    
    # Run simulation
    sim = GameSimulation(cards_data, output_file="game_output.txt")
    winner = sim.run_game(max_turns=10)
    
    if winner:
        print(f"\nWinner: {winner.name}")
    else:
        print("\nNo winner")
    
    # Save logs to file
    sim.save_logs_to_file()

if __name__ == "__main__":
    main()
