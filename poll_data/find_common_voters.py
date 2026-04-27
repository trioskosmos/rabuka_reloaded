import csv
import sys
import io
from collections import defaultdict

# Set UTF-8 encoding for output
sys.stdout = io.TextIOWrapper(sys.stdout.buffer, encoding='utf-8')

file_path = r"C:\Users\trios\OneDrive\Documents\rabuka_reloaded\poll_data\ll_demographics.csv"

# Read the CSV to get column names
with open(file_path, 'r', encoding='utf-8-sig', errors='replace') as f:
    reader = csv.DictReader(f)
    columns = reader.fieldnames
    columns = [col for col in columns if 'Timestamp' not in col]

print("Available columns:")
for i, col in enumerate(columns, 1):
    print(f"  {i}. {col}")

print("\n" + "="*80)
print("Enter the criteria to find common voters")
print("Format: column_number:value (e.g., '7:Kotori Minami')")
print("Enter multiple criteria separated by commas to find voters matching ALL criteria")
print("Type 'done' when finished")
print("="*80 + "\n")

criteria = []
while True:
    user_input = input("Enter criteria (or 'done'): ").strip()
    if user_input.lower() == 'done':
        break
    if user_input:
        criteria.append(user_input)

if not criteria:
    print("No criteria entered. Exiting.")
    sys.exit(0)

# Parse criteria
parsed_criteria = []
for crit in criteria:
    try:
        col_num, value = crit.split(':', 1)
        col_num = int(col_num) - 1  # Convert to 0-indexed
        if 0 <= col_num < len(columns):
            parsed_criteria.append((columns[col_num], value.strip()))
        else:
            print(f"Invalid column number: {col_num + 1}")
    except ValueError:
        print(f"Invalid format: {crit}. Use 'column_number:value'")

if not parsed_criteria:
    print("No valid criteria. Exiting.")
    sys.exit(0)

print("\n" + "="*80)
print("Searching for voters matching ALL of the following criteria:")
for col, val in parsed_criteria:
    print(f"  - {col}: {val}")
print("="*80 + "\n")

# Count matching rows
matching_count = 0
total_rows = 0

with open(file_path, 'r', encoding='utf-8-sig', errors='replace') as f:
    reader = csv.DictReader(f)
    for row in reader:
        total_rows += 1
        match = True
        for col, val in parsed_criteria:
            if row.get(col, '').strip() != val:
                match = False
                break
        if match:
            matching_count += 1

print(f"\nResults:")
print(f"  Total respondents: {total_rows}")
print(f"  Matching ALL criteria: {matching_count}")
print(f"  Percentage: {(matching_count / total_rows * 100):.2f}%")
