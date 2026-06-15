import json

def recover(s):
    try:
        # Try CP1252/Latin-1 to UTF-8
        return s.encode('latin-1').decode('utf-8')
    except:
        try:
            # Try another common mangling: CP932 misread as Latin-1
            return s.encode('latin-1').decode('cp932')
        except:
            return s

# Test with the mangled string from the output
mangled = "{{kidou.png|N}}̃o[Xe[WTɒuF̍T烁o[J[h1DɉB"
print(f"Mangled: {mangled}")
print(f"Recovered: {recover(mangled)}")

with open('cards/abilities.json', 'rb') as f:
    raw = f.read(1000)
    print(f"\nRaw bytes (first 1000): {raw[:100]}")
    try:
        print(f"Decoded as UTF-8: {raw.decode('utf-8')[:100]}")
    except Exception as e:
        print(f"UTF-8 failed: {e}")
    try:
        print(f"Decoded as CP932: {raw.decode('cp932')[:100]}")
    except Exception as e:
        print(f"CP932 failed: {e}")
