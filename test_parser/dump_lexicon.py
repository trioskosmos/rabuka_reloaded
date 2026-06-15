import json
import os

def main():
    path = "cards/cards.json"
    if not os.path.exists(path):
        print("File not found")
        return
        
    with open(path, "r", encoding="utf-8") as f:
        data = json.load(f)
        
    names = set()
    units = set()
    series = set()
    
    for v in data.values():
        if v.get("name"): names.add(v["name"])
        if v.get("unit"): units.add(v["unit"])
        if v.get("series"): series.add(v["series"])
        
    with open("test_parser/card_lexicon.txt", "w", encoding="utf-8") as f:
        f.write("NAMES:\n")
        f.write("\n".join(sorted(names)) + "\n\n")
        f.write("UNITS:\n")
        f.write("\n".join(sorted(units)) + "\n\n")
        f.write("SERIES:\n")
        f.write("\n".join(sorted(series)) + "\n\n")

if __name__ == "__main__":
    main()
