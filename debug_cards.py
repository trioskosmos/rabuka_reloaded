#!/usr/bin/env python3

import json

# Load cards database
with open('cards/cards.json', 'r', encoding='utf-8') as f:
    cards = json.load(f)

# Find cards by ID
for card in cards:
    if 'card_no' in card:
        if card['card_no'] == 'PL!SP-sd1-019-SD':
            print(f"PL!SP-sd1-019-SD ID: {card.get('id', 'N/A')}")
        if card['card_no'] == 'PL!SP-sd1-020-SD':
            print(f"PL!SP-sd1-020-SD ID: {card.get('id', 'N/A')}")

# Find card ID 1781
for card in cards:
    if isinstance(card, dict) and card.get('id') == 1781:
        print(f"Card 1781: {card.get('card_no', 'N/A')} - {card.get('name', 'N/A')}")
