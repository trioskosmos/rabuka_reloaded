# Parser Validation Report

Generated: 2026-05-12T19:16:59.634125

Total abilities: 645

## WARNING (59)

- **#23** `any_number_missing`: any_number=True in tree
  - Text: `手札にあるメンバーカードを好きな枚数公開する：公開したカードのコストの合計が、10、20、30、40、50のいずれかの場合、ライブ終了時まで、「{{jyouji`
  - Actual: any_number=set()

- **#56** `max_flag_missing`: max=True in tree
  - Text: `手札を2枚まで控え室に置いてもよい：ライブ終了時まで、これによって控え室に置いたカード1枚につき、{{icon_blade.png|ブレード}}{{icon_b`
  - Actual: max=set()

- **#57** `stage_dest_missing`: destination=stage expected for ステージに置く/登場させる
  - Text: `{{center.png|センター}}このメンバーをウェイトにし、手札を1枚控え室に置く：このメンバー以外の『Aqours』のメンバー1人を自分のステージから控`
  - Actual: dests={'same_area', 'discard'}

- **#77** `optional_flag_missing`: optional=True somewhere in tree
  - Text: `このカードのプレイに際し、2人のメンバーとバトンタッチしてもよい。`
  - Actual: optional=set()

- **#78** `stage_dest_missing`: destination=stage expected for ステージに置く/登場させる
  - Text: `{{center.png|センター}}『Liella!』のメンバー2人からバトンタッチして登場している場合、カードを2枚引き、自分の控え室にあるコスト4以下の『`
  - Actual: dests={'empty_area', 'hand'}

- **#100** `choice_not_parsed`: action=choice expected
  - Text: `{{icon_energy.png|E}}支払ってもよい：以下から1つを選ぶ。
・相手のステージにいるコスト4以下のメンバー1人をウェイトにする。
・カードを1`
  - Actual: actions=set()

- **#110** `multiple_targets_missing`: multiple_targets=True in tree
  - Text: `自分のステージにコストがそれぞれ異なるメンバーが3人以上いるかぎり、{{heart_05.png|heart05}}{{icon_blade.png|ブレード}`
  - Actual: multiple_targets=set()

- **#111** `stage_dest_missing`: destination=stage expected for ステージに置く/登場させる
  - Text: `{{icon_energy.png|E}}{{icon_energy.png|E}}：自分の控え室からコスト2以下のメンバーカードを1枚、メンバーのいないエリア`
  - Actual: dests={'empty_area'}

- **#116** `hand_to_discard_not_found`: source=hand AND dest=discard somewhere in tree
  - Text: `以下から1つを選ぶ。
・カードを1枚引き、手札を1枚控え室に置く。
・相手のステージにいるすべてのコスト2以下のメンバーをウェイトにする。`
  - Actual: sources={'deck'} dests={'hand'}

- **#116** `choice_not_parsed`: action=choice expected
  - Text: `以下から1つを選ぶ。
・カードを1枚引き、手札を1枚控え室に置く。
・相手のステージにいるすべてのコスト2以下のメンバーをウェイトにする。`
  - Actual: actions=set()

- **#119** `target_opponent_missing`: target=opponent in tree
  - Text: `自分か相手のステージにコスト13以上のメンバーがいる場合、{{icon_blade.png|ブレード}}{{icon_blade.png|ブレード}}を得る。`
  - Actual: targets={'either'}

- **#120** `hand_to_discard_not_found`: source=hand AND dest=discard somewhere in tree
  - Text: `手札をすべて公開する：自分のステージにほかのメンバーがおり、かつこれにより公開した手札の中にライブカードがない場合、自分のデッキの上からカードを5枚見る。その中`
  - Actual: sources={'deck_top', 'hand'} dests={'hand'}

- **#136** `max_flag_missing`: max=True in tree
  - Text: `手札のブレードハートを持たないメンバーカードを2枚まで控え室に置いてもよい：自分の控え室から、これにより控え室に置いたカードと同じ枚数の『Aqours』のライブ`
  - Actual: max=set()

- **#137** `choice_not_parsed`: action=choice expected
  - Text: `以下から1つを選ぶ。
・自分のステージにいるこのメンバー以外の『Aqours』のメンバー1人は、ライブ終了時まで、{{icon_blade.png|ブレード}}`
  - Actual: actions=set()

- **#143** `target_both_missing`: target=both in tree
  - Text: `自分と相手のステージの中で、このメンバーがほかのすべてのメンバーより多くのハートを持つかぎり、ライブの合計スコアを＋１する。`
  - Actual: targets=set()

- **#143** `all_flag_missing`: all=True in tree
  - Text: `自分と相手のステージの中で、このメンバーがほかのすべてのメンバーより多くのハートを持つかぎり、ライブの合計スコアを＋１する。`
  - Actual: all=set()

- **#151** `choice_not_parsed`: action=choice expected
  - Text: `以下から1つを選ぶ。
・自分の控え室にカード名が異なるライブカードが3枚以上ある場合、自分の控え室からライブカードを1枚手札に加える。
・自分の控え室にグループ`
  - Actual: actions=set()

- **#154** `multiple_targets_missing`: multiple_targets=True in tree
  - Text: `手札を1枚控え室に置いてもよい：自分のデッキの上からカードを5枚見る。その中から各グループ名につき1枚ずつ公開し、3枚まで手札に加えてもよい。残りを控え室に置く`
  - Actual: multiple_targets=set()

- **#154** `max_flag_missing`: max=True in tree
  - Text: `手札を1枚控え室に置いてもよい：自分のデッキの上からカードを5枚見る。その中から各グループ名につき1枚ずつ公開し、3枚まで手札に加えてもよい。残りを控え室に置く`
  - Actual: max=set()

- **#193** `stage_dest_missing`: destination=stage expected for ステージに置く/登場させる
  - Text: `{{icon_energy.png|E}}{{icon_energy.png|E}}、このメンバーをステージから控え室に置く：自分の控え室からコスト15以下の『`
  - Actual: dests={'same_area', 'discard'}

- **#194** `max_flag_missing`: max=True in tree
  - Text: `手札を3枚まで控え室に置いてもよい：これにより置いた枚数分カードを引く。`
  - Actual: max=set()

- **#206** `target_opponent_missing`: target=opponent in tree
  - Text: `このメンバーが登場か、エリアを移動するたび、ライブ終了時まで、{{icon_blade.png|ブレード}}{{icon_blade.png|ブレード}}を得る`
  - Actual: targets=set()

- **#211** `stage_dest_missing`: destination=stage expected for ステージに置く/登場させる
  - Text: `「鬼塚冬毬」以外の『Liella!』のメンバー1人をステージから控え室に置いてもよい：自分の控え室から、これにより控え室に置いたメンバーカードを1枚、そのメンバ`
  - Actual: dests={'same_area', 'discard'}

- **#234** `multiple_targets_missing`: multiple_targets=True in tree
  - Text: `自分のステージの右サイドエリアに「大沢瑠璃乃」が、左サイドエリアに「安養寺姫芽」が、センターエリアに「藤島慈」がそれぞれ登場している場合、このカードのスコアを＋`
  - Actual: multiple_targets=set()

- **#244** `hand_to_discard_not_found`: source=hand AND dest=discard somewhere in tree
  - Text: `このメンバーをウェイトにする：カードを1枚引き、手札を1枚控え室に置く。（ウェイト状態のメンバーが持つ{{icon_blade.png|ブレード}}は、エールで`
  - Actual: sources={'deck'} dests={'hand'}

- **#262** `stage_dest_missing`: destination=stage expected for ステージに置く/登場させる
  - Text: `{{icon_energy.png|E}}{{icon_energy.png|E}}このメンバーをステージから控え室に置く：自分の手札からコスト13以下の「優木`
  - Actual: dests={'same_area', 'under_member', 'discard'}

- **#264** `multiple_targets_missing`: multiple_targets=True in tree
  - Text: `相手のステージにいる「ミア・テイラー」以外のメンバーを1人選ぶ。そのメンバーが持つハートと、このメンバーが持つハートの中に同じ色のハートがある場合、ライブ終了時`
  - Actual: multiple_targets=set()

- **#274** `discard_source_missing`: source=discard expected for 控え室から手札に加える
  - Text: `手札を3枚控え室に置く：自分のステージにほかの『lilywhite』のメンバーがいる場合、自分の控え室から『μ's』のライブカードを1枚手札に加える。この能力を`
  - Actual: sources={'hand'}

- **#274** `hand_dest_missing`: destination=hand expected for 手札に加える/置く
  - Text: `手札を3枚控え室に置く：自分のステージにほかの『lilywhite』のメンバーがいる場合、自分の控え室から『μ's』のライブカードを1枚手札に加える。この能力を`
  - Actual: dests={'discard'}

- **#275** `max_flag_missing`: max=True in tree
  - Text: `メンバーを3人までウェイトにしてもよい：これによりウェイト状態にしたメンバー1人につき、カードを1枚引く。`
  - Actual: max=set()

- **#304** `hand_to_discard_not_found`: source=hand AND dest=discard somewhere in tree
  - Text: `カードを2枚引き、手札を2枚控え室に置く。（この能力は左サイドエリアか右サイドエリアに登場した場合のみ発動する。）`
  - Actual: sources={'deck'} dests={'hand'}

- **#319** `multiple_targets_missing`: multiple_targets=True in tree
  - Text: `自分のライブ中のライブカードの必要ハートの中に{{heart_01.png|heart01}}、{{heart_02.png|heart02}}、{{heart`
  - Actual: multiple_targets=set()

- **#323** `choice_not_parsed`: action=choice expected
  - Text: `以下から1つを選ぶ。
・エネルギーを1枚アクティブにする。
・自分の控え室にある『虹ヶ咲』のライブカードを2枚まで好きな順番でデッキの上に置く。`
  - Actual: actions=set()

- **#326** `exclude_self_missing`: exclude_self=True in tree
  - Text: `自分のステージにこのメンバー以外のコスト11のメンバーが登場したとき、自分のエネルギーデッキから、エネルギーカードを1枚ウェイト状態で置く。`
  - Actual: exclude_self=set()

- **#357** `deck_top_source_missing`: source=deck_top in tree
  - Text: `数1つを選ぶ。自分のデッキの一番上のカードを公開する。公開したカードがメンバーカードで、かつコストが選んだ数以上の場合、公開したカードを手札に加える。選んだ数以`
  - Actual: sources={'discard'}

- **#390** `optional_flag_missing`: optional=True somewhere in tree
  - Text: `直前のターンに相手がライブをし、それが成功していない場合、相手にエマパンチ打つ？と聞いてもよい。
回答がお願いしますの場合、自分は相手にエマパンチする。ライブ終`
  - Actual: optional=set()

- **#400** `original_value_missing`: original_value=True in tree
  - Text: `自分のステージに、元々持つハートの数より多い数のハートを持つメンバーがいる場合、カードを1枚引く。`
  - Actual: original_value=set()

- **#434** `target_opponent_missing`: target=opponent in tree
  - Text: `このメンバーがエリアを移動するたび、カードを1枚引く。
(対戦相手のカードの効果でも発動する。)`
  - Actual: targets=set()

- **#461** `any_number_missing`: any_number=True in tree
  - Text: `手札の「渡辺曜」と「鬼塚夏美」と「大沢瑠璃乃」を、好きな枚数控え室に置いてもよい：ライブ終了時まで、これによって控え室に置いた枚数1枚につき、{{icon_bl`
  - Actual: any_number=set()

- **#470** `deck_top_source_missing`: source=deck_top in tree
  - Text: `自分のデッキの上から、自分と相手のステージにいるメンバー1人につき、1枚公開する。それらの中にあるライブカード1枚につき、このカードのスコアを＋１する。その後、`
  - Actual: sources={'revealed_cards', 'hand'}

- **#481** `choice_not_parsed`: action=choice expected
  - Text: `自分のステージのセンターエリアにコスト9以上の『Aqours』のメンバーがいる場合、以下から1つを選ぶ。
・ライブ終了時まで、自分のステージにいるメンバー1人は`
  - Actual: actions=set()

- **#495** `target_opponent_missing`: target=opponent in tree
  - Text: `相手のステージにウェイト状態のメンバーがいる場合、このカードを成功させるための必要ハートを{{heart_00.png|heart0}}{{heart_00.p`
  - Actual: targets=set()

- **#522** `discard_source_missing`: source=discard expected for 控え室から手札に加える
  - Text: `以下から1つを選ぶ。自分の成功ライブカード置き場に『虹ヶ咲』のカードがある場合、代わりに1つ以上を選ぶ。
・自分のエネルギーデッキから、エネルギーカードを1枚ウ`
  - Actual: sources={'energy_deck'}

- **#522** `hand_dest_missing`: destination=hand expected for 手札に加える/置く
  - Text: `以下から1つを選ぶ。自分の成功ライブカード置き場に『虹ヶ咲』のカードがある場合、代わりに1つ以上を選ぶ。
・自分のエネルギーデッキから、エネルギーカードを1枚ウ`
  - Actual: dests={'energy_zone'}

- **#522** `choice_not_parsed`: action=choice expected
  - Text: `以下から1つを選ぶ。自分の成功ライブカード置き場に『虹ヶ咲』のカードがある場合、代わりに1つ以上を選ぶ。
・自分のエネルギーデッキから、エネルギーカードを1枚ウ`
  - Actual: actions=set()

- **#554** `multiple_targets_missing`: multiple_targets=True in tree
  - Text: `自分のステージにメンバーが1人以上いる場合、自分と相手はカードを1枚引き、手札を1枚控え室に置く。2人以上いる場合、さらに自分のステージにいる『μ's』のメンバ`
  - Actual: multiple_targets=set()

- **#561** `all_flag_missing`: all=True in tree
  - Text: `相手のステージにいるすべてのメンバーのそれぞれのコストよりコストが高いメンバーが自分のステージにいる場合、ライブ終了時まで、{{icon_blade.png|ブ`
  - Actual: all=set()

- **#561** `multiple_targets_missing`: multiple_targets=True in tree
  - Text: `相手のステージにいるすべてのメンバーのそれぞれのコストよりコストが高いメンバーが自分のステージにいる場合、ライブ終了時まで、{{icon_blade.png|ブ`
  - Actual: multiple_targets=set()

- **#577** `exclude_self_missing`: exclude_self=True in tree
  - Text: `このターン、自分のステージにいるほかのメンバーがエリアを移動している場合、カードを1枚引く。`
  - Actual: exclude_self=set()

- **#587** `stage_dest_missing`: destination=stage expected for ステージに置く/登場させる
  - Text: `{{icon_energy.png|E}}{{icon_energy.png|E}}このメンバーをステージから控え室に置く：自分の控え室からコスト15以下の『蓮`
  - Actual: dests={'same_area', 'discard'}

- **#592** `multiple_targets_missing`: multiple_targets=True in tree
  - Text: `{{icon_energy.png|E}}支払ってもよい：自分のステージに『蓮ノ空』のメンバー1人を含むメンバーが2人以上おり、かつそれらのメンバーのユニット名`
  - Actual: multiple_targets=set()

- **#593** `multiple_targets_missing`: multiple_targets=True in tree
  - Text: `自分のステージに名前とコストが両方ともそれぞれ異なるメンバーが3人以上いる場合、このカードのスコアを＋１する。`
  - Actual: multiple_targets=set()

- **#598** `stage_dest_missing`: destination=stage expected for ステージに置く/登場させる
  - Text: `{{icon_energy.png|E}}{{icon_energy.png|E}}支払ってもよい：自分のステージにコスト9以上の『EdelNote』のメンバー`
  - Actual: dests={'empty_area'}

- **#598** `choice_not_parsed`: action=choice expected
  - Text: `{{icon_energy.png|E}}{{icon_energy.png|E}}支払ってもよい：自分のステージにコスト9以上の『EdelNote』のメンバー`
  - Actual: actions=set()

- **#600** `multiple_targets_missing`: multiple_targets=True in tree
  - Text: `自分のステージにグループ名がそれぞれ異なるメンバーが3人以上いる場合、ライブ終了時まで、自分のセンターエリアにいるメンバーは{{icon_all.png|ハート`
  - Actual: multiple_targets=set()

- **#602** `choice_not_parsed`: action=choice expected
  - Text: `自分のステージに『A-RISE』のメンバーがいる場合、以下から1つを選ぶ。
・ウェイト状態のメンバー1人をアクティブにし、ライブ終了時まで、そのメンバーは{{i`
  - Actual: actions=set()

- **#620** `max_flag_missing`: max=True in tree
  - Text: `手札の『蓮ノ空』のメンバーカードを3枚まで控え室に置いてもよい：ライブ終了時まで、自分のステージのメンバー1人は、これにより控え室に置いたカード1枚につき、{{`
  - Actual: max=set()

- **#626** `stage_dest_missing`: destination=stage expected for ステージに置く/登場させる
  - Text: `手札を1枚控え室に置いてもよい：自分の控え室からコスト2以下の『Aqours』のメンバーカードを1枚、メンバーのいないエリアに登場させる。（この効果で登場したメ`
  - Actual: dests={'empty_area', 'discard'}

- **#642** `optional_flag_missing`: optional=True somewhere in tree
  - Text: `自分のステージにいるコスト10以上の『DOLLCHESTRA』のメンバー1人を選ぶ。そのメンバーの{{live_start.png|ライブ開始時}}能力1つを発`
  - Actual: optional=set()

## INFO (43)

- **#6** `discard_dest_missing`: destination=discard might be expected
  - Text: `自分のデッキの上からカードを3枚見る。その中から好きな枚数を好きな順番でデッキの上に置き、残りを控え室に置く。`
  - Actual: dests={'deck_top'}

- **#8** `discard_dest_missing`: destination=discard might be expected
  - Text: `このメンバーをウェイトにしてもよい：自分のデッキの上からカードを2枚見る。その中から好きな枚数を好きな順番でデッキの上に置き、残りを控え室に置く。（ウェイト状態`
  - Actual: dests={'deck_top'}

- **#33** `discard_dest_missing`: destination=discard might be expected
  - Text: `手札のライブカードを1枚公開し、デッキの一番下に置いてもよい：自分のデッキの上からカードを2枚見る。その中から好きな枚数を好きな順番でデッキの上に置き、残りを控`
  - Actual: dests={'deck_bottom', 'deck_top'}

- **#38** `kore_niyori_not_parsed`: conditional_on_result or condition with これにより expected
  - Text: `手札のコスト4以下の『Liella!』のメンバーカードを1枚控え室に置く：これにより控え室に置いたメンバーカードの{{toujyou.png|登場}}能力1つを`
  - Actual: actions=set()

- **#62** `kore_niyori_not_parsed`: conditional_on_result or condition with これにより expected
  - Text: `手札を2枚控え室に置いてもよい：自分のステージにいるこのメンバー以外のウェイト状態のメンバー1人をアクティブにする。そうした場合、ライブ終了時まで、これによりア`
  - Actual: actions=set()

- **#120** `kore_niyori_not_parsed`: conditional_on_result or condition with これにより expected
  - Text: `手札をすべて公開する：自分のステージにほかのメンバーがおり、かつこれにより公開した手札の中にライブカードがない場合、自分のデッキの上からカードを5枚見る。その中`
  - Actual: actions=set()

- **#125** `discard_dest_missing`: destination=discard might be expected
  - Text: `{{icon_energy.png|E}}支払ってもよい：自分のデッキの上からカードを3枚見る。その中から1枚を手札に加え、残りを控え室に置く。`
  - Actual: dests={'hand'}

- **#139** `discard_dest_missing`: destination=discard might be expected
  - Text: `自分のデッキの上からカードを4枚見る。その中からハートに{{heart_04.png|heart04}}を2つ以上持つメンバーカードを1枚公開して手札に加えても`
  - Actual: dests={'hand'}

- **#144** `pay_energy_not_parsed`: pay_energy expected for energy payment
  - Text: `手札を1枚控え室に置く：自分の控え室にあるライブカードを1枚選び、そのカードのスコアに等しい数の{{icon_energy.png|E}}を支払ってもよい。そう`
  - Actual: actions=set()

- **#171** `pay_energy_not_parsed`: pay_energy expected for energy payment
  - Text: `手札を1枚控え室に置いてもよい：自分のデッキの上からカードを3枚見る。その中から1枚を手札に加え、残りを控え室に置く。{{live_start.png|ライブ開`
  - Actual: actions=set()

- **#199** `duration_as_long_as_not_parsed`: duration=as_long_as expected
  - Text: `{{icon_energy.png|E}}{{icon_energy.png|E}}支払わないかぎり、自分の手札を2枚控え室に置く。`
  - Actual: duration=set()

- **#220** `discard_dest_missing`: destination=discard might be expected
  - Text: `{{icon_energy.png|E}}{{icon_energy.png|E}}支払ってもよい：自分のデッキの上からカードを7枚見る。その中から『Liell`
  - Actual: dests={'hand'}

- **#223** `kore_niyori_not_parsed`: conditional_on_result or condition with これにより expected
  - Text: `自分の控え室にある、カード名の異なるライブカードを2枚選ぶ。そうした場合、相手はそれらのカードのうち1枚を選ぶ。これにより相手に選ばれたカードを自分の手札に加え`
  - Actual: actions=set()

- **#231** `discard_dest_missing`: destination=discard might be expected
  - Text: `自分のデッキの上からカードを2枚見る。その中から好きな枚数を好きな順番でデッキの上に置き、残りを控え室に置く。`
  - Actual: dests={'deck_top'}

- **#241** `discard_dest_missing`: destination=discard might be expected
  - Text: `自分か相手を選ぶ。自分は、そのプレイヤーのデッキの上からカードを2枚見る。その中から好きな枚数を好きな順番でデッキの上に置き、残りを控え室に置く。`
  - Actual: dests={'deck_top'}

- **#266** `kore_niyori_not_parsed`: conditional_on_result or condition with これにより expected
  - Text: `自分のステージにいる『虹ヶ咲』のメンバー1人につき、自分のデッキの上からカードを1枚見る。その中から1枚までをデッキの上に置き、残りを控え室に置く。その後、自分`
  - Actual: actions=set()

- **#286** `duration_as_long_as_not_parsed`: duration=as_long_as expected
  - Text: `このメンバーをウェイトにしてもよい：カードを1枚引く。その後、このメンバーが『Printemps』のメンバーからバトンタッチして登場していないかぎり、手札を1枚`
  - Actual: duration={'unless'}

- **#290** `discard_dest_missing`: destination=discard might be expected
  - Text: `自分の成功ライブカード置き場にあるカードのスコアの合計が３以上の場合、自分のデッキの上からカードを5枚見る。その中から『μ's』のメンバーカードを1枚公開して手`
  - Actual: dests={'hand'}

- **#303** `discard_dest_missing`: destination=discard might be expected
  - Text: `このメンバーをウェイトにしてもよい：自分のデッキの上からカードを4枚見る。その中から必要ハートの合計が8以上の『Liella!』のライブカードを1枚公開して手札`
  - Actual: dests={'hand'}

- **#331** `discard_dest_missing`: destination=discard might be expected
  - Text: `自分のデッキの上からカードを2枚見る。その中から「朝香果林」のメンバーカードを1枚公開して手札に加えてもよい。残りを控え室に置く。`
  - Actual: dests={'hand'}

- **#333** `discard_dest_missing`: destination=discard might be expected
  - Text: `自分のデッキの上からカードを2枚見る。その中から「近江彼方」のメンバーカードを1枚公開して手札に加えてもよい。残りを控え室に置く。`
  - Actual: dests={'hand'}

- **#336** `discard_dest_missing`: destination=discard might be expected
  - Text: `自分のデッキの上からカードを2枚見る。その中から「天王寺璃奈」のメンバーカードを1枚公開して手札に加えてもよい。残りを控え室に置く。`
  - Actual: dests={'hand'}

- **#339** `discard_dest_missing`: destination=discard might be expected
  - Text: `自分のデッキの上からカードを2枚見る。その中から「鐘嵐珠」のメンバーカードを1枚公開して手札に加えてもよい。残りを控え室に置く。`
  - Actual: dests={'hand'}

- **#367** `discard_dest_missing`: destination=discard might be expected
  - Text: `自分のデッキの上からカードを2枚見る。その中から好きな枚数を好きな順番でデッキの上に置き、残りを控え室に置く。`
  - Actual: dests={'deck_top'}

- **#377** `discard_dest_missing`: destination=discard might be expected
  - Text: `自分のデッキの上からカードを5枚見る。その中から『μ's』のライブカードを1枚公開して手札に加えてもよい。残りを控え室に置く。`
  - Actual: dests={'hand'}

- **#378** `kore_niyori_not_parsed`: conditional_on_result or condition with これにより expected
  - Text: `手札のライブカードを1枚公開してもよい：自分の成功ライブカード置き場にあるカードを1枚手札に加える。そうした場合、これにより公開したカードを自分の成功ライブカー`
  - Actual: actions=set()

- **#382** `discard_dest_missing`: destination=discard might be expected
  - Text: `自分のデッキの上からカードを3枚見る。その中から好きな枚数を好きな順番でデッキの上に置き、残りを控え室に置く。`
  - Actual: dests={'deck_top'}

- **#407** `per_unit_not_parsed`: per_unit=True expected
  - Text: `{{icon_energy.png|E}}{{icon_energy.png|E}}支払ってもよい：自分のステージに『虹ヶ咲』のメンバーがいる場合、このカードの`
  - Actual: per_unit=set()

- **#408** `per_unit_not_parsed`: per_unit=True expected
  - Text: `自分のライブ中のカードが3枚以上ある場合、このカードのスコアを＋２する。
(エールをすべて行った後、エールで出た{{icon_draw.png|ドロー}}1つに`
  - Actual: per_unit=set()

- **#409** `per_unit_not_parsed`: per_unit=True expected
  - Text: `ライブの合計スコアが相手より高い場合、自分のエネルギーデッキから、エネルギーカードを1枚ウェイト状態で置く。
(エールで出た{{icon_score.png|ス`
  - Actual: per_unit=set()

- **#413** `per_unit_not_parsed`: per_unit=True expected
  - Text: `自分のエネルギーが12枚以上ある場合、このカードのスコアを＋１する。
(エールをすべて行った後、エールで出た{{icon_draw.png|ドロー}}1つにつき`
  - Actual: per_unit=set()

- **#415** `per_unit_not_parsed`: per_unit=True expected
  - Text: `エールにより公開された自分のカードの中に『蓮ノ空』のメンバーカードが10枚以上ある場合、このカードのスコアを＋１する。
(エールをすべて行った後、エールで出た{`
  - Actual: per_unit=set()

- **#417** `discard_dest_missing`: destination=discard might be expected
  - Text: `自分のデッキの上からカードを5枚見る。その中から『虹ヶ咲』のライブカードを1枚まで公開して手札に加えてもよい。残りを控え室に置く。`
  - Actual: dests={'hand'}

- **#422** `per_unit_not_parsed`: per_unit=True expected
  - Text: `自分のステージにいるメンバーが持つ{{icon_blade.png|ブレード}}の合計が10以上の場合、このカードのスコアを＋１する。
(エールをすべて行った後`
  - Actual: per_unit=set()

- **#428** `discard_dest_missing`: destination=discard might be expected
  - Text: `{{icon_energy.png|E}}支払ってもよい：自分のエネルギーが9枚以上ある場合、自分のデッキの上からカードを5枚見る。その中から1枚を手札に加え、`
  - Actual: dests={'hand'}

- **#430** `per_unit_not_parsed`: per_unit=True expected
  - Text: `自分のエネルギーが9枚以上ある場合、このカードのスコアを＋１する。
(エールをすべて行った後、エールで出た{{icon_draw.png|ドロー}}1つにつき、`
  - Actual: per_unit=set()

- **#448** `discard_dest_missing`: destination=discard might be expected
  - Text: `このメンバーがステージから控え室に置かれたとき、自分のデッキの上からカードを5枚見る。その中からメンバーカードを1枚公開して手札に加えてもよい。残りを控え室に置`
  - Actual: dests={'hand'}

- **#449** `discard_dest_missing`: destination=discard might be expected
  - Text: `このメンバーがステージから控え室に置かれたとき、自分のデッキの上からカードを5枚見る。その中からライブカードを1枚公開して手札に加えてもよい。残りを控え室に置く`
  - Actual: dests={'hand'}

- **#453** `pay_energy_not_parsed`: pay_energy expected for energy payment
  - Text: `自分のメインフェイズの場合、{{icon_energy.png|E}}{{icon_energy.png|E}}支払ってもよい：自分の控え室からライブカードを1`
  - Actual: actions=set()

- **#572** `kore_niyori_not_parsed`: conditional_on_result or condition with これにより expected
  - Text: `自分のステージに「中須かすみ」がいる場合、自分のデッキの上からカードを4枚公開する。自分はそれらの中から「中須かすみ」のカードを1枚選ぶ。ライブ終了時まで、自分`
  - Actual: actions=set()

- **#608** `kore_niyori_not_parsed`: conditional_on_result or condition with これにより expected
  - Text: `手札を2枚控え室に置いてもよい：自分のデッキの上からカードを5枚見る。その中からメンバーカードを1枚公開して手札に加えてもよい。残りを控え室に置く。これにより『`
  - Actual: actions=set()

- **#623** `discard_dest_missing`: destination=discard might be expected
  - Text: `自分のデッキの上からカードを5枚見る。その中から『Aqours』のライブカードを1枚公開して手札に加えてもよい。残りを控え室に置く。`
  - Actual: dests={'hand'}

- **#631** `discard_dest_missing`: destination=discard might be expected
  - Text: `自分のステージにいる『Aqours』のメンバー1人につき、カードを1枚引く。その後、これにより引いた枚数と同じ枚数を手札から控え室に置く。`
  - Actual: dests={'hand'}

## Summary

- Total: 102
- Errors: 0
- Warnings: 59
- Infos: 43