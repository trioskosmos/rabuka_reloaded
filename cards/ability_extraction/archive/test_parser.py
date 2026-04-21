"""Test the general parser on specific cards to verify correctness."""
import json
from general_parser import GeneralAbilityParser


def test_specific_cards():
    """Test parser on specific cards from MANUAL_GRAMMAR_ANALYSIS.md."""
    parser = GeneralAbilityParser()
    
    # Test abilities from the manual analysis
    test_abilities = [
        # Ability 1 - Nested conditionals
        "自分がエールしたとき、エールにより公開された自分のカードが持つブレードハートの中に{{heart_01.png|heart01}}、{{heart_02.png|heart02}}、{{heart_03.png|heart03}}、{{heart_04.png|heart04}}、{{heart_05.png|heart05}}、{{heart_06.png|heart06}}、{{icon_all.png|ハート}}のうち、3種類以上ある場合、ライブ終了時まで、{{heart_01.png|heart01}}を得る。6種類以上ある場合、さらにライブ終了時まで、「{{jyouji.png|常時}}ライブの合計スコアを＋１する。」を得る。",
        
        # Ability 2 - Cost modification
        "自分のステージに『蓮ノ空』のメンバーがいる場合、このカードを成功させるための必要ハートは、{{heart_01.png|heart01}}{{heart_01.png|heart01}}{{heart_00.png|heart0}}か、{{heart_04.png|heart04}}{{heart_04.png|heart04}}{{heart_00.png|heart0}}か、{{heart_05.png|heart05}}{{heart_05.png|heart05}}{{heart_00.png|heart0}}のうち、選んだ1つにしてもよい。",
        
        # Ability 4 - OR conditionals
        "エールにより公開された自分のカードの中にライブカードが2枚以上あるか、自分のステージにいるメンバーが持つハートの中に{{heart_01.png|heart01}}、{{heart_04.png|heart04}}、{{heart_05.png|heart05}}、{{heart_02.png|heart02}}、{{heart_03.png|heart03}}、{{heart_06.png|heart06}}のうち合計5種類以上あるか、このターンに自分のステージにいるメンバーがエリアを移動している場合、このカードのスコアを＋１する。",
        
        # Ability 5 - Per-unit modifiers
        "自分のステージにいる『虹ヶ咲』のメンバーが持つ{{heart_01.png|heart01}}、{{heart_04.png|heart04}}、{{heart_05.png|heart05}}、{{heart_02.png|heart02}}、{{heart_03.png|heart03}}、{{heart_06.png|heart06}}のうち1色につき、このカードのスコアを＋１する。",
        
        # Ability 8 - Sequential actions
        "{{icon_energy.png|E}}{{icon_energy.png|E}}このメンバーをステージから控え室に置く：自分の手札からコスト13以下の「優木せつ菜」のメンバーカードを1枚、このメンバーがいたエリアに登場させる。その後、自分のエネルギー置き場にあるエネルギー1枚をそのメンバーの下に置く。",
        
        # Ability 11 - Optional action with conditional follow-up
        "自分のエネルギー置き場にあるエネルギー1枚をこのメンバーの下に置いてもよい。そうした場合、カードを1枚引き、ライブ終了時まで、自分のステージにいるメンバーは{{icon_blade.png|ブレード}}{{icon_blade.png|ブレード}}を得る。",
        
        # Ability 14 - Choice effects
        "以下から1つを選ぶ。\n・自分のステージにいるこのメンバー以外の『Aqours』のメンバー1人は、ライブ終了時まで、{{icon_blade.png|ブレード}}を得る。\n・自分のステージにいる『SaintSnow』のメンバー1人をポジションチェンジさせる。",
        
        # Ability 18 - "その中から" pattern
        "手札を1枚控え室に置いてもよい：自分のデッキの上からカードを3枚見る。その中から1枚を手札に加え、残りを控え室に置く。",
        
        # Ability 19 - Sequential effects and state change
        "自分のステージに{{heart_02.png|heart02}}を4つ以上持つメンバーがいる場合、このカードのスコアを＋２し、必要ハートは{{heart_02.png|heart02}}{{heart_02.png|heart02}}{{heart_02.png|heart02}}{{heart_02.png|heart02}}{{heart_02.png|heart02}}になる。",
        
        # Ability 20 - Choice that affects effect
        "手札の『蓮ノ空』のカードを2枚控え室に置いてもよい：{{heart_01.png|heart01}}か{{heart_04.png|heart04}}か{{heart_05.png|heart05}}か{{heart_06.png|heart06}}のうち、1つを選ぶ。ライブ終了時まで、自分のステージにいるこのメンバー以外の『蓮ノ空』のメンバー1人は、選んだハートを2つ得る。",
    ]
    
    print("=" * 80)
    print("TESTING PARSER ON MANUAL GRAMMAR ANALYSIS EXAMPLES")
    print("=" * 80)
    
    for i, ability in enumerate(test_abilities, 1):
        print(f"\n{'=' * 80}")
        print(f"ABILITY {i}")
        print(f"{'=' * 80}")
        print(f"Raw: {ability[:100]}...")
        print()
        
        result = parser.parse_ability(ability)
        
        print(f"Parsed: {result['parsed']}")
        if result['parsed']:
            print(f"Timing: {result['timing']}")
            print(f"Conditions: {len(result['conditions'])} found")
            for cond in result['conditions']:
                print(f"  - {cond['text'][:60]}... (marker: {cond['marker']})")
            print(f"Costs: {result['costs']}")
            print(f"Effects: {len(result['effects'])} found")
            for effect in result['effects']:
                print(f"  - {effect}")
            print(f"Targets: {result['targets']}")
            print(f"Resources: {result['resources']}")
            print(f"Locations: {result['locations']}")
            print(f"Groups: {result['groups']}")
            print(f"Patterns detected: {result['patterns']}")
        else:
            print(f"Error: {result.get('error', 'Unknown')}")


def check_unparsed_cards():
    """Check some of the unparsed cards to understand why they failed."""
    with open('parsed_abilities_general.json', 'r', encoding='utf-8') as f:
        data = json.load(f)
    
    print("\n" + "=" * 80)
    print("CHECKING UNPARSED CARDS")
    print("=" * 80)
    
    unparsed = []
    for card_id, card_data in data['results'].items():
        if not card_data['parsed_ability']['parsed']:
            unparsed.append({
                'id': card_id,
                'name': card_data['card_name'],
                'raw': card_data['parsed_ability']['raw'],
                'error': card_data['parsed_ability'].get('error', 'Unknown')
            })
    
    print(f"\nTotal unparsed: {len(unparsed)}")
    print(f"\nFirst 10 unparsed cards:")
    
    for i, card in enumerate(unparsed[:10], 1):
        print(f"\n{i}. {card['id']} - {card['name']}")
        print(f"   Error: {card['error']}")
        print(f"   Raw: {card['raw'][:150]}...")


if __name__ == '__main__':
    test_specific_cards()
    check_unparsed_cards()
