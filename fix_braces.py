import re

with open('engine/src/turn/triggers.rs', 'r') as f:
    lines = f.readlines()

# Find and fix the extra closing braces for the first collapsible_if fix
# Original had: } } (two closing braces for the two nested ifs)
# Now we need only one

# The pattern is: after the abilities_to_trigger.push block, there are two }
# We need to remove one of them

# First fix (around line 273-274 in original)
# Look for:                     })
#                          }
# Remove one }

i = 0
while i < len(lines) - 1:
    # First collapsible_if extra brace removal
    if (lines[i].rstrip() == '                            }' and 
        lines[i+1].rstrip() == '                        }' and
        i > 260 and i < 280):
        # Check if this is the right place - after abilities_to_trigger.push
        if 'abilities_to_trigger.push' in ''.join(lines[max(0,i-10):i]):
            # Remove the second brace (i+1)
            lines.pop(i+1)
            print(f"Removed extra brace at line {i+2}")
            break
    i += 1

# Second collapsible_if extra brace removal (around line 328-329)
i = 0
while i < len(lines) - 1:
    if (lines[i].rstrip() == '                                }' and 
        lines[i+1].rstrip() == '                            }' and
        i > 310 and i < 340):
        if 'abilities_to_trigger.push' in ''.join(lines[max(0,i-10):i]):
            lines.pop(i+1)
            print(f"Removed extra brace at line {i+2}")
            break
    i += 1

with open('engine/src/turn/triggers.rs', 'w') as f:
    f.writelines(lines)

print("Done fixing braces")