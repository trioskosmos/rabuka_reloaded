/**
 * Character Names Mapping
 * Maps Japanese character names to English equivalents.
 * Extracted from ability_translator.js for maintainability.
 */

export const COMMON_NAMES = {
    // μ's (Love Live! School Idol Project)
    "高坂 穂乃果": "Honoka Kosaka",
    "絢瀬 絵里": "Eli Ayase",
    "南 ことり": "Kotori Minami",
    "園田 海未": "Umi Sonoda",
    "星空 凛": "Rin Hoshizora",
    "西木野 真姫": "Maki Nishikino",
    "東條 希": "Nozomi Tojo",
    "小泉 花陽": "Hanayo Koizumi",
    "矢澤 にこ": "Nico Yazawa",

    // Aqours (Love Live! Sunshine!!)
    "高海 千歌": "Chika Takami",
    "桜内 梨子": "Riko Sakurauchi",
    "松浦 果南": "Kanan Matsuura",
    "黒澤 ダイヤ": "Dia Kurosawa",
    "渡辺 曜": "You Watanabe",
    "津島 善子": "Yoshiko Tsushima",
    "国木田 花丸": "Hanamaru Kunikida",
    "小原 鞠莉": "Mari Ohara",
    "黒澤 ルビィ": "Ruby Kurosawa",

    // Nijigasaki (Love Live! Nijigasaki High School Idol Club)
    "上原 歩夢": "Ayumu Uehara",
    "中須 かすみ": "Kasumi Nakasu",
    "桜坂 しずく": "Shizuku Osaka",
    "朝香 果林": "Karin Asaka",
    "宮下 愛": "Ai Miyashita",
    "近江 彼方": "Kanata Konoe",
    "優木 せつ菜": "Setsuna Yuki",
    "エマ・ヴェルデ": "Emma Verde",
    "天王寺 璃奈": "Rina Tennoji",
    "三船 栞子": "Shioriko Mifune",
    "ミア・テイラー": "Mia Taylor",
    "鐘 嵐珠": "Lanzhu Zhong",

    // Liella! (Love Live! Superstar!!)
    "澁谷 かのん": "Kanon Shibuya",
    "唐 可可": "Keke Tang",
    "嵐 千砂都": "Chisato Arashi",
    "平安名 すみれ": "Sumire Heanna",
    "葉月 恋": "Ren Hazuki",
    "桜小路 きな子": "Kinako Sakurakoji",
    "米女 メイ": "Mei Yoneme",
    "若菜 四季": "Shiki Wakana",
    "鬼塚 夏美": "Natsumi Onitsuka",
    "ウィーン・マルガレーテ": "Wien Margarete",
    "鬼塚 冬毬": "Tomari Onitsuka",

    // Hasunosora (Love Live! Hasunosora Girls' High School Idol Club)
    "日野下 花帆": "Kaho Hinoshita",
    "乙宗 梢": "Kozue Otomune",
    "村野 さやか": "Sayaka Murano",
    "夕霧 綴理": "Tsuzuri Yugiri",
    "大沢 瑠璃乃": "Rurino Osawa",
    "藤島 慈": "Megumi Fujishima",
    "徒町 小鈴": "Kosuzu Kachimachi",
    "百生 吟子": "Ginko Momose",
    "安養寺 姫芽": "Hime Anyoji",

    // Additional characters from new cards
    "聖澤 悠奈": "Yuna Sezegawa",
    "桂城 泉": "Izumi Katsuragi",
    "柊 摩央": "Mao Hiiragi",
    "統堂 英玲奈": "Erena Todo",
    "綺羅 ツバサ": "Tsubasa Kira",
    "鹿角 理亞": "Ria Kazuno",
    "鹿角 聖良": "Seira Kazuno",
    "高咲 侑": "Yu Takasaki",
    "優木 あんじゅ": "Anju Yuki",

    // Multi-character cards
    "上原 歩夢&澁谷 かのん&日野下 花帆": "Ayumu Uehara & Kanon Shibuya & Kaho Hinoshita",
    "園田 海未&津島 善子&天王寺 璃奈": "Umi Sonoda & Yoshiko Tsushima & Rina Tennoji",

    // Character variants without spaces
    "三船栞子": "Shioriko Mifune",
    "天王寺璃奈": "Rina Tennoji",
    "セラス柳田リリエンフェルト": "Ceras Yanagita Lilienfeld",

    // Generic card names and special types
    "エネルギーカード": "Energy Card",
    "ライブカード": "Live Card",
    "メンバーカード": "Member Card",
    "ライブカードゴールド": "Live Card Gold",
    "ライブカード プレシャス": "Live Card Precious",
    "ライブカード＆エネルギーカード": "Live Card & Energy Card",
    "謎のメンバー": "Mystery Member",
    "???": "Unknown",
    "???メンバー": "Secret Member"
};

/**
 * NAME_MAP - Extended mapping including space-removed variants
 * Auto-generated from COMMON_NAMES for flexible matching
 */
export const NAME_MAP = {};

// Build NAME_MAP with both original and space-removed variants
for (const [jp, en] of Object.entries(COMMON_NAMES)) {
    NAME_MAP[jp] = en;
    NAME_MAP[jp.replace(/\s/g, '')] = en;
}

// Make NAME_MAP available globally for backward compatibility
if (typeof window !== 'undefined') {
    window.NAME_MAP = NAME_MAP;
    window.COMMON_NAMES = COMMON_NAMES;
}
