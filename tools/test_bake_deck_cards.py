import struct
import unittest
from concurrent.futures import ThreadPoolExecutor

from tools import bake_deck_cards


FIELDS = ("card_no", "name", "series", "group", "unit", "img", "product", "rare", "ability")


class DeckBlobTests(unittest.TestCase):
    def decode_blob(self, blob):
        magic, count, strtab_size = struct.unpack_from("<4sHI", blob)
        self.assertEqual(magic, b"CARD")
        end = 10 + strtab_size
        pos = 10
        strings = []
        while pos < end:
            length = struct.unpack_from("<H", blob, pos)[0]
            pos += 2
            self.assertLessEqual(pos + length, end)
            strings.append(blob[pos:pos + length].decode("utf-8"))
            pos += length
        self.assertEqual(pos, end)
        self.assertEqual(strings[0], "")
        lengths = struct.unpack_from(f"<{count}B", blob, end)
        pos = end + count
        cards = []
        for length in lengths:
            self.assertGreaterEqual(length, 25)
            self.assertLessEqual(pos + length, len(blob))
            refs = struct.unpack_from("<9H", blob, pos)
            card = {}
            for field, index in zip(FIELDS, refs):
                if index == 0xFFFF:
                    self.assertIn(field, ("unit", "ability"))
                    card[field] = None
                else:
                    self.assertLess(index, len(strings), field)
                    card[field] = strings[index]
            cards.append(card)
            pos += length
        self.assertEqual(pos, len(blob))
        return strings, cards

    def test_all_string_fields_are_serialized(self):
        card = {field: f"{field}-\u3042" for field in FIELDS}
        card["card_no"] = "TEST-A"
        card["base_heart"] = {"heart00": 2}
        card["blade_heart"] = {"b_all": 1}
        card["need_heart"] = {"heart01": 3}
        card["special_heart"] = {"score": 1}
        blob = bake_deck_cards.make_deck_blob({"TEST-A": card}, ["test-a"])
        strings, decoded = self.decode_blob(blob)
        self.assertEqual(decoded, [{field: card[field] for field in FIELDS}])
        self.assertEqual(set(strings), {"", *(card[field] for field in FIELDS)})

    def test_deduplication_missing_cards_and_optional_strings(self):
        cards = {"TEST-A": {"card_no": "TEST-A"}, "TEST-B": {"card_no": "TEST-B"}}
        blob = bake_deck_cards.make_deck_blob(cards, ["test-b", "TEST-B", "missing", "test-a"])
        strings, decoded = self.decode_blob(blob)
        self.assertEqual(strings, ["", "TEST-B", "TEST-A"])
        self.assertEqual([card["card_no"] for card in decoded], ["TEST-B", "TEST-A"])
        for card in decoded:
            self.assertIsNone(card["unit"])
            self.assertIsNone(card["ability"])
            self.assertEqual(card["group"], "")
        self.assertEqual(self.decode_blob(bake_deck_cards.make_deck_blob(cards, [])), ([""], []))

    def test_parallel_decks_are_isolated_and_deterministic(self):
        cards = {
            f"TEST-{number}": {field: f"{field}-{number}" for field in FIELDS}
            for number in range(8)
        }
        for card_no, card in cards.items():
            card["card_no"] = card_no
        expected = {
            card_no: bake_deck_cards.make_deck_blob(cards, [card_no])
            for card_no in cards
        }
        for order in (list(cards), list(reversed(cards)), list(cards)[::2] + list(cards)[1::2]):
            with ThreadPoolExecutor(max_workers=4) as executor:
                futures = [(card_no, executor.submit(bake_deck_cards.make_deck_blob, cards, [card_no]))
                           for card_no in order * 3]
                for card_no, future in futures:
                    blob = future.result()
                    self.assertEqual(blob, expected[card_no])
                    strings, decoded = self.decode_blob(blob)
                    self.assertEqual(decoded, [cards[card_no]])
                    self.assertEqual(set(strings), {"", *cards[card_no].values()})


if __name__ == "__main__":
    unittest.main()
