import csv
import sys
import io
from collections import defaultdict
from itertools import combinations

# Set UTF-8 encoding for output
sys.stdout = io.TextIOWrapper(sys.stdout.buffer, encoding='utf-8')

file_path = r"C:\Users\trios\OneDrive\Documents\rabuka_reloaded\poll_data\ll_demographics.csv"

# Read the CSV to get column names and data
with open(file_path, 'r', encoding='utf-8-sig', errors='replace') as f:
    reader = csv.DictReader(f)
    columns = [col for col in reader.fieldnames if 'Timestamp' not in col]
    rows = list(reader)

print(f"Total respondents: {len(rows)}")
print(f"Total columns: {len(columns)}\n")

# Function to find common voters between two specific values in different columns
def find_common_voters(col1, val1, col2, val2):
    count = 0
    for row in rows:
        if row.get(col1, '').strip() == val1 and row.get(col2, '').strip() == val2:
            count += 1
    return count

# Analyze top combinations
print("="*80)
print("TOP 20 COMMON VOTER COMBINATIONS ACROSS DIFFERENT CATEGORIES")
print("="*80 + "\n")

# Get top values for each column (top 10 by frequency)
column_top_values = {}
for col in columns:
    value_counts = defaultdict(int)
    for row in rows:
        val = row.get(col, '').strip()
        if val:
            value_counts[val] += 1
    column_top_values[col] = sorted(value_counts.items(), key=lambda x: -x[1])[:10]

# Analyze combinations between different columns
combinations_data = []

# Limit to first 15 columns to avoid too many combinations
cols_to_analyze = columns[:15]

for col1, col2 in combinations(cols_to_analyze, 2):
    for val1, count1 in column_top_values[col1]:
        for val2, count2 in column_top_values[col2]:
            common = find_common_voters(col1, val1, col2, val2)
            if common > 0:
                combinations_data.append({
                    'col1': col1,
                    'val1': val1,
                    'col2': col2,
                    'val2': val2,
                    'common': common,
                    'count1': count1,
                    'count2': count2
                })

# Sort by common count descending
combinations_data.sort(key=lambda x: -x['common'])

# Print top 20
print(f"{'#':<3} {'Count':<6} {'% of Total':<10} {'Column 1':<30} {'Value 1':<25} {'Column 2':<30} {'Value 2':<25}")
print("-" * 140)

for i, combo in enumerate(combinations_data[:20], 1):
    percentage = (combo['common'] / len(rows)) * 100
    col1_short = combo['col1'][:28] + '..' if len(combo['col1']) > 28 else combo['col1']
    val1_short = combo['val1'][:23] + '..' if len(combo['val1']) > 23 else combo['val1']
    col2_short = combo['col2'][:28] + '..' if len(combo['col2']) > 28 else combo['col2']
    val2_short = combo['val2'][:23] + '..' if len(combo['val2']) > 23 else combo['val2']
    
    print(f"{i:<3} {combo['common']:<6} {percentage:<9.2f}% {col1_short:<30} {val1_short:<25} {col2_short:<30} {val2_short:<25}")

print("\n" + "="*80)
print("SPECIFIC CATEGORY PAIR ANALYSIS")
print("="*80 + "\n")

# Let user analyze specific column pairs
print("Available columns (first 20):")
for i, col in enumerate(columns[:20], 1):
    print(f"  {i}. {col}")

print("\nEnter two column numbers to analyze (e.g., '1 2'), or 'done' to exit")

while True:
    user_input = input("\nEnter column pair: ").strip()
    if user_input.lower() == 'done':
        break
    
    try:
        parts = user_input.split()
        if len(parts) != 2:
            print("Please enter exactly two column numbers")
            continue
        
        col1_idx = int(parts[0]) - 1
        col2_idx = int(parts[1]) - 1
        
        if not (0 <= col1_idx < len(columns) and 0 <= col2_idx < len(columns)):
            print("Invalid column numbers")
            continue
        
        col1 = columns[col1_idx]
        col2 = columns[col2_idx]
        
        print(f"\nAnalyzing: {col1} vs {col2}")
        print("-" * 80)
        
        # Get all value combinations for these two columns
        combo_counts = defaultdict(int)
        for row in rows:
            val1 = row.get(col1, '').strip()
            val2 = row.get(col2, '').strip()
            if val1 and val2:
                combo_counts[(val1, val2)] += 1
        
        # Sort and display
        sorted_combos = sorted(combo_counts.items(), key=lambda x: -x[1])
        print(f"{'Count':<6} {'%':<6} {col1[:40]:<40} {col2[:40]:<40}")
        print("-" * 100)
        
        for (val1, val2), count in sorted_combos[:20]:
            percentage = (count / len(rows)) * 100
            val1_short = val1[:38] + '..' if len(val1) > 38 else val1
            val2_short = val2[:38] + '..' if len(val2) > 38 else val2
            print(f"{count:<6} {percentage:<5.1f}% {val1_short:<40} {val2_short:<40}")
        
    except ValueError:
        print("Invalid input. Please enter two numbers separated by space")
