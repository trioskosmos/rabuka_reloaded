"""
Parser utilities for ability extraction.
This module contains pure utility functions for text processing, regex extraction,
and normalization used across the parsing pipeline.
"""
import re

# Precompiled regex patterns for performance
DIGIT_PATTERN = re.compile(r'(\d+)')
COUNT_PATTERN = re.compile(r'(\d+)枚')
PEOPLE_PATTERN = re.compile(r'(\d+)人')
GROUP_PATTERN = re.compile(r"『(.+?)』")
QUOTED_NAME_PATTERN = re.compile(r'「(.+?)」')
COST_PATTERN = re.compile(r'コスト(\d+)')
HEART_PATTERN = re.compile(r'{{heart_(\d+)\.png\|heart\d+}}')
BLADE_PATTERN = re.compile(r'{{icon_blade\.png\|ブレード}}')


def extract_int(pattern, text, default=None):
    """Extract an integer from text using a pattern or regex."""
    if isinstance(pattern, str):
        match = re.search(pattern, text)
    else:
        match = pattern.search(text)
    if match:
        return int(match.group(1))
    return default


def extract_group_name(text):
    """Extract group name from text (e.g., 『虹ヶ咲』 -> 虹ヶ咲)."""
    match = GROUP_PATTERN.search(text)
    if match:
        return match.group(1)
    return None


def extract_quoted_name(text):
    """Extract quoted name from text (e.g., 「上原歩夢」 -> 上原歩夢)."""
    match = QUOTED_NAME_PATTERN.search(text)
    if match:
        return match.group(1)
    return None


def has_any(text, phrases):
    """Check if text contains any of the given phrases."""
    return any(phrase in text for phrase in phrases)


def strip_suffix_period(text):
    """Remove trailing period from text."""
    return text.rstrip('。')


def strip_prefix_period(text):
    """Remove leading period from text."""
    return text.lstrip('。')


def parse_optional_flag(text, phrases):
    """Check if text contains optional phrases and return boolean."""
    return any(phrase in text for phrase in phrases)


def normalize_whitespace(text):
    """Normalize whitespace in text - collapse multiple spaces to single space."""
    return re.sub(r'\s+', ' ', text).strip()


def normalize_fullwidth_digits(text):
    """Normalize full-width digits to half-width (e.g., １ -> 1)."""
    fullwidth = '０１２３４５６７８９'
    halfwidth = '0123456789'
    translation = str.maketrans(fullwidth, halfwidth)
    return text.translate(translation)


def normalize_text(text):
    """Apply all normalization steps to text."""
    text = normalize_whitespace(text)
    text = normalize_fullwidth_digits(text)
    text = strip_suffix_period(text)
    return text


def extract_count(text):
    """Extract count from text (e.g., '3枚' -> 3, '2人' -> 2)."""
    match = COUNT_PATTERN.search(text)
    if match:
        return int(match.group(1))
    match = PEOPLE_PATTERN.search(text)
    if match:
        return int(match.group(1))
    return None


def extract_cost(text):
    """Extract cost value from text (e.g., 'コスト3' -> 3)."""
    match = COST_PATTERN.search(text)
    if match:
        return int(match.group(1))
    return None


def extract_heart_types(text):
    """Extract heart types from text (e.g., heart icons)."""
    matches = HEART_PATTERN.findall(text)
    return matches if matches else None


def extract_blade_count(text):
    """Extract blade count from text (number of blade icons)."""
    matches = BLADE_PATTERN.findall(text)
    return len(matches) if matches else 0


def create_fallback(raw_text):
    """Create a fallback result with raw_text."""
    return {'raw_text': raw_text}


def is_fallback(result):
    """Check if a result is a fallback (contains raw_text)."""
    return isinstance(result, dict) and 'raw_text' in result


def merge_position_requirement(result, action):
    """Merge position_requirement from action into result if present."""
    if 'position_requirement' in action:
        result['position_requirement'] = action['position_requirement']
        del action['position_requirement']
    return result


def split_commas_smartly(text):
    """Split text by commas, but preserve structural commas."""
    parts = []
    current = ""
    i = 0
    while i < len(text):
        if text[i] == '、':
            if i >= 1:
                prev_char = text[i-1]
                if prev_char == 'は':
                    current += '、'
                    i += 1
                    continue
                if i >= 7 and text[i-7:i] == 'ライブ終了時まで':
                    current += '、'
                    i += 1
                    continue
                if i >= 2 and text[i-2:i] == '場合':
                    current += '、'
                    i += 1
                    continue
            if i >= 3 and text[i-3:i] == 'その後':
                parts.append(current)
                current = ""
                i += 1
                continue
            parts.append(current)
            current = ""
            i += 1
        else:
            current += text[i]
            i += 1
    if current:
        parts.append(current)
    return parts


# Main groups (large idol groups)
MAIN_GROUPS = {
    'μ\'s', 'Aqours', 'Saint Snow', '虹ヶ咲', 'Liella!',
    'Nijigaku', 'Liella', 'SaintSnow', 'Muse',
    '蓮ノ空',  # Hasunosora
}

# Subunits (smaller groups within main groups)
SUBUNITS = {
    'CYaRon!', 'AZALEA', 'Guilty Kiss', 'Dance',
    'Qu4rtz', 'R3BIRTH',
    'CatChu!', '5yncri5e!', 'BiBi', 'Printemps',
    'lily white', 'DOLLCHESTRA', 'スリーズブーケ',
    'みらくらぱーく！', 'MIRAPARK', 'EdelNote',
    'A-RISE', 'SunnyPassion', 'KALEIDOSCORE',
}

# Combined known units (both main groups and subunits)
KNOWN_UNITS = MAIN_GROUPS | SUBUNITS


def detect_group_type(group_name):
    """Detect whether a group name is a unit or character.
    Returns 'unit' if it's a known unit name, 'character' otherwise."""
    # Normalize the group name for comparison
    normalized = group_name.strip()
    
    # Check against known units
    if normalized in KNOWN_UNITS:
        return 'unit'
    
    # Check for common unit patterns
    # Units often have special characters like !, ', or are in English
    if '!' in normalized or "'" in normalized:
        return 'unit'
    
    # Japanese katakana/hiragana names are typically characters
    # Units are usually in English or have special formatting
    # This is a heuristic - may need refinement
    if any(c in normalized for c in 'アイウエオカキクケコサシスセソタチツテトナニヌネノハヒフヘホマミムメモヤユヨラリルレロワヲン'):
        # If it's mostly katakana and not a known unit, it's likely a character
        return 'character'
    
    # Default to character if unknown
    return 'character'


def extract_all_groups(text):
    """Extract all group names from text (『...』 patterns)."""
    matches = GROUP_PATTERN.findall(text)
    return matches if matches else []


def extract_all_quoted_names(text):
    """Extract all quoted names from text (「...」 patterns)."""
    matches = QUOTED_NAME_PATTERN.findall(text)
    return matches if matches else []


def annotate_tree(value, text):
    """Attach source text to every parsed dict in a tree."""
    if not text or value is None:
        return value
    if isinstance(value, dict):
        value.setdefault('text', text)
        for item in value.values():
            annotate_tree(item, text)
    elif isinstance(value, list):
        for item in value:
            annotate_tree(item, text)
    return value
