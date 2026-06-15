import json
import os

def main():
    path = "test_parser/real_abilities.json"
    if not os.path.exists(path):
        print("File not found")
        return
        
    with open(path, "r", encoding="utf-8") as f:
        data = json.load(f)
        
    abilities = data.get("abilities", [])
    total = len(abilities)
    unknown = 0
    partial = 0
    full = 0
    
    for ab in abilities:
        parsed = ab.get("parsed", {})
        effects = parsed.get("effects", [])
        
        has_unknown = any(e.get("type") == "unknown" for e in effects)
        has_known = any(e.get("type") != "unknown" for e in effects)
        
        if has_unknown and not has_known:
            unknown += 1
        elif has_unknown and has_known:
            partial += 1
        else:
            full += 1
            
    print(f"Total Unique Abilities: {total}")
    print(f"Fully Parsed:         {full} ({full/total:.1%})")
    print(f"Partially Parsed:     {partial} ({partial/total:.1%})")
    print(f"Unknown:              {unknown} ({unknown/total:.1%})")

if __name__ == "__main__":
    main()
