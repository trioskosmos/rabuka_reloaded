import json

sc = json.load(open('../cards/scenarios.json', encoding='utf-8'))
s = sc[396]
print("Scenario 396:")
print("  card_no:", s["card_no"])
print("  triggers:", repr(s["triggers"]))
print("  setup:", s["setup"])
print("  action:", s["action"])
print("  text:", repr(s["text"][:100]))
