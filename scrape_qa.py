import json
import re
import requests
from bs4 import BeautifulSoup


def scrape_qa():
    url = "https://llofficial-cardgame.com/question/searchresults/?keyword=&keyword_type%5B%5D=all&search_type=and&title=&card_kind=&work_title="
    print(f"Fetching {url}...")
    headers = {
        "User-Agent": "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36"
    }
    response = requests.get(url, headers=headers)
    response.encoding = response.apparent_encoding
    soup = BeautifulSoup(response.text, "html.parser")

    qa_list = []

    # Find all QA items
    qa_items = soup.find_all("div", class_="qa-Item")
    print(f"Found {len(qa_items)} QA items on page.")

    for item in qa_items:
        # Heading contains ID and Date
        heading_tag = item.find("h2", class_="faq-Heading")
        if not heading_tag:
            continue

        heading_text = heading_tag.get_text().strip()
        match = re.match(r"(Q\d+)\s*\((.*?)\)", heading_text)
        if not match:
            print(f"Could not match heading: {heading_text}")
            continue

        qa_id = match.group(1)
        date = match.group(2)

        # Question
        q_tag = item.find("p", class_="question-Detail")
        question = process_html_to_text(q_tag) if q_tag else ""

        # Answer
        a_tag = item.find("p", class_="answer-Detail")
        answer = process_html_to_text(a_tag) if a_tag else ""

        # Related cards
        related_cards = []
        relation_div = item.find("div", class_="relation-Detail")
        if relation_div:
            relation_text_tag = relation_div.find("p", class_="relation-Text")
            if relation_text_tag:
                relation_text = relation_text_tag.get_text().strip()
                card_matches = re.findall(r"\[(.*?)\s*：\s*(.*?)\]", relation_text)
                for c_no, c_name in card_matches:
                    related_cards.append(
                        {"card_no": c_no.strip(), "name": c_name.strip()}
                    )

        qa_list.append(
            {
                "id": qa_id,
                "date": date,
                "question": question,
                "answer": answer,
                "related_cards": related_cards,
            }
        )

    return qa_list


def process_html_to_text(element):
    # Replace images with {{filename|alt}}
    # mapping
    mapping = {
        "score.png": "icon_score.png",
        "energy.png": "icon_energy.png",
        "heart_all.png": "icon_all.png",
        "blade.png": "icon_blade.png",
    }

    # We can't easily use BeautifulSoup to replace and keep text flow perfectly
    # but we can iterate through children
    text_parts = []
    for child in element.children:
        if child.name == "img":
            src = child.get("src", "")
            alt = child.get("alt", "")
            filename = src.split("/")[-1]
            final_filename = mapping.get(filename, filename)

            # Special case for energy
            if final_filename == "icon_energy.png":
                alt = "E"

            text_parts.append(f"{{{{{final_filename}|{alt}}}}}")
        elif child.name == "br":
            text_parts.append("\n")
        else:
            text_parts.append(child.get_text())

    return "".join(text_parts).strip()


def main():
    file_path = r"C:\Users\trios\OneDrive\Documents\rabuka_reloaded\cards\qa_data.json"

    with open(file_path, "r", encoding="utf-8") as f:
        data = json.load(f)

    existing_ids = {item["id"] for item in data}

    new_qas = scrape_qa()

    added_count = 0
    for qa in new_qas:
        if qa["id"] not in existing_ids:
            data.append(qa)
            existing_ids.add(qa["id"])
            added_count += 1

    if added_count > 0:
        # Sort by ID descending if necessary, or just keep order
        # The website has them newest first, so we might want to prepend or sort
        # data.sort(key=lambda x: x['id'], reverse=True) # This doesn't work well with 'Q' prefix
        # Let's just sort by the number part of the ID
        def get_id_num(s):
            return int(re.search(r"\d+", s).group())

        data.sort(key=lambda x: get_id_num(x["id"]), reverse=True)

        with open(file_path, "w", encoding="utf-8") as f:
            json.dump(data, f, ensure_ascii=False, indent=2)
        print(f"Added {added_count} new Q&As.")
    else:
        print("No new Q&As found.")


if __name__ == "__main__":
    main()
