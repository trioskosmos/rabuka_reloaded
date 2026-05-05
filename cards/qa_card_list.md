# QA Card List (grouped by ability)

Cards with QA entries, grouped by ability (all variants share the same ability).
Tests MUST validate actual game behavior from `qa_data.json` entries — not just confirm
that JSON fields map to other JSON fields. Each test should prove a real game rule:
engine evaluates conditions correctly, filters work, edge cases are handled, etc.
Write tests in engine\tests\gameplay_test_process.md

| # | QA Count | Card | QA IDs | Status |
|---|----------|------|--------|--------|
|1|5|LL-bp1-001-R＋ (上原歩夢&澁谷かのん&日野下花帆 (ab#1))|Q62, Q65, Q69, Q89, Q90|✓ ayumu_azuna_test|
|2|4|LL-bp2-001-R＋ (渡辺 曜&鬼塚夏美&大沢瑠璃乃 (ab#2))|Q129, Q186, Q62, Q89|✓ ll_joint_test|
|3|4|PL!-bp3-026-L (Oh,Love&Peace! (ab#1))|Q149, Q150, Q172, Q36|✓ lovepeace (gameplay_test)|
|4|4|PL!-pb1-018-R (矢澤にこ (ab#0))|Q168, Q169, Q170, Q181|✓ nico (gameplay_test)|
| 5 | 4 | PL!N-bp1-002-R＋ (中須かすみ (ab#1)) | Q122, Q63, Q75, Q76 | | ✓ kasumi_test |
|6|4|PL!N-bp3-027-L (La Bella Patria (ab#0))|Q142, Q173, Q174, Q36|✓ bella (gameplay_test)|
|7|4|PL!SP-bp1-026-L (未来予報ハレルヤ！ (ab#0))|Q105, Q127, Q64, Q74|✓ hareruya (gameplay_test)|
|8|4|PL!SP-bp2-010-R＋ (ウィーン・マルガレーテ (ab#1))|Q110, Q111, Q117, Q127|✓ wien (gameplay_test)|
|9|4|PL!SP-pb1-001-R (澁谷かのん (ab#1))|Q36, Q91, Q92, Q93|✓ kanon_test|
|10|3|LL-bp3-001-R＋ (園田海未&津島善子&天王寺璃奈 (ab#1))|Q165, Q62, Q89|✓ ll_joint_test|
|11|3|PL!N-bp3-001-R＋ (上原歩夢 (ab#0))|Q157, Q158, Q184|✓ ayumu_azuna_test|
|12|3|PL!N-bp3-005-R＋ (宮下 愛 (ab#1))|Q160, Q161, Q162|✓ miyashita_ai_test|
|13|3|PL!S-bp3-001-R＋ (高海千歌 (ab#0))|Q151, Q152, Q171|✓ chika_test|
| 14 | 3 | PL!S-pb1-021-L (Strawberry Trapper (ab#0)) | Q132, Q142, Q36 | ✓ strawberry_trapper_test |
|15|3|PL!SP-bp1-003-R＋ (嵐 千砂都 (ab#0))|Q129, Q171, Q78|✓ chisato_test|
|16|3|PL!SP-bp2-024-L (ビタミンSUMMER！ (ab#0))|Q119, Q128, Q36|✓ vitamin_test|
|17|3|PL!SP-pb1-023-L (ディストーション (ab#0))|Q103, Q96, Q97|✓ distortion (gameplay_test)|
| 18 | 2 | PL!-bp3-025-L (タカラモノズ (ab#0)) | Q142, Q36 | ✓ takaramono_test |
| 19 | 2 | PL!-bp5-003-R＋ (南 ことり (ab#1)) | Q207, Q208 | |
|20|2|PL!-bp5-021-L (SUNNY DAY SONG (ab#0))|Q210, Q211|✓ sunny_test|
| 21 | 2 | PL!-pb1-001-R (高坂穂乃果 (ab#0)) | Q166, Q167 | |
| 22 | 2 | PL!-pb1-028-L (WAO-WAO Powerful day! (ab#0)) | Q178, Q179 | ✓ wao_wao_test |
| 23 | 2 | PL!N-bp3-030-L (Love U my friends (ab#0)) | Q192, Q36 | ✓ love_u_test |
| 24 | 2 | PL!-sd1-005-SD (星空 凛 (ab#0)) | Q123, Q79 | ✓ rin_test |
| 25 | 2 | PL!HS-bp1-002-R (村野さやか (ab#0)) | Q63, Q80 | ✓ sayaka_test |
| 26 | 2 | PL!HS-bp1-022-L (AWOKE (ab#0)) | Q107, Q36 | |
| 27 | 2 | PL!N-bp1-011-R (ミア・テイラー (ab#0)) | Q102, Q73 | ✓ mia_test |
| 28 | 2 | PL!N-bp1-026-L (Poppin' Up! (ab#0)) | Q36, Q66 | ✓ poppin_test |
| 29 | 2 | PL!N-bp3-007-R (優木せつ菜 (ab#0)) | Q157, Q184 | ✓ setsuna_test |
| 30 | 2 | PL!N-bp3-013-N (上原歩夢 (ab#0)) | Q157, Q184 | | ✓ (same ability as azuna)
| 31 | 2 | PL!N-bp5-026-L (TOKIMEKI Runners (ab#1)) | Q216, Q232 | |
| 32 | 2 | PL!N-bp5-027-L (ミラクル STAY TUNE！ (ab#0)) | Q207, Q208 | |
| 33 | 2 | PL!N-bp5-030-L (繚乱！ビクトリーロード (ab#1)) | Q217, Q227 | |
| 34 | 2 | PL!N-pb1-013-R (上原歩夢 (ab#0)) | Q199, Q200 | ✓ ayumu_pb1_test |
| 35 | 2 | PL!N-pb1-017-R (宮下 愛 (ab#0)) | Q199, Q201 | ✓ ayumu_pb1_test |
| 36 | 2 | PL!N-pb1-023-R (ミア・テイラー (ab#0)) | Q199, Q202 | ✓ ayumu_pb1_test |
| 37 | 2 | PL!S-bp2-024-L (君のこころは輝いてるかい？ (ab#0)) | Q125, Q36 | ✓ kagayaiteru_test |
| 38 | 2 | PL!S-bp3-019-L (MIRACLE WAVE (ab#0)) | Q182, Q36 | |
| 39 | 2 | PL!S-pb1-002-R (桜内梨子 (ab#0)) | Q130, Q171 | |
| 40 | 2 | PL!SP-bp1-023-L (START!! True dreams (ab#0)) | Q36, Q66 | ✓ start_true_dreams_test |
| 41 | 2 | PL!SP-bp2-001-R＋ (澁谷かのん (ab#0)) | Q106, Q171 | |
| 42 | 2 | PL!SP-bp2-009-R＋ (鬼塚夏美 (ab#0)) | Q109, Q36 | ✓ natsumi_test |
| 43 | 2 | PL!SP-bp2-015-N (平安名すみれ (ab#0)) | Q112, Q113 | ✓ sumire_auto_test |
| 44 | 2 | PL!SP-bp2-020-N (鬼塚夏美 (ab#0)) | Q112, Q113 | ✓ sumire_auto_test |
|46|2|PL!SP-bp2-021-N (ウィーン・マルガレーテ (ab#0))|Q112, Q113|
|47|2|PL!SP-bp4-004-R＋ (平安名すみれ (ab#1))|Q193, Q194|
|48|2|PL!SP-bp4-023-L (Dazzling Game (ab#1))|Q187, Q192|
|49|2|PL!SP-bp5-005-R＋ (葉月 恋 (ab#1))|Q221, Q233|
|50|2|PL!SP-pb1-006-R (桜小路きな子 (ab#0))|Q171, Q94|
|51|2|PL!SP-pb1-011-R (鬼塚冬毬 (ab#0))|Q63, Q95|
|52|2|PL!SP-pb1-025-L (Jellyfish (ab#0))|Q98, Q99|✓ jellyfish_test|
| 53 | 1 | LL-PR-004-PR (愛♡スクリ～ム！ (ab#0)) | Q185 | |
| 54 | 1 | LL-bp5-001-L (Live with a smile! (ab#0)) | Q224 | |
| 55 | 1 | LL-bp5-002-L (Bring the LOVE！ (ab#1)) | Q225 | |
| 56 | 1 | PL!-bp3-002-R (絢瀬絵里 (ab#1)) | Q144 | |
| 57 | 1 | PL!-bp3-003-R (南ことり (ab#0)) | Q145 | |
| 58 | 1 | PL!-bp3-004-R＋ (園田海未 (ab#1)) | Q146 | |
| 59 | 1 | PL!-bp3-008-R＋ (小泉花陽 (ab#1)) | Q145 | |
| 60 | 1 | PL!-bp3-019-L (僕らのLIVE 君とのLIFE (ab#0)) | Q147 | |
| 61 | 1 | PL!-bp3-023-L (ミはμ'sicのミ (ab#0)) | Q148 | |
| 62 | 1 | PL!-bp4-009-R (矢澤にこ (ab#0)) | Q189 | |
| 63 | 1 | PL!-bp5-004-R＋ (園田海未 (ab#1)) | Q228 | |
| 64 | 1 | PL!-bp5-007-R (東條 希 (ab#0)) | Q229 | |
| 65 | 1 | PL!-bp5-009-R (矢澤にこ (ab#0)) | Q209 | |
| 66 | 1 | PL!-pb1-008-R (小泉花陽 (ab#0)) | Q183 | ✓ hanayo_test |
| 67 | 1 | PL!-pb1-009-R (矢澤にこ (ab#1)) | Q180 | ✓ batch3_test | |
| 68 | 1 | PL!-pb1-013-R (園田海未 (ab#0)) | Q176 | ✓ batch3_test | |
| 69 | 1 | PL!-pb1-015-R (西木野真姫 (ab#1)) | Q177 | |
| 70 | 1 | PL!-pb1-030-L (Cutie Panther (ab#1)) | Q36 | |
| 71 | 1 | PL!-pb1-031-L (輝夜の城で踊りたい (ab#0)) | Q36 | |
| 72 | 1 | PL!-pb1-032-L (SENTIMENTAL StepS (ab#0)) | Q36 | |
| 73 | 1 | PL!-sd1-002-SD (絢瀬 絵里 (ab#0)) | Q79 | |
| 74 | 1 | PL!-sd1-006-SD (西木野 真姫 (ab#0)) | Q125 | |
| 75 | 1 | PL!-sd1-019-SD (START:DASH!! (ab#0)) | Q36 | |
| 76 | 1 | PL!HS-PR-016-PR (日野下花帆 (ab#0)) | Q175 | |
| 77 | 1 | PL!HS-PR-017-PR (村野さやか (ab#0)) | Q175 | |
| 78 | 1 | PL!HS-PR-019-PR (百生 吟子 (ab#0)) | Q171 | |
| 79 | 1 | PL!HS-PR-021-PR (安養寺 姫芽 (ab#0)) | Q171 | |
| 80 | 1 | PL!HS-bp1-003-R＋ (乙宗 梢 (ab#1)) | Q81 | |
| 81 | 1 | PL!HS-bp1-004-R＋ (夕霧綴理 (ab#1)) | Q38 | |
| 82 | 1 | PL!HS-bp1-009-R (安養寺 姫芽 (ab#0)) | Q82 | |
| 83 | 1 | PL!HS-bp1-021-L (Holiday∞Holiday (ab#0)) | Q36 | |
| 84 | 1 | PL!HS-bp1-023-L (ド！ド！ド！ (ab#0)) | Q36 | |
| 85 | 1 | PL!HS-bp2-008-R (徒町 小鈴 (ab#0)) | Q171 | |
| 86 | 1 | PL!HS-bp2-009-R (安養寺 姫芽 (ab#0)) | Q171 | |
| 87 | 1 | PL!HS-bp2-014-N (大沢瑠璃乃 (ab#0)) | Q68 | |
| 88 | 1 | PL!HS-bp2-019-L (Bloom the smile, Bloom the dream! (ab#0)) | Q127 | |
| 89 | 1 | PL!HS-bp2-024-L (レディバグ (ab#0)) | Q114 | |
| 90 | 1 | PL!HS-bp5-007-R (セラス 柳田 リリエンフェルト (ab#1)) | Q209 | |
| 91 | 1 | PL!HS-bp5-017-L (Dream Believers（104期Ver.） (ab#0)) | Q212 | |
| 92 | 1 | PL!HS-bp5-019-L (ハナムスビ (ab#0)) | Q213 | |
| 93 | 1 | PL!N-bp1-006-R＋ (近江彼方 (ab#0)) | Q77 | | ✓ konata_test |
| 94 | 1 | PL!N-bp1-012-R＋ (鐘 嵐珠 (ab#0)) | Q38 | |
| 95 | 1 | PL!N-bp1-027-L (Solitude Rain (ab#0)) | Q67 | |
| 96 | 1 | PL!N-bp1-029-L (Eutopia (ab#0)) | Q38 | |
| 97 | 1 | PL!N-bp3-003-R (桜坂しずく (ab#0)) | Q159 | |
| 98 | 1 | PL!N-bp3-008-R＋ (エマ・ヴェルデ (ab#1)) | Q163 | |
| 99 | 1 | PL!N-bp3-009-R＋ (天王寺璃奈 (ab#0)) | Q164 | |
| 100 | 1 | PL!N-bp3-011-R (ミア・テイラー (ab#0)) | Q171 | |
| 101 | 1 | PL!N-bp3-031-L (MONSTER GIRLS (ab#0)) | Q36 | |
| 102 | 1 | PL!N-bp4-011-R＋ (ミア・テイラー (ab#1)) | Q190 | |
| 103 | 1 | PL!N-bp4-018-N (近江彼方 (ab#0)) | Q188 | |
| 104 | 1 | PL!N-bp4-025-L (VIVID WORLD (ab#1)) | Q192 | |
| 105 | 1 | PL!N-bp4-030-L (Daydream Mermaid (ab#0)) | Q191 | |
| 106 | 1 | PL!N-bp5-003-R (桜坂しずく (ab#0)) | Q214 | |
| 107 | 1 | PL!N-bp5-007-R＋ (優木せつ菜 (ab#1)) | Q230 | |
| 108 | 1 | PL!N-bp5-008-R (エマ・ヴェルデ (ab#0)) | Q215 | |
| 109 | 1 | PL!N-bp5-010-R (三船栞子 (ab#0)) | Q231 | |
| 110 | 1 | PL!N-bp5-015-N (桜坂しずく (ab#0)) | Q216 | |
| 111 | 1 | PL!N-bp5-021-N (天王寺璃奈 (ab#0)) | Q226 | |
| 112 | 1 | PL!N-pb1-003-R (桜坂しずく (ab#0)) | Q196 | |
| 113 | 1 | PL!N-pb1-005-R (宮下 愛 (ab#0)) | Q197 | |
| 114 | 1 | PL!N-pb1-007-R (優木せつ菜 (ab#0)) | Q205 | |
| 115 | 1 | PL!N-pb1-008-R (エマ・ヴェルデ (ab#1)) | Q206 | |
| 116 | 1 | PL!N-pb1-012-R (鐘 嵐珠 (ab#1)) | Q198 | |
| 117 | 1 | PL!N-pb1-015-R (桜坂しずく (ab#0)) | Q199 | |
| 118 | 1 | PL!N-pb1-037-L (Cara Tesoro (ab#0)) | Q203 | |
| 119 | 1 | PL!N-pb1-042-L (Eternalize Love!! (ab#0)) | Q204 | |
| 120 | 1 | PL!N-sd1-009-SD (天王寺璃奈 (ab#0)) | Q209 | |
| 121 | 1 | PL!N-sd1-028-SD (Dream with You (ab#0)) | Q116 | |
| 122 | 1 | PL!S-PR-016-PR (黒澤ダイヤ (ab#0)) | Q171 | |
| 123 | 1 | PL!S-bp2-004-R (黒澤ダイヤ (ab#0)) | Q107 | ✓ batch3_test | |
| 124 | 1 | PL!S-bp2-005-R＋ (渡辺 曜 (ab#0)) | Q124 | |
| 125 | 1 | PL!S-bp2-007-R＋ (国木田花丸 (ab#1)) | Q120 | |
| 126 | 1 | PL!S-bp2-008-R＋ (小原鞠莉 (ab#1)) | Q36 | |
| 127 | 1 | PL!S-bp2-021-L (未体験HORIZON (ab#0)) | Q36 | |
| 128 | 1 | PL!S-bp2-022-L (未熟DREAMER (ab#0)) | Q36 | |
| 129 | 1 | PL!S-bp2-023-L (MY舞☆TONIGHT (ab#0)) | Q121 | |
| 130 | 1 | PL!S-bp3-005-R (渡辺 曜 (ab#0)) | Q153 | |
| 131 | 1 | PL!S-bp3-006-R＋ (津島善子 (ab#0)) | Q154 | |
| 132 | 1 | PL!S-bp3-008-R (小原鞠莉 (ab#0)) | Q79 | |
| 133 | 1 | PL!S-bp3-016-N (国木田花丸 (ab#0)) | Q155 | |
| 134 | 1 | PL!S-bp3-020-L (ダイスキだったらダイジョウブ！ (ab#0)) | Q156 | |
| 135 | 1 | PL!S-bp5-001-R＋ (高海千歌 (ab#1)) | Q218 | |
| 136 | 1 | PL!S-pb1-003-R (松浦果南 (ab#0)) | Q36 | |
| 137 | 1 | PL!S-pb1-006-R (津島善子 (ab#0)) | Q171 | |
| 138 | 1 | PL!S-pb1-007-R (国木田花丸 (ab#0)) | Q36 | |
| 139 | 1 | PL!S-pb1-008-R (小原鞠莉 (ab#0)) | Q131 | |
| 140 | 1 | PL!S-pb1-019-L (元気全開DAY！DAY！DAY！ (ab#0)) | Q36 | |
| 141 | 1 | PL!S-pb1-022-L (逃走迷走メビウスループ (ab#0)) | Q36 | |
| 142 | 1 | PL!S-pb1-024-L (僕らの走ってきた道は・・・ (ab#0)) | Q36 | |
| 143 | 1 | PL!SP-bp1-001-R (澁谷かのん (ab#0)) | Q68 | |
| 144 | 1 | PL!SP-bp1-024-L (Tiny Stars (ab#1)) | Q36 | |
| 145 | 1 | PL!SP-bp2-003-R (嵐 千砂都 (ab#0)) | Q126 | |
| 146 | 1 | PL!SP-bp2-006-R＋ (桜小路きな子 (ab#1)) | Q108 | | ✓ kinako_test |
| 147 | 1 | PL!SP-bp2-011-R (鬼塚冬毬 (ab#0)) | Q118 | |
| 148 | 1 | PL!SP-bp2-025-L (Bubble Rise (ab#0)) | Q36 | |
| 149 | 1 | PL!SP-bp4-025-L (Special Color (ab#1)) | Q195 | |
| 150 | 1 | PL!SP-bp5-003-R＋ (嵐 千砂都 (ab#1)) | Q219 | |
| 151 | 1 | PL!SP-bp5-004-R＋ (平安名すみれ (ab#0)) | Q220 | |
| 152 | 1 | PL!SP-bp5-006-R (桜小路きな子 (ab#0)) | Q234 | |
| 153 | 1 | PL!SP-bp5-007-R (米女メイ (ab#0)) | Q235 | |
| 154 | 1 | PL!SP-bp5-009-R (鬼塚夏美 (ab#0)) | Q222 | |
| 155 | 1 | PL!SP-bp5-010-R (ウィーン・マルガレーテ (ab#0)) | Q223 | |
| 156 | 1 | PL!SP-pb1-004-R (平安名すみれ (ab#1)) | Q36 | |
| 157 | 1 | PL!SP-sd1-002-SD (唐 可可 (ab#0)) | Q63 | |
| 158 | 1 | PL!SP-sd1-004-SD (平安名すみれ (ab#0)) | Q171 | |
| 159 | 1 | PL!SP-sd1-026-SD (私のSymphony 〜澁谷かのんVer.〜 (ab#0)) | Q90 | |