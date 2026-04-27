import csv
import sys
import io

# Set UTF-8 encoding for output
sys.stdout = io.TextIOWrapper(sys.stdout.buffer, encoding='utf-8')

file_path = r"C:\Users\trios\OneDrive\Documents\rabuka_reloaded\poll_data\ll_demographics.csv"

# Read the CSV and extract unique values with counts for each column
with open(file_path, 'r', encoding='utf-8-sig', errors='replace') as f:
    reader = csv.DictReader(f)
    columns = reader.fieldnames
    
    # Exclude timestamp column
    columns = [col for col in columns if 'Timestamp' not in col]
    
    # Initialize dictionary to store value counts for each column
    value_counts = {col: {} for col in columns}
    
    # Collect all values with counts
    for row in reader:
        for col in columns:
            value = row.get(col, '').strip()
            if value:
                value_counts[col][value] = value_counts[col].get(value, 0) + 1

# Print results
print(f"Total columns: {len(columns)}")
with open(file_path, 'r', encoding='utf-8-sig', errors='replace') as f:
    total_rows = sum(1 for _ in f) - 1
print(f"Total rows: {total_rows}\n")

for col in columns:
    # Sort by count descending, then alphabetically
    sorted_items = sorted(value_counts[col].items(), key=lambda x: (-x[1], x[0]))
    total_responses = sum(value_counts[col].values())
    print(f"\n{'='*80}")
    print(f"COLUMN: {col}")
    print(f"{'='*80}")
    print(f"Total unique values: {len(sorted_items)}")
    print(f"Total responses: {total_responses}\n")
    for val, count in sorted_items:
        percentage = (count / total_responses) * 100 if total_responses > 0 else 0
        print(f"  - {val}: {count} ({percentage:.1f}%)")
