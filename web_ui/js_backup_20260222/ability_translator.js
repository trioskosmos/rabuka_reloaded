/**
 * Lovecasim Ability Translator
 * Converts raw opcode-style ability text into human-readable Japanese and English.
 */

import { TriggerType, Opcodes as EffectType, ConditionTypes as ConditionCheck } from './generated_constants.js';

// Add missing aliases if any
TriggerType.ON_ACTIVATE = TriggerType.ACTIVATED;
EffectType.FLAVOR_ACTION = EffectType.FLAVOR; // Alias for compatibility

const COMMON_NAMES = {
    "高坂 穂乃果": "Honoka Kosaka", "絢瀬 絵里": "Eli Ayase", "南 ことり": "Kotori Minami", "園田 海未": "Umi Sonoda", "星空 凛": "Rin Hoshizora", "西木野 真姫": "Maki Nishikino", "東條 希": "Nozomi Tojo", "小泉 花陽": "Hanayo Koizumi", "矢澤 にこ": "Nico Yazawa",
    "高海 千歌": "Chika Takami", "桜内 梨子": "Riko Sakurauchi", "松浦 果南": "Kanan Matsuura", "黒澤 ダイヤ": "Dia Kurosawa", "渡辺 曜": "You Watanabe", "津島 善子": "Yoshiko Tsushima", "国木田 花丸": "Hanamaru Kunikida", "小原 鞠莉": "Mari Ohara", "黒澤 ルビィ": "Ruby Kurosawa",
    "上原 歩夢": "Ayumu Uehara", "中須 かすみ": "Kasumi Nakasu", "桜坂 しずく": "Shizuku Osaka", "朝香 果林": "Karin Asaka", "宮下 愛": "Ai Miyashita", "近江 彼方": "Kanata Konoe", "優木 せつ菜": "Setsuna Yuki", "エマ・ヴェルデ": "Emma Verde", "天王寺 璃奈": "Rina Tennoji", "三船 栞子": "Shioriko Mifune", "ミア・テイラー": "Mia Taylor", "鐘 嵐珠": "Lanzhu Zhong",
    "澁谷 かのん": "Kanon Shibuya", "唐 可可": "Keke Tang", "嵐 千砂都": "Chisato Arashi", "平安名 すみれ": "Sumire Heanna", "葉月 恋": "Ren Hazuki", "桜小路 きな子": "Kinako Sakurakoji", "米女 メイ": "Mei Yoneme", "若菜 四季": "Shiki Wakana", "鬼塚 夏美": "Natsumi Onitsuka", "ウィーン・マルガレーテ": "Wien Margarete", "鬼塚 冬毬": "Tomari Onitsuka",
    "日野下 花帆": "Kaho Hinoshita", "乙宗 梢": "Kozue Otomune", "村野 さやか": "Sayaka Murano", "夕霧 綴理": "Tsuzuri Yugiri", "大沢 瑠璃乃": "Rurino Osawa", "藤島 慈": "Megumi Fujishima", "徒町 小鈴": "Kosuzu Kachimachi", "百生 吟子": "Ginko Momose", "安養寺 姫芽": "Hime Anyoji"
};

const NAME_MAP = {};
for (const [jp, en] of Object.entries(COMMON_NAMES)) {
    NAME_MAP[jp] = en;
    NAME_MAP[jp.replace(/\s/g, '')] = en;
}

const Translations = {
    jp: {
        triggers: { [TriggerType.ON_PLAY]: "【登場時】", [TriggerType.ON_LIVE_START]: "【ライブ開始時】", [TriggerType.ON_LIVE_SUCCESS]: "【ライブ成功時】", [TriggerType.TURN_START]: "【ターン開始時】", [TriggerType.TURN_END]: "【ターン終了時】", [TriggerType.CONSTANT]: "【常時】", [TriggerType.ACTIVATED]: "【起動】", [TriggerType.ON_LEAVES]: "【離脱時】", [TriggerType.ON_REVEAL]: "【公開時】", [TriggerType.ON_POSITION_CHANGE]: "【配置変更時】" },
        opcodes: {
            [EffectType.ENERGY]: "【エネ】{value}獲得",
            [EffectType.DRAW]: "{value}枚ドロー", [EffectType.ADD_BLADES]: "【ブレード】+{value}", [EffectType.ADD_HEARTS]: "【ハート】+{value}", [EffectType.REDUCE_COST]: "コスト-{value}", [EffectType.LOOK_DECK]: "デッキを{value}枚見る", [EffectType.RECOVER_LIVE]: "控えライブ{value}枚回収", [EffectType.BOOST_SCORE]: "スコア+{value}", [EffectType.RECOVER_MEMBER]: "控えメンバー{value}枚回収", [EffectType.BUFF_POWER]: "パワー+{value}", [EffectType.IMMUNITY]: "効果無効", [EffectType.MOVE_MEMBER]: "メンバー移動", [EffectType.SWAP_CARDS]: "手札交換", [EffectType.SEARCH_DECK]: "デッキ検索", [EffectType.ENERGY_CHARGE]: "エネチャージ+{value}", [EffectType.ACTIVATE_MEMBER]: "アクティブ化({value})", [EffectType.ADD_TO_HAND]: "手札+{value}", [EffectType.MOVE_TO_DECK]: "デッキ戻し({value})", [EffectType.REVEAL_CARDS]: "{value}枚公開", [EffectType.LOOK_AND_CHOOSE]: "見て選ぶ({value})", [EffectType.TAP_OPPONENT]: "相手ウェイト({value})", [EffectType.TAP_MEMBER]: "自ウェイト({value})", [EffectType.PLAY_MEMBER_FROM_HAND]: "手札から登場", [EffectType.SET_BLADES]: "【ブレード】を{value}にセット", [EffectType.SET_HEARTS]: "【ハート】を{value}にセット", [EffectType.FORMATION_CHANGE]: "配置変更", [EffectType.NEGATE_EFFECT]: "打ち消し", [EffectType.ORDER_DECK]: "デッキ並べ替え({value}枚)", [EffectType.BATON_TOUCH_MOD]: "バトン条件変更", [EffectType.SET_SCORE]: "ライブスコアを{value}にセット", [EffectType.SWAP_ZONE]: "移動", [EffectType.TRANSFORM_COLOR]: "色変換", [EffectType.TRIGGER_REMOTE]: "リモート誘発", [EffectType.REDUCE_HEART_REQ]: "ハート条件変更", [EffectType.CHEER_REVEAL]: "応援公開", [EffectType.COLOR_SELECT]: "色選択",
            [EffectType.REPLACE_EFFECT]: "効果置換", [EffectType.MODIFY_SCORE_RULE]: "スコアルール変更", [EffectType.MOVE_TO_DISCARD]: "控え室に{value}枚置く", [EffectType.GRANT_ABILITY]: "能力付与", [EffectType.INCREASE_HEART_COST]: "コスト+{value}", [EffectType.REDUCE_YELL_COUNT]: "エネ要件低下", [EffectType.PLAY_MEMBER_FROM_DISCARD]: "控えから登場",
            [EffectType.PAY_ENERGY]: "【エネ】{value}支払い",
            [EffectType.SELECT_MEMBER]: "メンバー選択({value})", [EffectType.DRAW_UNTIL]: "{value}枚までドロー", [EffectType.SELECT_PLAYER]: "プレイヤー選択", [EffectType.SELECT_LIVE]: "ライブ選択", [EffectType.REVEAL_UNTIL]: "{value}が出るまで公開", [EffectType.INCREASE_COST]: "コスト+{value}", [EffectType.PREVENT_PLAY_TO_SLOT]: "登場制限", [EffectType.SWAP_AREA]: "エリア移動", [EffectType.TRANSFORM_HEART]: "ハート変換", [EffectType.SELECT_CARDS]: "カード選択", [EffectType.OPPONENT_CHOOSE]: "相手が選ぶ", [EffectType.PLAY_LIVE_FROM_DISCARD]: "控えライブ登場", [EffectType.REDUCE_LIVE_SET_LIMIT]: "リミット低下",
            [EffectType.PREVENT_SET_TO_SUCCESS_PILE]: "成功エリア配置制限", [EffectType.PREVENT_BATON_TOUCH]: "バトンタッチ制限",
            [EffectType.PREVENT_ACTIVATE]: "起動制限", [EffectType.ACTIVATE_ENERGY]: "エネアクティブ化({value})", [EffectType.ADD_STAGE_ENERGY]: "メンバーにエネ追加", [EffectType.FLAVOR]: "フレーバーアクション", [EffectType.META_RULE]: "[ルール変更]", [EffectType.PLACE_UNDER]: "下に置く", [EffectType.RESTRICTION]: "制限", [EffectType.SET_HEART_COST]: "ハートコスト変更({value})",
            [EffectType.SET_TARGET_SELF]: "対象:自分", [EffectType.SET_TARGET_PLAYER]: "対象:PL", [EffectType.SET_TARGET_OPPONENT]: "対象:相手", [EffectType.SET_TARGET_ALL_PLAYERS]: "対象:双方", [EffectType.SET_TARGET_MEMBER_SELF]: "対象:自メンバー", [EffectType.SET_TARGET_MEMBER_OTHER]: "対象:他メンバー", [EffectType.SET_TARGET_CARD_HAND]: "対象:手札", [EffectType.SET_TARGET_CARD_DISCARD]: "対象:控え", [EffectType.SET_TARGET_CARD_DECK_TOP]: "対象:デッキ上", [EffectType.SET_TARGET_OPPONENT_HAND]: "対象:相手手札", [EffectType.SET_TARGET_MEMBER_SELECT]: "対象選択", [EffectType.SET_TARGET_MEMBER_NAMED]: "名前指定",
            [EffectType.ADD_CONTINUOUS]: "能力継続", [EffectType.SET_TAPPED]: "ウェイトにする", [EffectType.RETURN]: "戻る", [EffectType.JUMP]: "ジャンプ", [EffectType.JUMP_IF_FALSE]: "条件ジャンプ",
            "ENERGY": "[エネ{value}]支払い", "TAP_SELF": "自身ウェイト", "DISCARD_HAND": "手札{value}枚捨て", "RETURN_HAND": "手札に戻す", "SACRIFICE_SELF": "自身控え室", "REVEAL_HAND_ALL": "手札全公開", "SACRIFICE_UNDER": "下カード控え室", "DISCARD_ENERGY": "エネ破棄", "REVEAL_HAND": "手札公開",
            "TAP_MEMBER": "メンバーウェイト", "TAP_ENERGY": "エネウェイト", "REST_MEMBER": "メンバーウェイト", "RETURN_MEMBER_TO_HAND": "メンバーを手札に戻す", "DISCARD_MEMBER": "メンバー控え室", "DISCARD_LIVE": "ライブ控え室", "REMOVE_LIVE": "ライブ除外", "REMOVE_MEMBER": "メンバー除外", "RETURN_LIVE_TO_HAND": "ライブを手札に戻す", "RETURN_LIVE_TO_DECK": "ライブをデッキに戻す", "RETURN_MEMBER_TO_DECK": "メンバーをデッキに戻す",
            "PLACE_MEMBER_FROM_HAND": "手札からメンバー配置", "PLACE_LIVE_FROM_HAND": "手札からライブ配置", "PLACE_ENERGY_FROM_HAND": "手札からエネ配置", "PLACE_MEMBER_FROM_DISCARD": "控えからメンバー配置", "PLACE_LIVE_FROM_DISCARD": "控えからライブ配置", "PLACE_ENERGY_FROM_DISCARD": "控えからエネ配置", "PLACE_MEMBER_FROM_DECK": "デッキからメンバー配置", "PLACE_LIVE_FROM_DECK": "デッキからライブ配置", "PLACE_ENERGY_FROM_DECK": "デッキからエネ配置",
            "SHUFFLE_DECK": "デッキをシャッフル", "DRAW_CARD": "カードを引く", "DISCARD_TOP_DECK": "デッキ上を控え室に置く", "REMOVE_TOP_DECK": "デッキ上を除外", "RETURN_DISCARD_TO_DECK": "控えをデッキに戻す", "RETURN_REMOVED_TO_DECK": "除外をデッキに戻す", "RETURN_REMOVED_TO_HAND": "除外を手札に戻す", "RETURN_REMOVED_TO_DISCARD": "除外を控えに戻す",
            "PLACE_ENERGY_FROM_SUCCESS": "成功エリアからエネ配置", "DISCARD_SUCCESS_LIVE": "成功ライブ控え室", "REMOVE_SUCCESS_LIVE": "成功ライブ除外", "RETURN_SUCCESS_LIVE_TO_HAND": "成功ライブを手札に戻す", "RETURN_SUCCESS_LIVE_TO_INDEX": "成功ライブを位置戻し", "RETURN_SUCCESS_LIVE_TO_DISCARD": "成功ライブを控えに戻す", "PLACE_MEMBER_FROM_SUCCESS": "成功エリアからメンバー配置", "PLACE_LIVE_FROM_SUCCESS": "成功エリアからライブ配置",
            "PLACE_ENERGY_FROM_REMOVED": "除外からエネ配置", "PLACE_MEMBER_FROM_REMOVED": "除外からメンバー配置", "PLACE_LIVE_FROM_REMOVED": "除外からライブ配置", "RETURN_ENERGY_TO_DECK": "エネをデッキに戻す", "RETURN_ENERGY_TO_HAND": "エネを手札に戻す", "REMOVE_ENERGY": "エネ除外",
            "RETURN_STAGE_ENERGY_TO_DECK": "メンバーエネをデッキに戻す", "RETURN_STAGE_ENERGY_TO_HAND": "メンバーエネを手札に戻す", "DISCARD_STAGE_ENERGY": "メンバーエネ控え室", "REMOVE_STAGE_ENERGY": "メンバーエネ除外", "PLACE_ENERGY_FROM_STAGE_ENERGY": "メンバーエネをエネ配置", "PLACE_MEMBER_FROM_STAGE_ENERGY": "メンバーエネをメンバー配置", "PLACE_LIVE_FROM_STAGE_ENERGY": "メンバーエネをライブ配置",
            "PLACE_ENERGY_FROM_HAND_TO_STAGE_ENERGY": "手札からメンバーにエネ配置", "PLACE_MEMBER_FROM_HAND_TO_STAGE_ENERGY": "手札からメンバーにメンバー配置", "PLACE_LIVE_FROM_HAND_TO_STAGE_ENERGY": "手札からメンバーにライブ配置",
            "PLACE_ENERGY_FROM_DISCARD_TO_STAGE_ENERGY": "控えからメンバーにエネ配置", "PLACE_MEMBER_FROM_DISCARD_TO_STAGE_ENERGY": "控えからメンバーにメンバー配置", "PLACE_LIVE_FROM_DISCARD_TO_STAGE_ENERGY": "控えからメンバーにライブ配置",
            "PLACE_ENERGY_FROM_DECK_TO_STAGE_ENERGY": "デッキからメンバーにエネ配置", "PLACE_MEMBER_FROM_DECK_TO_STAGE_ENERGY": "デッキからメンバーにメンバー配置", "PLACE_LIVE_FROM_DECK_TO_STAGE_ENERGY": "デッキからメンバーにライブ配置",
            "PLACE_ENERGY_FROM_SUCCESS_TO_STAGE_ENERGY": "成功エリアからメンバーにエネ配置", "PLACE_MEMBER_FROM_SUCCESS_TO_STAGE_ENERGY": "成功エリアからメンバーにメンバー配置", "PLACE_LIVE_FROM_SUCCESS_TO_STAGE_ENERGY": "成功エリアからメンバーにライブ配置",
            "PLACE_ENERGY_FROM_REMOVED_TO_STAGE_ENERGY": "除外からメンバーにエネ配置", "PLACE_MEMBER_FROM_REMOVED_TO_STAGE_ENERGY": "除外からメンバーにメンバー配置", "PLACE_LIVE_FROM_REMOVED_TO_STAGE_ENERGY": "除外からメンバーにライブ配置",
            "RETURN_LIVE_TO_DISCARD": "ライブを控えに送る", "RETURN_LIVE_TO_REMOVED": "ライブを除外する", "RETURN_LIVE_TO_SUCCESS": "ライブを成功エリアに送る", "RETURN_MEMBER_TO_DISCARD": "メンバーを控えに送る", "RETURN_MEMBER_TO_REMOVED": "メンバーを除外する", "RETURN_MEMBER_TO_SUCCESS": "メンバーを成功エリアに送る", "RETURN_ENERGY_TO_DISCARD": "エネを控えに送る", "RETURN_ENERGY_TO_REMOVED": "エネを除外する", "RETURN_ENERGY_TO_SUCCESS": "エネを成功エリアに送る",
            "RETURN_SUCCESS_LIVE_TO_REMOVED": "成功ライブを除外する", "RETURN_REMOVED_SUCCESS": "除外から成功エリアに配置", "RETURN_STAGE_ENERGY_TO_DISCARD": "メンバーエネを控えに送る", "RETURN_STAGE_ENERGY_TO_REMOVED": "メンバーエネを除外する", "RETURN_STAGE_ENERGY_TO_SUCCESS": "メンバーエネを成功エリアに送る", "RETURN_DISCARD_TO_HAND": "控えを手札に戻す", "RETURN_DISCARD_TO_REMOVED": "控えを除外する",
            "SELECT_MODE": "モード選択", "CARD_DISCARD": "控え室", "SELF": "自分", "PLAYER": "自分", "OPPONENT": "相手", "ENEMY": "相手", "MEMBER_NAMED": "{name}", "ALL_MEMBERS": "全員", "ENEMY_STAGE": "相手場", "HAS_KEYWORD": "{group}の{keyword}が{zone}にある場合", "HAS_MOVED": "移動があった場合", "COUNT_SUCCESS_LIVE": "成功ライブ{value}枚以上の場合", "HAS_LIVE_CARD": "ライブカードがある場合", "TAP_PLAYER": "プレイヤーウェイト", "CARD_HAND": "手札", "TURN_1": "ターン1", "COUNT_STAGE": "場に{value}枚以上", "OPTION": "選択肢{value}", "ACTIVATE_ENERGY": "エネアクティブ化", "SUCCESS": "成功", "AREA": "エリア{value}", "TARGET_MEMBER": "メンバー", "IS_CENTER": "センターの場合", "HAS_MEMBER": "メンバーがいる場合", "HAS_COLOR": "色がある場合", "COUNT_HAND": "手札{value}枚以上", "COUNT_DISCARD": "控え{value}枚以上", "LIFE_LEAD": "リード時", "COUNT_GROUP": "グループ{value}枚以上", "GROUP_FILTER": "フィルタ{value}", "OPPONENT_HAS": "相手が所持", "SELF_IS_GROUP": "自身がグループ", "MODAL_ANSWER": "回答", "COUNT_ENERGY": "エネ{value}以上", "COST_CHECK": "コスト{value}以上", "RARITY_CHECK": "レアリティ{value}", "HAND_HAS_NO_LIVE": "手札ライブなし", "OPPONENT_HAND_DIFF": "相手手札差", "SCORE_COMPARE": "スコア比較", "HAS_CHOICE": "選択肢あり", "OPPONENT_CHOICE": "相手の選択", "COUNT_HEARTS": "ハート{value}以上", "COUNT_BLADES": "ブレード{value}以上", "OPPONENT_ENERGY_DIFF": "相手エネ差", "DECK_REFRESHED": "リフレッシュ後", "HAND_INCREASED": "手札増加", "COUNT_LIVE_ZONE": "ライブゾーン{value}枚以上", "BATON": "バトン", "TYPE_CHECK": "タイプ比較", "COUNT_THIS_TURN": "今ターン{value}回以上", "BATON_TOUCH": "バトンタッチ", "CHARGE_ENERGY": "エネチャージ{value}", "TARGET_AREA": "エリア{value}", "POSITION_CHANGE": "配置変更", "TARGET_PLAYER": "プレイヤー", "TYPE_MEMBER": "メンバータイプ", "SUCCESS_LIVE": "ライブ成功",
            "ACTIVATE_AND_SELF": "アクティブ&自身", "ADD_HEART": "ハート追加", "ADD_TAG": "タグ追加", "ALL": "全て", "ALL_AREAS": "全エリア", "ALL_ENERGY_ACTIVE": "全エネアクティブ", "ALL_MEMBER": "全メンバー", "ANY_NOT_TARGETED_THIS_TURN": "未対象", "AREA_IN": "エリア内",
            "BASE_BLADES_LE": "基本ブレード{value}以下", "BATON_COUNT": "バトン数", "BATON_FROM_NAME": "バトン元", "BATON_PASS": "バトンパス", "BATON_REPLACED": "バトン置換", "BATON_TOUCHED": "バトンタッチ済",
            "BLADE_GE_5": "ブレード5以上", "BLADE_GE_9": "ブレード9以上", "BLADE_LE_1": "ブレード1以下", "BLADE_LE_3": "ブレード3以下", "BOTH": "双方", "BOTH_PLAYERS": "双方プレイヤー", "CENTER": "センター", "CERISE": "Cerise Bouquet", "CHANGE_BASE_HEART": "基本ハート変更", "CHANGE_YELL_BLADE_COLOR": "エール色変更", "CHARGED_ENERGY": "チャージ済エネ",
            "COLOR_BLUE": "青", "COLOR_GREEN": "緑", "COLOR_PINK": "ピンク", "COLOR_PURPLE": "紫", "COLOR_RED": "赤", "COLOR_YELLOW": "黄", "CONSTANT": "永続",
            "COST_EQUAL_TARGET_PLUS_2": "コスト＝対象+2", "COST_GE": "コスト{value}以上", "COST_GE_10": "コスト10以上", "COST_GE_13": "コスト13以上", "COST_GT": "コスト{value}より上", "COST_GT_SELF": "コスト＞自身", "COST_LE": "コスト{value}以下", "COST_LEAD": "コストリード", "COST_LE_13": "コスト13以下", "COST_LE_2": "コスト2以下", "COST_LE_4": "コスト4以下", "COST_LE_9": "コスト9以下", "COST_LE_REVEALED": "コスト≦公開カード", "COST_LT_DISCARDED": "コスト＜捨て札", "COST_LT_SELF": "コスト＜自身",
            "COUNT": "回数", "COUNT_ACTIVATED": "起動数", "COUNT_CHARGED_ENERGY": "チャージ数", "COUNT_DISCARDED_THIS_TURN": "今ターン捨てた数", "COUNT_LIVE": "ライブ数", "COUNT_MEMBER": "メンバー数", "COUNT_PLAYED_THIS_TURN": "プレイ数", "COUNT_UNIQUE_NAMES": "種類数", "COUNT_YELL_REVEALED": "エール公開数", "CYCLE": "サイクル",
            "DECK_BOTTOM": "デッキ下", "DECK_REFRESHED_THIS_TURN": "今ターンリフレッシュ", "DECK_TOP": "デッキ上", "DISCARD": "控え室", "DISCARDED": "捨てられた", "DISCARDED_COST": "捨て札コスト", "DISCARDED_THESE": "これらを捨てる", "DISCARDED_THIS": "これが捨てられた", "DISCARD_ENERGY": "エネ破棄", "DISCARD_HEART": "ハート破棄", "DISCARD_STAGE": "退場", "DIVE": "DIVE!", "DOLLCHESTRA": "DOLLCHESTRA",
            "EFFECT_NEGATED_THIS_TURN": "効果無効化済", "EMOTION": "エモーション", "ENERGY_DECK": "エネデッキ", "ENERGY_LAGGING": "エネ劣勢", "ENERGY_LEAD": "エネ優勢", "ENERGY_REMOVED": "エネ削除", "EQUALS": "等しい", "EXTRA_HEARTS": "追加ハート", "FILTER": "フィルタ", "FROM": "〜から", "FROM_DISCARD": "控えから", "GREATER_THAN": "より大きい", "GROUP": "グループ", "GROUP_ID": "グループID",
            "HAND": "手札", "HAND_OTHER": "他の手札", "HAND_SIZE": "手札枚数", "HAND_SIZE_DIFF": "手札差", "HAS_ACTIVE_ENERGY": "アクティブエネあり", "HAS_ALL_BLADE": "全ブレード", "HAS_ALL_COLORS": "全色", "HAS_BLADE_HEART": "剣心あり", "HAS_EXCESS_HEART": "過剰ハート", "HAS_HEART_TYPE": "ハート種類あり", "HAS_LIVE_HEART_COLORS": "ライブハート色", "HAS_MOVED_THIS_TURN": "移動済", "HAS_REMAINING_HEART": "残ハートあり", "HAS_SUCCESS_LIVE": "成功ライブあり", "HAS_TYPE_LIVE": "タイプライブあり", "HEART_COLORS": "ハート色", "HEART_GE_3": "ハート3以上", "HEART_LEAD": "ハート優勢", "HEART_MIN": "ハート最小", "HEART_TYPE": "ハートタイプ", "HIGHEST_COST_ON_STAGE": "最大コスト",
            "INCREASE_HEART": "ハート増加", "IS_IN_DISCARD": "控えにある", "IS_IN_HAND": "手札にある", "IS_ON_STAGE": "場にある", "KALEIDOSCORE": "KALEIDOSCORE", "LEFT": "左", "LEFT_SIDE": "左側", "LESS_THAN": "より小さい", "LIST": "リスト", "LIVE": "ライブ", "LIVE_AREA": "ライブエリア", "LIVE_END": "ライブ終了", "LIVE_IN_PROGRESS": "ライブ中", "LOOK_AND_CHOOSE_ORDER": "見て順序選択", "LOOK_AND_CHOOSE_REVEALED": "見て公開選択",
            "MAIN_PHASE": "メインフェイズ", "MATCH_BASE_BLADE": "基本ブレード一致", "MATCH_COST": "コスト一致", "MATCH_HEART": "ハート一致", "MAX": "最大", "MEMBER": "メンバー", "MEMBER_AT_SLOT": "スロットのメンバー", "MIN": "最小", "MIRAKURA": "みらくらぱーく", "MODE": "モード", "NAME": "名前", "NAME_IN": "名前含む", "NEXT_TURN": "次ターン", "NONE": "なし", "NON_CENTER": "非センター", "NOT_HAS_BLADE_HEART": "剣心なし", "NOT_MOVED_THIS_TURN": "未移動", "NOT_NAME": "名前以外", "NOT_TARGET": "非対象", "NO_ABILITY": "能力なし", "ON_LIVE_START": "ライブ開始時", "ON_LIVE_SUCCESS": "ライブ成功時", "ON_PLAY": "登場時", "OPPONENT_HAS_WAIT": "相手ウェイトあり", "OPPONENT_LIVE": "相手ライブ", "OPPONENT_SCORE_ZERO_THIS_TURN": "相手スコア0", "OPPONENT_STAGE": "相手ステージ", "OPPONENT_WAIT": "相手ウェイト", "OTHER": "その他", "OTHER_MEMBER": "他メンバー", "OTHER_UNIT_MIRAKURA": "他みらくら",
            "PER_CARD": "枚数につき", "PER_ENERGY": "エネにつき", "PER_ENERGY_PAID": "支払エネにつき", "PER_HAND": "手札につき", "PLAYED": "プレイ済", "PLAY_LIVE_FROM_HAND": "手札からライブ", "PREVENT_LIVE": "ライブ制限", "PREVENT_SET_TO_SUCCESS_PILE": "成功エリア制限", "PRINTEMPS": "Printemps", "RECOVERED_THIS": "回収済", "REDUCE_HEART": "ハート減少", "REDUCE_PER": "減少量", "REMOVE_ENERGY_FROM_MEMBER": "エネ除去", "REPLACE_DISCARDED": "捨て札置換", "RESET_YELL_HEARTS": "エールリセット", "REVEALED_LIVE": "公開ライブ", "REVEALED_OPTIONS": "公開肢", "REVEALED_THIS": "公開済", "REVERSED": "反転", "RIGHT": "右", "RIGHT_SIDE": "右側",
            "SAME_AS_TARGET": "対象と同じ", "SAME_NAME_MEMBER": "同名メンバー", "SAME_SLOT": "同枠", "SCORE": "スコア", "SCORE_GE": "スコア{value}以上", "SCORE_LE": "スコア{value}以下", "SCORE_LEAD": "スコア優勢", "SCORE_LE_3": "スコア3以下", "SCORE_TOTAL": "合計スコア", "SELECTED": "選択された", "SET_BASE_BLADES": "基本ブレード設定", "SLOT": "スロット", "STAGE": "ステージ", "STAGE_ENTRY": "登場", "SUB_GROUP": "サブグループ", "SUCCESS_PILE": "成功エリア", "SUM_COST": "合計コスト", "SUM_COST_IN": "内部合計コスト", "SUM_ENERGY": "合計エネ", "SUM_HEARTS": "合計ハート", "SUM_SCORE": "合計スコア", "SUM_SUCCESS_LIVE": "成功ライブ計",
            "TAPPED": "ウェイト", "TARGET": "対象", "TARGET_1": "対象1", "TARGET_2": "対象2", "TARGET_CARD": "対象カード", "TARGET_FILTER": "対象フィルタ", "TARGET_LIVE": "対象ライブ", "TIE_BREAKER": "タイブレーク", "TONIGHT": "TONIGHT", "TRIGGER": "誘発", "TRIGGER_YELL_AGAIN": "エール再誘発", "TRUE": "真", "TYPE": "タイプ", "TYPE_LIVE": "ライブタイプ",
            "UNIQUE_HEART_COLORS": "ユニークハート色", "UNIQUE_NAMES": "ユニーク名", "UNIT": "ユニット", "UNIT_BIBI": "BiBi", "UNIT_CERISE": "Cerise Bouquet", "UNIT_DOLL": "DOLLCHESTRA", "UNIT_HASU": "スリーズブーケ", "UNIT_LILYWHITE": "lily white", "UNIT_MIRAKURA": "みらくらぱーく", "UNIT_PRINTEMPS": "Printemps", "UNTIL": "まで", "WAIT": "待機", "YELL_COUNT": "エール数", "YELL_REVEALED": "エール公開", "ZONE": "ゾーン",
            "REMOVE_SELF": "自身退場",
            "CHARGE_SELF": "自身チャージ", "DISCARD_SELF": "自身捨て", "LOOK_AND_CHOOSE_REVEAL": "見て公開", "SELECT_LIVE_CARD": "ライブ選択", "SELECT_REVEALED": "公開札選択",
            "ADD_HAND": "手札追加", "MOVE_TO_HAND": "手札移動", "MOVE_DECK": "デッキ移動", "MOVE_DISCARD": "控え移動",
            "COUNT_SUCCESS_LIVES": "成功ライブ{value}枚以上", "REDUCE_LIMIT": "リミット低下", "SUCCESS_LIVES_CONTAINS": "成功ライブに含む", "REVEALED_CONTAINS": "公開札に含む",
            "MATCH_RECOVERED": "回収札一致", "IS_MAIN_PHASE": "メインフェイズ",
            "ON_STAGE_ENTRY": "登場時", "ON_MEMBER_DISCARD": "メンバー退場時", "ON_OPPONENT_TAP": "相手ウェイト時"
        },
        params: { "UNTIL": { "live_end": "ライブ終了まで", "turn_end": "終了まで" }, "COLOR": { "PINK": "ピンク", "RED": "赤", "YELLOW": "黄", "GREEN": "緑", "BLUE": "青", "PURPLE": "紫", "ALL": "全色" }, "GROUP": "{v}", "COST_MAX": "コスト{v}以下", "COUNT": "{v}回", "KEYWORD": "{v}", "ZONE": "{v}", "FILTER": { "energy": "エネ", "member": "メンバー" }, "AREA": { "0": "左サイド", "1": "センター", "2": "右サイド", "LEFT_SIDE": "左サイド", "CENTER": "センター", "RIGHT_SIDE": "右サイド" }, "GROUPS": { "虹ヶ咲": "虹ヶ咲", "リエラ": "Liella!", "μ's": "μ's", "Aqours": "Aqours", "蓮ノ空": "蓮ノ空" }, "TO": { "bottom": "底", "top": "上" }, "ZONES": { "REVEALED": "公開領域", "DISCARD": "控え室", "HAND": "手札", "ENERGY": "エネ" } },
        misc: { "optional": "(任意)", "cost_prefix": "", "effect_prefix": "→ ", "condition_prefix": "条件: ", "once_per_turn": "(ターン1回)" }
    },
    en: {
        triggers: { [TriggerType.ON_PLAY]: "[Play]", [TriggerType.ON_LIVE_START]: "[Live Start]", [TriggerType.ON_LIVE_SUCCESS]: "[Live Success]", [TriggerType.TURN_START]: "[Turn Start]", [TriggerType.TURN_END]: "[Turn End]", [TriggerType.CONSTANT]: "[Const]", [TriggerType.ACTIVATED]: "[Activate]", [TriggerType.ON_LEAVES]: "[Exit]", [TriggerType.ON_REVEAL]: "[Reveal]", [TriggerType.ON_POSITION_CHANGE]: "[On Move]" },
        opcodes: {
            [EffectType.SET_TARGET_SELF]: "Target: Self", [EffectType.SET_TARGET_PLAYER]: "Target: Player", [EffectType.SET_TARGET_OPPONENT]: "Target: Opponent", [EffectType.SET_TARGET_ALL_PLAYERS]: "Target: All Players", [EffectType.SET_TARGET_MEMBER_SELF]: "Target: My Member", [EffectType.SET_TARGET_MEMBER_OTHER]: "Target: Other Member", [EffectType.SET_TARGET_CARD_HAND]: "Target: Hand", [EffectType.SET_TARGET_CARD_DISCARD]: "Target: Discard", [EffectType.SET_TARGET_CARD_DECK_TOP]: "Target: Deck Top", [EffectType.SET_TARGET_OPPONENT_HAND]: "Target: Opp Hand", [EffectType.SET_TARGET_MEMBER_SELECT]: "Select Target", [EffectType.SET_TARGET_MEMBER_NAMED]: "Target Named",
            [EffectType.DRAW]: "Draw {value} card(s)",
            [EffectType.ADD_BLADES]: "Gain 【Blades】{value}",
            [EffectType.ADD_HEARTS]: "Gain 【Hearts】{value}",
            [EffectType.REDUCE_COST]: "Cost -{value}",
            [EffectType.LOOK_DECK]: "Look at top {value} card(s)",
            [EffectType.RECOVER_LIVE]: "Retrieve {value} Live(s)",
            [EffectType.BOOST_SCORE]: "Score +{value}",
            [EffectType.RECOVER_MEMBER]: "Retrieve {value} Member(s)",
            [EffectType.BUFF_POWER]: "Power +{value}",
            [EffectType.IMMUNITY]: "Immune to effects",
            [EffectType.MOVE_MEMBER]: "Move Member",
            [EffectType.SWAP_CARDS]: "Swap Hand cards",
            [EffectType.SEARCH_DECK]: "Search Deck",
            [EffectType.ENERGY_CHARGE]: "Charge {value} Energy",
            [EffectType.ACTIVATE_MEMBER]: "Activate {value} {filter}",
            [EffectType.ADD_TO_HAND]: "Add {value} to Hand",
            [EffectType.MOVE_TO_DECK]: "Return {value} to {to}",
            [EffectType.REVEAL_CARDS]: "Reveal {value}",
            [EffectType.LOOK_AND_CHOOSE]: "Look & Choose {value}",
            [EffectType.TAP_OPPONENT]: "Tap {value} Opponent(s)",
            [EffectType.TAP_MEMBER]: "Tap {value} Self Member(s)",
            [EffectType.PLAY_MEMBER_FROM_HAND]: "Play from hand",
            [EffectType.SET_BLADES]: "Set 【Blades】 to {value}",
            [EffectType.SET_HEARTS]: "Set 【Hearts】 to {value}",
            [EffectType.FORMATION_CHANGE]: "Formation Change",
            [EffectType.NEGATE_EFFECT]: "Negate Effect",
            [EffectType.ORDER_DECK]: "Reorder top {value}",
            [EffectType.BATON_TOUCH_MOD]: "Baton Mod",
            [EffectType.SET_SCORE]: "Set Score to {value}",
            [EffectType.SWAP_ZONE]: "Move Card",
            [EffectType.TRANSFORM_COLOR]: "Change Color",
            [EffectType.TRIGGER_REMOTE]: "Trigger Remote Ability",
            [EffectType.REDUCE_HEART_REQ]: "Reduce Heart Req.",
            [EffectType.CHEER_REVEAL]: "Reveal via Cheer",
            [EffectType.COLOR_SELECT]: "Select Color",
            [EffectType.REPLACE_EFFECT]: "Replace Effect",
            [EffectType.MODIFY_SCORE_RULE]: "Modify Score Rule",
            [EffectType.MOVE_TO_DISCARD]: "Discard {value}",
            [EffectType.GRANT_ABILITY]: "Grant Ability",
            [EffectType.INCREASE_HEART_COST]: "Heart Cost +{value}",
            [EffectType.REDUCE_YELL_COUNT]: "Reduce E-Req by {value}",
            [EffectType.PLAY_MEMBER_FROM_DISCARD]: "Play from Discard",
            [EffectType.PAY_ENERGY]: "Spend 【Energy】{value}",
            [EffectType.ENERGY]: "Gain 【Energy】{value}",
            [EffectType.SELECT_MEMBER]: "Select {value} Member(s)",
            [EffectType.DRAW_UNTIL]: "Draw until {value} in hand",
            [EffectType.SELECT_PLAYER]: "Select Player",
            [EffectType.SELECT_LIVE]: "Select Live",
            [EffectType.REVEAL_UNTIL]: "Reveal until {value}",
            [EffectType.INCREASE_COST]: "Cost +{value}",
            [EffectType.PREVENT_PLAY_TO_SLOT]: "Limit Play to Slot",
            [EffectType.SWAP_AREA]: "Move Area",
            [EffectType.TRANSFORM_HEART]: "Change Heart Type",
            [EffectType.SELECT_CARDS]: "Select {value} Card(s)",
            [EffectType.OPPONENT_CHOOSE]: "Opponent Chooses",
            [EffectType.PLAY_LIVE_FROM_DISCARD]: "Play Live from Discard",
            [EffectType.REDUCE_LIVE_SET_LIMIT]: "Reduce Live Set Limit",
            [EffectType.PREVENT_SET_TO_SUCCESS_PILE]: "Limit Success Pile",
            [EffectType.PREVENT_BATON_TOUCH]: "Limit Baton Touch",
            [EffectType.PREVENT_ACTIVATE]: "Limit Activation",
            [EffectType.ACTIVATE_ENERGY]: "Activate {value} Energy",
            [EffectType.ADD_STAGE_ENERGY]: "Add Energy to Member",
            [EffectType.FLAVOR]: "Flavor Action",
            [EffectType.META_RULE]: "[Rule Change]",
            [EffectType.PLACE_UNDER]: "Place Under",
            [EffectType.RESTRICTION]: "Restriction",
            [EffectType.SET_HEART_COST]: "Set Heart Cost to {value}",
            [EffectType.ADD_CONTINUOUS]: "Grant Continuous Ability",
            [EffectType.SET_TAPPED]: "Tapped",
            [EffectType.RETURN]: "Return",
            [EffectType.JUMP]: "Jump",
            [EffectType.JUMP_IF_FALSE]: "Jump if False",
            [ConditionCheck.TURN_1]: "Once per turn",
            [ConditionCheck.HAS_MEMBER]: "If you have a Member",
            [ConditionCheck.HAS_COLOR]: "If you have Color",
            [ConditionCheck.COUNT_STAGE]: "If {value}+ Members on Stage",
            [ConditionCheck.COUNT_HAND]: "If {value}+ cards in Hand",
            [ConditionCheck.COUNT_DISCARD]: "If {value}+ cards in Discard",
            [ConditionCheck.IS_CENTER]: "If Center",
            [ConditionCheck.LIFE_LEAD]: "If leading",
            [ConditionCheck.COUNT_GROUP]: "If {value}+ in Group",
            [ConditionCheck.GROUP_FILTER]: "Filter {value}+",
            [ConditionCheck.OPPONENT_HAS]: "If Opponent has",
            [ConditionCheck.SELF_IS_GROUP]: "If Self is Group",
            [ConditionCheck.MODAL_ANSWER]: "Answered",
            [ConditionCheck.COUNT_ENERGY]: "If {value}+ Energy",
            [ConditionCheck.HAS_LIVE_CARD]: "If you have a Live card",
            [ConditionCheck.COST_CHECK]: "Cost {value}+",
            [ConditionCheck.RARITY_CHECK]: "Rarity {value}",
            [ConditionCheck.HAND_HAS_NO_LIVE]: "No Live in Hand",
            [ConditionCheck.COUNT_SUCCESS_LIVE]: "If {value}+ Success Lives",
            [ConditionCheck.OPPONENT_HAND_DIFF]: "Opponent Hand difference",
            [ConditionCheck.SCORE_COMPARE]: "Score Compare",
            [ConditionCheck.HAS_CHOICE]: "If choice exists",
            [ConditionCheck.OPPONENT_CHOICE]: "Opponent Choice",
            [ConditionCheck.COUNT_HEARTS]: "If {value}+ Hearts",
            [ConditionCheck.COUNT_BLADES]: "If {value}+ Blades",
            [ConditionCheck.OPPONENT_ENERGY_DIFF]: "Opponent Energy difference",
            [ConditionCheck.HAS_KEYWORD]: "If {group} {keyword} is in {zone}",
            [ConditionCheck.DECK_REFRESHED]: "If Deck Refreshed",
            [ConditionCheck.HAS_MOVED]: "If moved",
            [ConditionCheck.HAND_INCREASED]: "If Hand increased",
            [ConditionCheck.COUNT_LIVE_ZONE]: "If {value}+ cards in Live Zone",
            [ConditionCheck.BATON]: "Baton",
            [ConditionCheck.TYPE_CHECK]: "Type Check",
            [ConditionCheck.IS_IN_DISCARD]: "If in Discard",
            [ConditionCheck.AREA_CHECK]: "If in the {value}",
            "ENERGY": "Spend [Energy {value}]", "TAP_SELF": "Tap Self", "DISCARD_HAND": "Discard {value} Hand", "RETURN_HAND": "Return to Hand", "SACRIFICE_SELF": "Sacrifice Self", "REVEAL_HAND_ALL": "Reveal Hand", "SACRIFICE_UNDER": "Discard Under", "DISCARD_ENERGY": "Discard Energy", "REVEAL_HAND": "Reveal {value} Hand",
            "TAP_MEMBER": "Tap Member", "TAP_ENERGY": "Tap Energy", "REST_MEMBER": "Tap Member", "RETURN_MEMBER_TO_HAND": "Return Member to Hand", "DISCARD_MEMBER": "Discard Member", "DISCARD_LIVE": "Discard Live", "REMOVE_LIVE": "Remove Live", "REMOVE_MEMBER": "Remove Member", "RETURN_LIVE_TO_HAND": "Return Live to Hand", "RETURN_LIVE_TO_DECK": "Return Live to Deck", "RETURN_MEMBER_TO_DECK": "Return Member to Deck",
            "PLACE_MEMBER_FROM_HAND": "Place Member from Hand", "PLACE_LIVE_FROM_HAND": "Place Live from Hand", "PLACE_ENERGY_FROM_HAND": "Place Energy from Hand", "PLACE_MEMBER_FROM_DISCARD": "Place Member from Discard", "PLACE_LIVE_FROM_DISCARD": "Place Live from Discard", "PLACE_ENERGY_FROM_DISCARD": "Place Energy from Discard", "PLACE_MEMBER_FROM_DECK": "Place Member from Deck", "PLACE_LIVE_FROM_DECK": "Place Live from Deck", "PLACE_ENERGY_FROM_DECK": "Place Energy from Deck",
            "SHUFFLE_DECK": "Shuffle Deck", "DRAW_CARD": "Draw a card", "DISCARD_TOP_DECK": "Discard top of Deck", "REMOVE_TOP_DECK": "Remove top of Deck", "RETURN_DISCARD_TO_DECK": "Return Discard to Deck", "RETURN_REMOVED_TO_DECK": "Return Removed to Deck", "RETURN_REMOVED_TO_HAND": "Return Removed to Hand", "RETURN_REMOVED_TO_DISCARD": "Return Removed to Discard",
            "PLACE_ENERGY_FROM_SUCCESS": "Place Energy from Success", "DISCARD_SUCCESS_LIVE": "Discard Success Live", "REMOVE_SUCCESS_LIVE": "Remove Success Live", "RETURN_SUCCESS_LIVE_TO_HAND": "Return Success Live to Hand", "RETURN_SUCCESS_LIVE_TO_INDEX": "Return Success Live to Index", "RETURN_SUCCESS_LIVE_TO_DISCARD": "Return Success Live to Discard", "PLACE_MEMBER_FROM_SUCCESS": "Place Member from Success", "PLACE_LIVE_FROM_SUCCESS": "Place Live from Success",
            "PLACE_ENERGY_FROM_REMOVED": "Place Energy from Removed", "PLACE_MEMBER_FROM_REMOVED": "Place Member from Removed", "PLACE_LIVE_FROM_REMOVED": "Place Live from Removed", "RETURN_ENERGY_TO_DECK": "Return Energy to Deck", "RETURN_ENERGY_TO_HAND": "Return Energy to Hand", "REMOVE_ENERGY": "Remove Energy",
            "RETURN_STAGE_ENERGY_TO_DECK": "Return Member Energy to Deck", "RETURN_STAGE_ENERGY_TO_HAND": "Return Member Energy to Hand", "DISCARD_STAGE_ENERGY": "Discard Member Energy", "REMOVE_STAGE_ENERGY": "Remove Member Energy", "PLACE_ENERGY_FROM_STAGE_ENERGY": "Place Member Energy to Energy", "PLACE_MEMBER_FROM_STAGE_ENERGY": "Place Member Energy to Member", "PLACE_LIVE_FROM_STAGE_ENERGY": "Place Member Energy to Live",
            "PLACE_ENERGY_FROM_HAND_TO_STAGE_ENERGY": "Place Energy from Hand to Member", "PLACE_MEMBER_FROM_HAND_TO_STAGE_ENERGY": "Place Member from Hand to Member", "PLACE_LIVE_FROM_HAND_TO_STAGE_ENERGY": "Place Live from Hand to Member",
            "PLACE_ENERGY_FROM_DISCARD_TO_STAGE_ENERGY": "Place Energy from Discard to Member", "PLACE_MEMBER_FROM_DISCARD_TO_STAGE_ENERGY": "Place Member from Discard to Member", "PLACE_LIVE_FROM_DISCARD_TO_STAGE_ENERGY": "Place Live from Discard to Member",
            "PLACE_ENERGY_FROM_DECK_TO_STAGE_ENERGY": "Place Energy from Deck to Member", "PLACE_MEMBER_FROM_DECK_TO_STAGE_ENERGY": "Place Member from Deck to Member", "PLACE_LIVE_FROM_DECK_TO_STAGE_ENERGY": "Place Live from Deck to Member",
            "PLACE_ENERGY_FROM_SUCCESS_TO_STAGE_ENERGY": "Place Energy from Success to Member", "PLACE_MEMBER_FROM_SUCCESS_TO_STAGE_ENERGY": "Place Member from Success to Member", "PLACE_LIVE_FROM_SUCCESS_TO_STAGE_ENERGY": "Place Live from Success to Member",
            "PLACE_ENERGY_FROM_REMOVED_TO_STAGE_ENERGY": "Place Energy from Removed to Member", "PLACE_MEMBER_FROM_REMOVED_TO_STAGE_ENERGY": "Place Member from Removed to Member", "PLACE_LIVE_FROM_REMOVED_TO_STAGE_ENERGY": "Place Live from Removed to Member",
            "RETURN_LIVE_TO_DISCARD": "Return Live to Discard", "RETURN_LIVE_TO_REMOVED": "Remove Live", "RETURN_LIVE_TO_SUCCESS": "Place Live in Success Pile", "RETURN_MEMBER_TO_DISCARD": "Discard Member", "RETURN_MEMBER_TO_REMOVED": "Remove Member", "RETURN_MEMBER_TO_SUCCESS": "Place Member in Success Pile", "RETURN_ENERGY_TO_DISCARD": "Discard Energy", "RETURN_ENERGY_TO_REMOVED": "Remove Energy", "RETURN_ENERGY_TO_SUCCESS": "Place Energy in Success Pile",
            "RETURN_SUCCESS_LIVE_TO_REMOVED": "Remove Success Live", "RETURN_REMOVED_SUCCESS": "Place Removed in Success Pile", "RETURN_STAGE_ENERGY_TO_DISCARD": "Discard Member Energy", "RETURN_STAGE_ENERGY_TO_REMOVED": "Remove Member Energy", "RETURN_STAGE_ENERGY_TO_SUCCESS": "Place Member Energy in Success Pile", "RETURN_DISCARD_TO_HAND": "Return Discard to Hand", "RETURN_DISCARD_TO_REMOVED": "Remove Discard",
            "DISCARD_HAND": "Discard {value} from Hand", "SELECT_MODE": "Select Mode", "SELF": "Self", "PLAYER": "Player", "OPPONENT": "Opponent", "TARGET_MEMBER": "Member", "OPTION": "Option {value}", "SUCCESS": "Success", "AREA": "Area {value}", "TURN_1": "Turn 1", "TAPPED": "Tapped", "MEMBER_NAMED": "{name}", "TYPE_MEMBER": "Member Type", "SUCCESS_LIVE": "Success Live", "HAS_KEYWORD": "If {group} {keyword} in {zone}", "HAS_MOVED": "If {group} moved this turn", "COUNT_SUCCESS_LIVE": "If {value}+ Cleared Lives", "HAS_LIVE_CARD": "If Live Card present", "TAP_PLAYER": "Tap Player", "CARD_HAND": "Hand", "COUNT_STAGE": "Stage {value}+", "ACTIVATE_ENERGY": "Energy Activation", "IS_CENTER": "If Center", "HAS_MEMBER": "If Member present", "HAS_COLOR": "If Color present", "COUNT_HAND": "Hand {value}+", "COUNT_DISCARD": "Discard {value}+", "LIFE_LEAD": "If Leading", "COUNT_GROUP": "Group {value}+", "GROUP_FILTER": "Filter {value}", "OPPONENT_HAS": "If Opponent has", "SELF_IS_GROUP": "If Self in Group", "MODAL_ANSWER": "Answer", "COUNT_ENERGY": "Energy {value}+", "COST_CHECK": "Cost {value}+", "RARITY_CHECK": "Rarity {value}", "HAND_HAS_NO_LIVE": "No Live in Hand", "OPPONENT_HAND_DIFF": "Opponent Hand Difference", "SCORE_COMPARE": "Score Compare", "HAS_CHOICE": "Has Choice", "OPPONENT_CHOICE": "Opponent Choice", "COUNT_HEARTS": "Hearts {value}+", "COUNT_BLADES": "Blades {value}+", "OPPONENT_ENERGY_DIFF": "Opponent Energy Difference", "DECK_REFRESHED": "Post-Refresh", "HAND_INCREASED": "Hand Increased", "COUNT_LIVE_ZONE": "Live Zone {value}+", "BATON": "Baton", "TYPE_CHECK": "Type Comparison", "COUNT_THIS_TURN": "{value}+ times this turn", "BATON_TOUCH": "Baton Touch", "CHARGE_ENERGY": "Charge {value} Energy", "TARGET_AREA": "Area {value}", "POSITION_CHANGE": "Position Change", "TARGET_PLAYER": "Target Player", "REVEAL_HAND": "Reveal Hand",
            "ACTIVATE_AND_SELF": "Activate & Self", "ADD_HEART": "Add Heart", "ADD_TAG": "Add Tag", "ALL": "All", "ALL_AREAS": "All Areas", "ALL_ENERGY_ACTIVE": "All Energy Active", "ALL_MEMBER": "All Members", "ANY_NOT_TARGETED_THIS_TURN": "Not Targeted", "AREA_IN": "In Area",
            "BASE_BLADES_LE": "Base Blades {value} or less", "BATON_COUNT": "Baton Count", "BATON_FROM_NAME": "Baton Source", "BATON_PASS": "Baton Pass", "BATON_REPLACED": "Baton Replaced", "BATON_TOUCHED": "Baton Touched",
            "BLADE_GE_5": "Blades 5+", "BLADE_GE_9": "Blades 9+", "BLADE_LE_1": "Blades 1 or less", "BLADE_LE_3": "Blades 3 or less", "BOTH": "Both", "BOTH_PLAYERS": "Both Players", "CENTER": "Center", "CERISE": "Cerise Bouquet", "CHANGE_BASE_HEART": "Change Base Heart", "CHANGE_YELL_BLADE_COLOR": "Change Yell Color", "CHARGED_ENERGY": "Charged Energy",
            "COLOR_BLUE": "Blue", "COLOR_GREEN": "Green", "COLOR_PINK": "Pink", "COLOR_PURPLE": "Purple", "COLOR_RED": "Red", "COLOR_YELLOW": "Yellow", "CONSTANT": "Constant",
            "COST_EQUAL_TARGET_PLUS_2": "Cost = Target + 2", "COST_GE": "Cost {value}+", "COST_GE_10": "Cost 10+", "COST_GE_13": "Cost 13+", "COST_GT": "Cost > {value}", "COST_GT_SELF": "Cost > Self", "COST_LE": "Cost {value}-", "COST_LEAD": "Cost Lead", "COST_LE_13": "Cost 13-", "COST_LE_2": "Cost 2-", "COST_LE_4": "Cost 4-", "COST_LE_9": "Cost 9-", "COST_LE_REVEALED": "Cost <= Revealed", "COST_LT_DISCARDED": "Cost < Discarded", "COST_LT_SELF": "Cost < Self",
            "COUNT": "Count", "COUNT_ACTIVATED": "Activation Count", "COUNT_CHARGED_ENERGY": "Charge Count", "COUNT_DISCARDED_THIS_TURN": "Discard Count This Turn", "COUNT_LIVE": "Live Count", "COUNT_MEMBER": "Member Count", "COUNT_PLAYED_THIS_TURN": "Play Count", "COUNT_UNIQUE_NAMES": "Unique Names", "COUNT_YELL_REVEALED": "Yell Reveal Count", "CYCLE": "Cycle",
            "DECK_BOTTOM": "Deck Bottom", "DECK_REFRESHED_THIS_TURN": "Refreshed This Turn", "DECK_TOP": "Deck Top", "DISCARD": "Discard", "DISCARDED": "Discarded", "DISCARDED_COST": "Discarded Cost", "DISCARDED_THESE": "Discard These", "DISCARDED_THIS": "Discard This", "DISCARD_ENERGY": "Discard Energy", "DISCARD_HEART": "Discard Heart", "DISCARD_STAGE": "Retire", "DIVE": "DIVE!", "DOLLCHESTRA": "DOLLCHESTRA",
            "EFFECT_NEGATED_THIS_TURN": "Effect Negated", "EMOTION": "Emotion", "ENERGY_DECK": "Energy Deck", "ENERGY_LAGGING": "Energy Lagging", "ENERGY_LEAD": "Energy Lead", "ENERGY_REMOVED": "Energy Removed", "EQUALS": "Equals", "EXTRA_HEARTS": "Extra Hearts", "FILTER": "Filter", "FROM": "From", "FROM_DISCARD": "From Discard", "GREATER_THAN": "Greater Than", "GROUP": "Group", "GROUP_ID": "Group ID",
            "HAND": "Hand", "HAND_OTHER": "Other Hand", "HAND_SIZE": "Hand Size", "HAND_SIZE_DIFF": "Hand Size Diff", "HAS_ACTIVE_ENERGY": "Has Active Energy", "HAS_ALL_BLADE": "Has All Blades", "HAS_ALL_COLORS": "Has All Colors", "HAS_BLADE_HEART": "Has Blade Heart", "HAS_EXCESS_HEART": "Has Excess Heart", "HAS_HEART_TYPE": "Has Heart Type", "HAS_LIVE_HEART_COLORS": "Live Heart Colors", "HAS_MOVED_THIS_TURN": "Moved This Turn", "HAS_REMAINING_HEART": "Has Remaining Heart", "HAS_SUCCESS_LIVE": "Has Success Live", "HAS_TYPE_LIVE": "Has Type Live", "HEART_COLORS": "Heart Colors", "HEART_GE_3": "Hearts 3+", "HEART_LEAD": "Heart Lead", "HEART_MIN": "Heart Min", "HEART_TYPE": "Heart Type", "HIGHEST_COST_ON_STAGE": "Highest Cost",
            "INCREASE_HEART": "Increase Heart", "IS_IN_DISCARD": "In Discard", "IS_IN_HAND": "In Hand", "IS_ON_STAGE": "On Stage", "KALEIDOSCORE": "KALEIDOSCORE", "LEFT": "Left", "LEFT_SIDE": "Left Side", "LESS_THAN": "Less Than", "LIST": "List", "LIVE": "Live", "LIVE_AREA": "Live Area", "LIVE_END": "Live End", "LIVE_IN_PROGRESS": "Live In Progress", "LOOK_AND_CHOOSE_ORDER": "Look & Order", "LOOK_AND_CHOOSE_REVEALED": "Look & Reveal",
            "MAIN_PHASE": "Main Phase", "MATCH_BASE_BLADE": "Match Base Blade", "MATCH_COST": "Match Cost", "MATCH_HEART": "Match Heart", "MAX": "Max", "MEMBER": "Member", "MEMBER_AT_SLOT": "Member at Slot", "MIN": "Min", "MIRAKURA": "Hasunosora", "MODE": "Mode", "NAME": "Name", "NAME_IN": "Name In", "NEXT_TURN": "Next Turn", "NONE": "None", "NON_CENTER": "Non-Center", "NOT_HAS_BLADE_HEART": "No Blade Heart", "NOT_MOVED_THIS_TURN": "Not Moved", "NOT_NAME": "Not Name", "NOT_TARGET": "Not Target", "NO_ABILITY": "No Ability", "ON_LIVE_START": "Live Start", "ON_LIVE_SUCCESS": "Live Success", "ON_PLAY": "On Play", "OPPONENT_HAS_WAIT": "Opponent Has Wait", "OPPONENT_LIVE": "Opponent Live", "OPPONENT_SCORE_ZERO_THIS_TURN": "Opponent Score 0", "OPPONENT_STAGE": "Opponent Stage", "OPPONENT_WAIT": "Opponent Wait", "OTHER": "Other", "OTHER_MEMBER": "Other Member", "OTHER_UNIT_MIRAKURA": "Other Hasunosora",
            "PER_CARD": "Per Card", "PER_ENERGY": "Per Energy", "PER_ENERGY_PAID": "Per Energy Paid", "PER_HAND": "Per Hand", "PLAYED": "Played", "PLAY_LIVE_FROM_HAND": "Play Live from Hand", "PREVENT_LIVE": "Prevent Live", "PREVENT_SET_TO_SUCCESS_PILE": "Prevent Success", "PRINTEMPS": "Printemps", "RECOVERED_THIS": "Recovered This", "REDUCE_HEART": "Reduce Heart", "REDUCE_PER": "Reduce Per", "REMOVE_ENERGY_FROM_MEMBER": "Remove Energy", "REPLACE_DISCARDED": "Replace Discarded", "RESET_YELL_HEARTS": "Reset Yell Hearts", "REVEALED_LIVE": "Revealed Live", "REVEALED_OPTIONS": "Revealed Options", "REVEALED_THIS": "Revealed This", "REVERSED": "Reversed", "RIGHT": "Right", "RIGHT_SIDE": "Right Side",
            "SAME_AS_TARGET": "Same as Target", "SAME_NAME_MEMBER": "Same Name Member", "SAME_SLOT": "Same Slot", "SCORE": "Score", "SCORE_GE": "Score {value}+", "SCORE_LE": "Score {value}-", "SCORE_LEAD": "Score Lead", "SCORE_LE_3": "Score 3-", "SCORE_TOTAL": "Score Total", "SELECTED": "Selected", "SET_BASE_BLADES": "Set Base Blades", "SLOT": "Slot", "STAGE": "Stage", "STAGE_ENTRY": "Stage Entry", "SUB_GROUP": "Sub-Group", "SUCCESS_PILE": "Success Pile", "SUM_COST": "Sum Cost", "SUM_COST_IN": "Sum Cost In", "SUM_ENERGY": "Sum Energy", "SUM_HEARTS": "Sum Hearts", "SUM_SCORE": "Sum Score", "SUM_SUCCESS_LIVE": "Sum Success Live",
            "TAPPED": "Resting", "TARGET": "Target", "TARGET_1": "Target 1", "TARGET_2": "Target 2", "TARGET_CARD": "Target Card", "TARGET_FILTER": "Target Filter", "TARGET_LIVE": "Target Live", "TIE_BREAKER": "Tie Breaker", "TONIGHT": "TONIGHT", "TRIGGER": "Trigger", "TRIGGER_YELL_AGAIN": "Trigger Yell Again", "TRUE": "True", "TYPE": "Type", "TYPE_LIVE": "Live Type",
            "UNIQUE_HEART_COLORS": "Unique Heart Colors", "UNIQUE_NAMES": "Unique Names", "UNIT": "Unit", "UNIT_BIBI": "BiBi", "UNIT_CERISE": "Cerise Bouquet", "UNIT_DOLL": "DOLLCHESTRA", "UNIT_HASU": "Cerise Bouquet", "UNIT_LILYWHITE": "lily white", "UNIT_MIRAKURA": "Hasunosora", "UNIT_PRINTEMPS": "Printemps", "UNTIL": "Until", "WAIT": "Resting", "YELL_COUNT": "Yell Count", "YELL_REVEALED": "Yell Revealed", "ZONE": "Zone",
            "REMOVE_SELF": "Retire Self",
            "CHARGE_SELF": "Charge Self", "DISCARD_SELF": "Discard Self", "LOOK_AND_CHOOSE_REVEAL": "Look & Reveal", "SELECT_LIVE_CARD": "Select Live", "SELECT_REVEALED": "Select Revealed",
            "ADD_HAND": "Add to Hand", "MOVE_TO_HAND": "Move to Hand", "MOVE_DECK": "Move to Deck", "MOVE_DISCARD": "Move to Discard",
            "COUNT_SUCCESS_LIVES": "Success Lives {value}+", "REDUCE_LIMIT": "Reduce Limit", "SUCCESS_LIVES_CONTAINS": "Success Lives Contains", "REVEALED_CONTAINS": "Revealed Contains",
            "MATCH_RECOVERED": "Match Recovered", "IS_MAIN_PHASE": "Main Phase",
            "ON_STAGE_ENTRY": "On Entry", "ON_MEMBER_DISCARD": "On Member Discard", "ON_OPPONENT_TAP": "On Opponent Wait"
        },
        params: { "UNTIL": { "live_end": "til end of live", "turn_end": "til end" }, "COLOR": { "PINK": "Pink", "RED": "Red", "YELLOW": "Yellow", "GREEN": "Green", "BLUE": "Blue", "PURPLE": "Purple", "ALL": "All colors" }, "GROUP": "{v}", "COST_MAX": "Cost <= {v}", "COUNT": "x{v}", "KEYWORD": "{v}", "ZONE": "{v}", "FILTER": { "energy": "Energy", "member": "Member" }, "AREA": { "0": "Left Side", "1": "Center", "2": "Right Side", "LEFT_SIDE": "Left Side", "CENTER": "Center", "RIGHT_SIDE": "Right Side" }, "TO": { "bottom": "bottom of Deck", "top": "top of Deck" }, "ZONES": { "REVEALED": "REVEALED", "DISCARD": "Discard", "HAND": "Hand", "ENERGY": "Energy Zone" }, "GROUPS": { "虹ヶ咲": "Nijigasaki", "リエラ": "Liella!", "μ's": "μ's", "Aqours": "Aqours", "蓮ノ空": "Hasunosora" } },
        misc: { "optional": "(Opt)", "cost_prefix": "", "effect_prefix": "→ ", "condition_prefix": "Cond: ", "once_per_turn": "(Once per turn)" }
    }
};

function parseParams(paramStr) {
    if (!paramStr) return {};
    const params = {}; let current = ""; let insideArray = 0;
    for (let i = 0; i < paramStr.length; i++) {
        const char = paramStr[i];
        if (char === '[') insideArray++; else if (char === ']') insideArray--;
        if (char === ',' && insideArray === 0) {
            const parts = current.split('=');
            if (parts.length >= 2) params[parts[0].trim()] = parts[1].trim();
            current = "";
        } else current += char;
    }
    if (current) { const parts = current.split('='); if (parts.length >= 2) params[parts[0].trim()] = parts[1].trim(); }
    return params;
}

function translatePart(part, t, lang, allParams, consumedKeys) {
    if (!part) return "";
    const actionMatch = part.match(/^(NOT\s+)?(\w+)\((.*?)\)/);
    const targetMatch = part.match(/^(NOT\s+)?(\w+)(?:\s*\{(.*?)\})?/);
    let op = "", val = "", targetOp = "", isNegated = false;

    if (actionMatch) {
        isNegated = !!actionMatch[1];
        op = actionMatch[2];
        val = actionMatch[3];
    } else if (targetMatch) {
        isNegated = !!targetMatch[1];
        targetOp = targetMatch[2];
    }

    const opcode = op || targetOp;
    if (opcode === 'AREA_CHECK' || opcode === 'CHECK_AREA_CHECK' || opcode === 'TARGET_AREA') {
        if (t.params.AREA && typeof t.params.AREA === 'object' && t.params.AREA[val]) {
            val = t.params.AREA[val];
        }
    }
    const transTemplate = (opcode && (EffectType[opcode] !== undefined && t.opcodes[EffectType[opcode]]) ? t.opcodes[EffectType[opcode]] : t.opcodes[opcode]);

    if (!transTemplate) return isNegated ? "NOT " + opcode : opcode;

    let translated = transTemplate.replace('{value}', val);

    if (isNegated) {
        if (lang === 'jp') {
            translated = translated + (translated.endsWith('場合') ? "でないなら" : "以外");
        } else {
            translated = "NOT " + translated;
        }
    }

    let targetNames = ""; const pStrings = [];
    let colorVal = null;

    for (const [k, vRaw] of Object.entries(allParams)) {
        const v = vRaw.replace(/['"]/g, '');
        const placeholder = `{${k.toLowerCase()}}`;
        let replacement = null;
        if (k === 'NAME') replacement = (lang === 'en') ? (NAME_MAP[v] || v) : v;
        else if (k === 'NAMES') {
            const namesList = v.replace(/[\[\]]/g, '').split(',').map(n => n.trim().replace(/['"]/g, ''));
            const translatedNames = (lang === 'en') ? namesList.map(n => NAME_MAP[n] || n) : namesList;
            replacement = (lang === 'en' ? translatedNames.join('/') : translatedNames.join(', '));
        } else if (t.params[k]) {
            const pTrans = t.params[k];
            replacement = (t.params.GROUPS && t.params.GROUPS[v]) ? t.params.GROUPS[v] : (typeof pTrans === 'object' ? (pTrans[v] || v) : pTrans.replace('{v}', v));
            if (k === 'ZONE' && t.params.ZONES && t.params.ZONES[v]) replacement = t.params.ZONES[v];
        }

        if (replacement !== null) {
            if (translated.includes(placeholder)) {
                translated = translated.replace(placeholder, replacement);
                consumedKeys.add(k);
            } else if (k === 'KEYWORD' && translated.includes('{keyword}')) {
                translated = translated.replace('{keyword}', replacement);
                consumedKeys.add(k);
            } else if (k === 'GROUP' && translated.includes('{group}')) {
                translated = translated.replace('{group}', replacement);
                consumedKeys.add(k);
            } else if (k === 'ZONE' && translated.includes('{zone}')) {
                translated = translated.replace('{zone}', replacement);
                consumedKeys.add(k);
            } else if (k === 'NAME' || k === 'NAMES') {
                targetNames = replacement;
            } else if (!consumedKeys.has(k)) {
                pStrings.push(replacement);
                consumedKeys.add(k);
            }
        }
    }

    if (colorVal && (opcode === 'HEARTS' || opcode === 'ADD_HEARTS' || opcode === 'SET_HEARTS' || opcode === 'PAY_ENERGY' || opcode === 'ENERGY')) {
        const cName = t.params.COLOR[colorVal] || colorVal;
        const iconTag = (opcode.includes('HEART')) ? `【${cName} Hearts】` : `【${cName} Energy】`;
        if (lang === 'jp') {
            const jpCName = Translations.jp.params.COLOR[colorVal] || colorVal;
            const jpIconTag = (opcode.includes('HEART')) ? `【${jpCName}ハート】` : `【${jpCName}エネ】`;
            translated = translated.replace(/【ハート】|ハート|【エネ】|エネ/, (lang === 'jp' ? jpIconTag : iconTag));
        } else {
            translated = translated.replace(/【Hearts】|Hearts|【Energy】|Energy/, iconTag);
        }
    }

    if (lang === 'en') translated = translated.replace('{filter}', "").replace('{to}', "the Deck");
    if (opcode === 'CARD_DISCARD') {
        const discardText = (lang === 'en' ? "Discard" : "控え室");
        translated = targetNames ? `${discardText} (${targetNames})` : discardText;
    } else {
        if (translated.includes('{name}')) translated = translated.replace('{name}', targetNames || "");
        else if (targetNames) translated += ` (${targetNames})`;
        if (pStrings.length > 0) translated += ` [${pStrings.join(', ')}]`;
    }
    return translated.trim().replace(/\s+/g, ' ');
}


function translateHeuristic(text) {
    if (!text) return "";
    let t = text;

    // Common Phrases
    t = t.replace(/自分のデッキの上からカードを(\d+)枚見る/g, "Look at top $1 card(s) of Deck");
    t = t.replace(/その中から(.+?)を(\d+)枚公開して手札に加えてもよい/g, "You may add $2 $1 from them to hand");
    t = t.replace(/残りを控え室に置く/g, "Discard the rest");
    t = t.replace(/ドローする/g, "Draw a card");
    t = t.replace(/カードを(\d+)枚引く/g, "Draw $1 card(s)");
    t = t.replace(/自分の手札を(\d+)枚控え室に置く/g, "Discard $1 card(s) from hand");
    t = t.replace(/手札を(\d+)枚控え室に置いてもよい/g, "You may discard $1 card from hand");
    t = t.replace(/このメンバーをステージから控え室に置く/g, "Retire this Member");
    t = t.replace(/自分の控え室から(.+?)を(\d+)枚手札に加える/g, "Return $2 $1 from Discard to Hand");
    t = t.replace(/ライブ/g, "Live");
    t = t.replace(/メンバー/g, "Member");
    t = t.replace(/カード/g, "Card");
    t = t.replace(/コスト/g, "Cost");
    t = t.replace(/以下/g, " or less");
    t = t.replace(/以上/g, " or more");
    t = t.replace(/の/g, " ");

    return t;
}

function translateAbility(rawText, lang = 'jp') {
    if (!rawText) return "";
    const t = Translations[lang] || Translations.en;
    let translatedLines = [];
    for (let line of rawText.split('\n')) {
        line = line.trim();
        if (!line || line.startsWith('//')) { translatedLines.push(line); continue; }
        if (line.startsWith('TRIGGER:')) {
            const triggerKey = line.replace('TRIGGER:', '').trim();
            const id = TriggerType[triggerKey];
            if (id !== undefined && t.triggers[id]) {
                translatedLines.push(t.triggers[id]);
            } else {
                const displayLabel = triggerKey.toLowerCase()
                    .split('_')
                    .map(word => word.charAt(0).toUpperCase() + word.slice(1))
                    .join(' ');
                translatedLines.push(lang === 'jp' ? `【${triggerKey}】` : `[${displayLabel}]`);
            }
            continue;
        }

        // --- Heuristic Check for Raw Japanese ---
        // If it contains Japanese characters and NO known pseudocode markers, try heuristic logic
        if (lang === 'en' && /[亜-熙ぁ-んァ-ヶ]/.test(line) && !line.includes('EFFECT:') && !line.includes('CONDITION:') && !line.includes('COST:')) {
            translatedLines.push(translateHeuristic(line));
            continue;
        }
        // ---------------------------------------

        let prefix = "", body = line, isPseudo = false;

        const upperLine = line.toUpperCase();
        if (upperLine.startsWith('CONDITION:')) { prefix = t.misc.condition_prefix; body = line.substr(10).trim(); isPseudo = true; }
        else if (upperLine.startsWith('COST:')) { prefix = t.misc.cost_prefix; body = line.substr(5).trim(); isPseudo = true; }
        else if (upperLine.startsWith('EFFECT:')) { prefix = t.misc.effect_prefix; body = line.substr(7).trim(); isPseudo = true; }
        else if (line.match(/^\d+:/)) { const m = line.match(/^\d+:/); prefix = m[0] + " "; body = line.replace(m[0], '').trim(); isPseudo = true; }
        else if (line.includes('->') || Object.keys(EffectType).some(op => line.includes(op + '('))) {
            isPseudo = true;
        }

        if (!isPseudo) {
            translatedLines.push(line);
            continue;
        }

        const isOnce = body.toLowerCase().includes('(once per turn)');
        if (isOnce) body = body.replace(/\(once per turn\)/i, '').trim();
        const isOpt = body.toLowerCase().includes('(optional)');
        if (isOpt) body = body.replace(/\(optional\)/i, '').trim();

        const translatedBody = body.split(';').map(sub => {
            const parts = sub.split('->').map(s => s.trim());
            const consumedKeys = new Set();
            if (parts.length > 1) {
                const trgtMatch = parts[1].match(/^(\w+)(?:\s*\{(.*?)\})?/);
                const allParams = (trgtMatch && trgtMatch[2]) ? parseParams(trgtMatch[2]) : {};
                const actOp = parts[0].split('(')[0];
                const joiner = (lang === 'en' ? ((actOp.startsWith('RECOVER') || actOp === 'MOVE_TO_DECK') ? " from " : " to ") : " ");
                const actionPart = translatePart(parts[0], t, lang, allParams, consumedKeys);
                const targetPart = translatePart(parts[1], t, lang, allParams, consumedKeys);
                return actionPart + joiner + targetPart;
            }
            return translatePart(parts[0], t, lang, {}, consumedKeys);
        }).join('; ');

        let result = prefix + translatedBody;
        if (isOnce) result = t.misc.once_per_turn + "\n" + result;
        if (isOpt) result += t.misc.optional;
        translatedLines.push(result);
    }
    return translatedLines.join('\n');
}

if (typeof window !== 'undefined') {
    window.translateAbility = translateAbility;
    window.Translations = Translations;
    window.NAME_MAP = NAME_MAP;
    window.COMMON_NAMES = COMMON_NAMES;
}
if (typeof module !== 'undefined' && module.exports) module.exports = { translateAbility };

/**
 * UNUSED/UNSUPPORTED OPCODES REFERENCE
 * These opcodes are defined in metadata but not yet implemented in this translator:
 * - NOP (0): No operation
 * - RETURN (1): Control flow
 * - JUMP (2): Control flow
 * - SWAP_ZONE (38): Generic movement (implemented via specifically named movements usually)
 * - TRANSFORM_COLOR (39): Rarely used in text-only cards
 * - TRANSFORM_HEART (73): Rarely used in text-only cards
 * - META_RULE (29): Complex rules usually have custom friendly text
 */

// Export for Node.js tests if applicable
if (typeof module !== 'undefined' && module.exports) {
    // This is handled by ESM export below, but kept for legacy Node if needed
    // However, since we use 'import' at the top, this file IS an ESM.
}

export { translateAbility, NAME_MAP };
