# Parser Validation Report

Generated: 2026-06-19T20:15:17.191652

Total abilities: 645

## ERROR (25)

- **#37** `move_cards_incomplete`: move_cards must have source + destination
  - Text: `バトンタッチして登場した場合、このバトンタッチで控え室に置かれた『Liella!』のメンバーカードを1枚手札に加える。`
  - Actual: source=None dest=hand text=このバトンタッチで控え室に置かれた『Liella!』のメンバーカードを1枚手札に

- **#58** `move_cards_incomplete`: move_cards must have source + destination
  - Text: `自分のエネルギー置き場にあるエネルギー1枚をこのメンバーの下に置いてもよい。そうした場合、カードを1枚引き、ライブ終了時まで、自分のステージにいるメンバーは{{`
  - Actual: source=None dest=under_member text=自分のエネルギー置き場にあるエネルギー1枚をこのメンバーの下に置いてもよい。

- **#69** `move_cards_incomplete`: move_cards must have source + destination
  - Text: `相手のステージにいるウェイト状態のメンバーの数まで、自分の控え室にある『虹ヶ咲』のメンバーカードを選ぶ。それらを好きな順番でデッキの上に置く。`
  - Actual: source=None dest=deck_top text=それらを好きな順番でデッキの上に置く

- **#120** `move_cards_incomplete`: move_cards must have source + destination
  - Text: `手札にあるコスト2以下の『μ's』のメンバーカードを1枚公開し、このメンバーの下に置いてもよい。そうした場合、好きなハートの色を1つ指定する。ライブ終了時まで、`
  - Actual: source=None dest=under_member text=手札にあるコスト2以下の『μ's』のメンバーカードを1枚公開し、このメンバーの下

- **#124** `move_cards_incomplete`: move_cards must have source + destination
  - Text: `『Aqours』のライブカードが自分のライブカード置き場から控え室に置かれたとき、そのライブカードをデッキの一番上か一番下に置いてもよい。`
  - Actual: source=None dest=deck_top_or_bottom text=そのライブカードをデッキの一番上か一番下に置いてもよい

- **#238** `move_cards_incomplete`: move_cards must have source + destination
  - Text: `エールにより公開された自分のカードの中にライブカードがないとき、それらのカードをすべて控え室に置いてもよい。これにより1枚以上のカードが控え室に置かれた場合、そ`
  - Actual: source=None dest=discard text=それらのカードをすべて控え室に置いてもよい

- **#288** `move_cards_incomplete`: move_cards must have source + destination
  - Text: `{{icon_energy.png|E}}{{icon_energy.png|E}}このメンバーをステージから控え室に置く：自分の手札からコスト13以下の「優木`
  - Actual: source=None dest=under_member text=自分のエネルギー置き場にあるエネルギー1枚をそのメンバーの下に置く

- **#293** `move_cards_incomplete`: move_cards must have source + destination
  - Text: `このメンバーをウェイトにし、手札を1枚控え室に置く：ライブカードかコスト10以上のメンバーカードのどちらか1つを選ぶ。選んだカードが公開されるまで、自分のデッキ`
  - Actual: source=None dest=hand text=そのカードを手札に加え

- **#323** `move_cards_incomplete`: move_cards must have source + destination
  - Text: `自分か相手を選ぶ。自分は、そのプレイヤーのデッキの一番上のカードを見る。自分はそのカードを控え室に置いてもよい。`
  - Actual: source=None dest=discard text=自分はそのカードを控え室に置いてもよい

- **#339** `move_cards_incomplete`: move_cards must have source + destination
  - Text: `自分のエネルギー置き場にあるエネルギー2枚をこのメンバーの下に置いてもよい。`
  - Actual: source=None dest=under_member text=自分のエネルギー置き場にあるエネルギー2枚をこのメンバーの下に置いてもよい

- **#343** `move_cards_incomplete`: move_cards must have source + destination
  - Text: `自分のデッキの一番上のカードを公開する。公開したカードがコスト9以下のメンバーカードの場合、公開したカードを手札に加え、このメンバーはポジションチェンジする。そ`
  - Actual: source=None dest=hand text=公開したカードを手札に加え

- **#343** `move_cards_incomplete`: move_cards must have source + destination
  - Text: `自分のデッキの一番上のカードを公開する。公開したカードがコスト9以下のメンバーカードの場合、公開したカードを手札に加え、このメンバーはポジションチェンジする。そ`
  - Actual: source=None dest=discard text=公開したカードを控え室に置く

- **#386** `move_cards_incomplete`: move_cards must have source + destination
  - Text: `数1つを選ぶ。自分のデッキの一番上のカードを公開する。公開したカードがメンバーカードで、かつコストが選んだ数以上の場合、公開したカードを手札に加える。選んだ数以`
  - Actual: source=None dest=hand text=公開したカードを手札に加える

- **#395** `move_cards_incomplete`: move_cards must have source + destination
  - Text: `自分と相手はそれぞれ、自身の控え室にあるすべてのメンバーカードをシャッフルし、自身のデッキの下に置く。これにより自分と相手のカードが合計20枚以上デッキの下に置`
  - Actual: source=None dest=deck_bottom text=自身のデッキの下に置く

- **#400** `gain_resource_incomplete`: gain_resource must have resource + count
  - Text: `自分のステージにほかのメンバーがいないかぎり、{{icon_blade.png|ブレード}}{{icon_blade.png|ブレード}}{{icon_blad`
  - Actual: resource=heart count=None dyn=None text={{icon_blade.png|ブレード}}{{icon_blade.png|

- **#527** `move_cards_incomplete`: move_cards must have source + destination
  - Text: `エールにより自分のカードを1枚以上公開したとき、それらのカードの中にブレードハートを持つカードが2枚以下の場合、それらのカードをすべて控え室に置いてもよい。その`
  - Actual: source=None dest=discard text=それらのカードをすべて控え室に置いてもよい

- **#531** `move_cards_incomplete`: move_cards must have source + destination
  - Text: `自分のエネルギー置き場にあるエネルギー1枚をこのメンバーの下に置いてもよい。そうした場合、カードを2枚引く。（メンバーの下に置かれているエネルギーカードではコス`
  - Actual: source=None dest=under_member text=自分のエネルギー置き場にあるエネルギー1枚をこのメンバーの下に置いてもよい。

- **#534** `move_cards_incomplete`: move_cards must have source + destination
  - Text: `自分のステージにいるメンバー1人の下にあるエネルギーカードを、好きな枚数エネルギーデッキに置いてもよい。そうした場合、ライブ終了時まで、そのメンバーは、これによ`
  - Actual: source=under_member dest=None text=自分のステージにいるメンバー1人の下にあるエネルギーカードを、好きな枚数エネルギ

- **#668** `move_cards_incomplete`: move_cards must have source + destination
  - Text: `カードを1枚引いてもよい。そうした場合、手札2枚を好きな順番でデッキの上に置く。`
  - Actual: source=None dest=deck_top text=手札2枚を好きな順番でデッキの上に置く

- **#693** `move_cards_incomplete`: move_cards must have source + destination
  - Text: `自分のデッキの上からカードを3枚見る。それらを好きな順番でデッキの上に置く。`
  - Actual: source=None dest=deck_top text=それらを好きな順番でデッキの上に置く

- **#700** `move_cards_incomplete`: move_cards must have source + destination
  - Text: `このカードを成功ライブカード置き場に置く場合、代わりに自分の控え室にある『μ's』のライブカードを1枚置いてもよい。`
  - Actual: source=discard dest=None text=自分の控え室にある『μ's』のライブカードを1枚置いてもよい

- **#708** `modify_score_incomplete`: modify_score must have operation + value
  - Text: `自分がエールしたとき、エールにより公開された自分のカードの中からブレードハートを持たない『Aqours』のメンバーカードを1枚まで控え室に置いてもよい。そうした`
  - Actual: op=add val=None text=これにより控え室に置いたカードのコスト5につき、追加で1枚エールを行う。この能力

- **#708** `modify_score_incomplete`: modify_score must have operation + value
  - Text: `自分がエールしたとき、エールにより公開された自分のカードの中からブレードハートを持たない『Aqours』のメンバーカードを1枚まで控え室に置いてもよい。そうした`
  - Actual: op=add val=None text=この能力では4枚までしか追加でエールできない

- **#725** `modify_score_incomplete`: modify_score must have operation + value
  - Text: `自分がエールしたとき、エールにより公開された自分のブレードハートを持たない『蓮ノ空』のカードを3枚まで控え室に置いてもよい。そうした場合、これにより控え室に置い`
  - Actual: op=add val=None text=これにより控え室に置いた数に等しい枚数のエールを追加で行う

- **#733** `move_cards_incomplete`: move_cards must have source + destination
  - Text: `自分のデッキの上からカードを1枚見る。そのカードを控え室に置いてもよい。`
  - Actual: source=None dest=discard text=そのカードを控え室に置いてもよい

## WARNING (61)

- **#23** `any_number_missing`: any_number=True in tree
  - Text: `手札にあるメンバーカードを好きな枚数公開する：公開したカードのコストの合計が、10、20、30、40、50のいずれかの場合、ライブ終了時まで、「{{jyouji`
  - Actual: any_number=set()

- **#33** `optional_flag_missing`: optional=True somewhere in tree
  - Text: `手札のライブカードを1枚公開し、デッキの一番下に置いてもよい：自分のデッキの上からカードを2枚見る。その中から好きな枚数を好きな順番でデッキの上に置き、残りを控`
  - Actual: optional=set()

- **#56** `max_flag_missing`: max=True in tree
  - Text: `手札を2枚まで控え室に置いてもよい：ライブ終了時まで、これによって控え室に置いたカード1枚につき、{{icon_blade.png|ブレード}}{{icon_b`
  - Actual: max=set()

- **#57** `stage_dest_missing`: destination=stage expected for ステージに置く/登場させる
  - Text: `このメンバーをウェイトにし、手札を1枚控え室に置く：このメンバー以外の『Aqours』のメンバー1人を自分のステージから控え室に置く。そうした場合、自分の控え室`
  - Actual: dests={'same_area', 'discard'}

- **#63** `optional_flag_missing`: optional=True somewhere in tree
  - Text: `控え室にあるメンバーカード2枚を好きな順番でデッキの一番下に置いてもよい：それらのカードのコストの合計が、6の場合、カードを1枚引く。合計が8の場合、ライブ終了`
  - Actual: optional=set()

- **#63** `placement_order_missing`: placement_order=any_order in tree
  - Text: `控え室にあるメンバーカード2枚を好きな順番でデッキの一番下に置いてもよい：それらのカードのコストの合計が、6の場合、カードを1枚引く。合計が8の場合、ライブ終了`
  - Actual: placement_order=set()

- **#78** `stage_dest_missing`: destination=stage expected for ステージに置く/登場させる
  - Text: `『Liella!』のメンバー2人からバトンタッチして登場している場合、カードを2枚引き、自分の控え室にあるコスト4以下の『Liella!』のメンバーカード1枚を`
  - Actual: dests={'hand', 'empty_area'}

- **#88** `discard_source_missing`: source=discard expected for 控え室から手札に加える
  - Text: `{{icon_energy.png|E}}{{icon_energy.png|E}}手札を1枚控え室に置く：これにより控え室に置いたカードが『μ's』のカードの`
  - Actual: sources={'hand', 'deck_top'}

- **#92** `stage_dest_missing`: destination=stage expected for ステージに置く/登場させる
  - Text: `能力を持たないメンバーカードを自分の手札から登場させるためのコストは1減る。`
  - Actual: dests=set()

- **#105** `stage_dest_missing`: destination=stage expected for ステージに置く/登場させる
  - Text: `コスト10の『Liella!』のメンバーカードを自分の手札から登場させるためのコストは2減る。`
  - Actual: dests=set()

- **#109** `optional_flag_missing`: optional=True somewhere in tree
  - Text: `自分のメインフェイズの間、自分のカードが1枚以上いずれかの領域から控え室に置かれるたび、{{icon_energy.png|E}}支払ってもよい。そうした場合、`
  - Actual: optional=set()

- **#112** `multiple_targets_missing`: multiple_targets=True in tree
  - Text: `自分のステージにコストがそれぞれ異なるメンバーが3人以上いるかぎり、{{heart_05.png|heart05}}{{icon_blade.png|ブレード}`
  - Actual: multiple_targets=set()

- **#113** `stage_dest_missing`: destination=stage expected for ステージに置く/登場させる
  - Text: `{{icon_energy.png|E}}{{icon_energy.png|E}}：自分の控え室からコスト2以下のメンバーカードを1枚、メンバーのいないエリア`
  - Actual: dests={'empty_area'}

- **#122** `deck_top_source_missing`: source=deck_top in tree
  - Text: `手札を1枚控え室に置く：好きなハートの色を1つ指定する。その後、自分のデッキの上からカードを5枚公開する。公開されたカードの中に指定した色のハートを持つメンバー`
  - Actual: sources={'hand'}

- **#136** `hand_to_discard_not_found`: source=hand AND dest=discard somewhere in tree
  - Text: `以下から1つを選ぶ。
・カードを1枚引き、手札を1枚控え室に置く。
・相手のステージにいるすべてのコスト2以下のメンバーをウェイトにする。`
  - Actual: sources={'stage', 'deck'} dests={'hand'}

- **#139** `target_opponent_missing`: target=opponent in tree
  - Text: `自分か相手のステージにコスト13以上のメンバーがいる場合、{{icon_blade.png|ブレード}}{{icon_blade.png|ブレード}}を得る。`
  - Actual: targets={'either'}

- **#140** `hand_to_discard_not_found`: source=hand AND dest=discard somewhere in tree
  - Text: `手札をすべて公開する：自分のステージにほかのメンバーがおり、かつこれにより公開した手札の中にライブカードがない場合、自分のデッキの上からカードを5枚見る。その中`
  - Actual: sources={'hand', 'deck_top'} dests={'hand'}

- **#161** `max_flag_missing`: max=True in tree
  - Text: `手札のブレードハートを持たないメンバーカードを2枚まで控え室に置いてもよい：自分の控え室から、これにより控え室に置いたカードと同じ枚数の『Aqours』のライブ`
  - Actual: max=set()

- **#168** `target_both_missing`: target=both in tree
  - Text: `自分と相手のステージの中で、このメンバーがほかのすべてのメンバーより多くのハートを持つかぎり、ライブの合計スコアを＋１する。`
  - Actual: targets=set()

- **#168** `all_flag_missing`: all=True in tree
  - Text: `自分と相手のステージの中で、このメンバーがほかのすべてのメンバーより多くのハートを持つかぎり、ライブの合計スコアを＋１する。`
  - Actual: all=set()

- **#168** `exclude_self_missing`: exclude_self=True in tree
  - Text: `自分と相手のステージの中で、このメンバーがほかのすべてのメンバーより多くのハートを持つかぎり、ライブの合計スコアを＋１する。`
  - Actual: exclude_self=set()

- **#178** `multiple_targets_missing`: multiple_targets=True in tree
  - Text: `手札を1枚控え室に置いてもよい：自分のデッキの上からカードを5枚見る。その中から各グループ名につき1枚ずつ公開し、3枚まで手札に加えてもよい。残りを控え室に置く`
  - Actual: multiple_targets=set()

- **#192** `target_opponent_missing`: target=opponent in tree
  - Text: `このメンバーがエリアを移動したとき、ライブ終了時まで、{{heart_06.png|heart06}}を得る。
(対戦相手のカードの効果でも発動する。)`
  - Actual: targets=set()

- **#219** `stage_dest_missing`: destination=stage expected for ステージに置く/登場させる
  - Text: `{{icon_energy.png|E}}{{icon_energy.png|E}}、このメンバーをステージから控え室に置く：自分の控え室からコスト15以下の『`
  - Actual: dests={'same_area', 'discard'}

- **#220** `max_flag_missing`: max=True in tree
  - Text: `手札を3枚まで控え室に置いてもよい：これにより置いた枚数分カードを引く。`
  - Actual: max=set()

- **#229** `target_opponent_missing`: target=opponent in tree
  - Text: `このメンバーが登場か、エリアを移動するたび、ライブ終了時まで、{{icon_blade.png|ブレード}}{{icon_blade.png|ブレード}}を得る`
  - Actual: targets=set()

- **#234** `stage_dest_missing`: destination=stage expected for ステージに置く/登場させる
  - Text: `「鬼塚冬毬」以外の『Liella!』のメンバー1人をステージから控え室に置いてもよい：自分の控え室から、これにより控え室に置いたメンバーカードを1枚、そのメンバ`
  - Actual: dests={'same_area', 'discard'}

- **#258** `multiple_targets_missing`: multiple_targets=True in tree
  - Text: `自分のステージの右サイドエリアに「大沢瑠璃乃」が、左サイドエリアに「安養寺姫芽」が、センターエリアに「藤島慈」がそれぞれ登場している場合、このカードのスコアを＋`
  - Actual: multiple_targets=set()

- **#288** `stage_dest_missing`: destination=stage expected for ステージに置く/登場させる
  - Text: `{{icon_energy.png|E}}{{icon_energy.png|E}}このメンバーをステージから控え室に置く：自分の手札からコスト13以下の「優木`
  - Actual: dests={'under_member', 'same_area', 'discard'}

- **#290** `multiple_targets_missing`: multiple_targets=True in tree
  - Text: `相手のステージにいる「ミア・テイラー」以外のメンバーを1人選ぶ。そのメンバーが持つハートと、このメンバーが持つハートの中に同じ色のハートがある場合、ライブ終了時`
  - Actual: multiple_targets=set()

- **#301** `max_flag_missing`: max=True in tree
  - Text: `メンバーを3人までウェイトにしてもよい：これによりウェイト状態にしたメンバー1人につき、カードを1枚引く。`
  - Actual: max=set()

- **#313** `stage_dest_missing`: destination=stage expected for ステージに置く/登場させる
  - Text: `自分と相手はそれぞれ、自身の控え室からコスト2以下のメンバーカードを1枚、メンバーのいないエリアにウェイト状態で登場させる。（この効果で登場したメンバーのいるエ`
  - Actual: dests={'empty_area'}

- **#346** `multiple_targets_missing`: multiple_targets=True in tree
  - Text: `自分のライブ中のライブカードの必要ハートの中に{{heart_01.png|heart01}}、{{heart_02.png|heart02}}、{{heart`
  - Actual: multiple_targets=set()

- **#353** `exclude_self_missing`: exclude_self=True in tree
  - Text: `自分のステージにこのメンバー以外のコスト11のメンバーが登場したとき、自分のエネルギーデッキから、エネルギーカードを1枚ウェイト状態で置く。`
  - Actual: exclude_self=set()

- **#379** `optional_flag_missing`: optional=True somewhere in tree
  - Text: `自分のステージにほかの『スリーズブーケ』のメンバーが登場するたび、{{icon_energy.png|E}}支払ってもよい。そうした場合、エネルギーを2枚アクテ`
  - Actual: optional=set()

- **#408** `stage_dest_missing`: destination=stage expected for ステージに置く/登場させる
  - Text: `{{icon_energy.png|E}}{{icon_energy.png|E}}手札を1枚控え室に置く：このメンバー以外の『Aqours』のメンバー1人を自`
  - Actual: dests={'same_area', 'discard'}

- **#412** `stage_dest_missing`: destination=stage expected for ステージに置く/登場させる
  - Text: `{{icon_energy.png|E}}{{icon_energy.png|E}}このメンバーをステージから控え室に置く：自分の控え室からコスト17以下の『A`
  - Actual: dests={'same_area', 'discard'}

- **#429** `optional_flag_missing`: optional=True somewhere in tree
  - Text: `手札のライブカードを1枚公開してもよい：自分の成功ライブカード置き場にあるカードを1枚手札に加える。そうした場合、これにより公開したカードを自分の成功ライブカー`
  - Actual: optional=set()

- **#441** `target_opponent_missing`: target=opponent in tree
  - Text: `直前のターンに相手がライブをし、それが成功していない場合、相手にエマパンチ打つ？と聞いてもよい。
回答がお願いしますの場合、自分は相手にエマパンチする。ライブ終`
  - Actual: targets=set()

- **#441** `optional_flag_missing`: optional=True somewhere in tree
  - Text: `直前のターンに相手がライブをし、それが成功していない場合、相手にエマパンチ打つ？と聞いてもよい。
回答がお願いしますの場合、自分は相手にエマパンチする。ライブ終`
  - Actual: optional=set()

- **#441** `all_flag_missing`: all=True in tree
  - Text: `直前のターンに相手がライブをし、それが成功していない場合、相手にエマパンチ打つ？と聞いてもよい。
回答がお願いしますの場合、自分は相手にエマパンチする。ライブ終`
  - Actual: all=set()

- **#483** `target_opponent_missing`: target=opponent in tree
  - Text: `このメンバーがエリアを移動するたび、カードを1枚引く。
(対戦相手のカードの効果でも発動する。)`
  - Actual: targets=set()

- **#518** `deck_top_source_missing`: source=deck_top in tree
  - Text: `自分のデッキの上から、自分と相手のステージにいるメンバー1人につき、1枚公開する。それらの中にあるライブカード1枚につき、このカードのスコアを＋１する。その後、`
  - Actual: sources={'hand', 'revealed_cards'}

- **#543** `target_opponent_missing`: target=opponent in tree
  - Text: `相手のステージにウェイト状態のメンバーがいる場合、このカードを成功させるための必要ハートを{{heart_00.png|heart0}}{{heart_00.p`
  - Actual: targets=set()

- **#600** `multiple_targets_missing`: multiple_targets=True in tree
  - Text: `自分のステージにメンバーが1人以上いる場合、自分と相手はカードを1枚引き、手札を1枚控え室に置く。2人以上いる場合、さらに自分のステージにいる『μ's』のメンバ`
  - Actual: multiple_targets=set()

- **#607** `all_flag_missing`: all=True in tree
  - Text: `相手のステージにいるすべてのメンバーのそれぞれのコストよりコストが高いメンバーが自分のステージにいる場合、ライブ終了時まで、{{icon_blade.png|ブ`
  - Actual: all=set()

- **#607** `multiple_targets_missing`: multiple_targets=True in tree
  - Text: `相手のステージにいるすべてのメンバーのそれぞれのコストよりコストが高いメンバーが自分のステージにいる場合、ライブ終了時まで、{{icon_blade.png|ブ`
  - Actual: multiple_targets=set()

- **#623** `exclude_self_missing`: exclude_self=True in tree
  - Text: `このターン、自分のステージにいるほかのメンバーがエリアを移動している場合、カードを1枚引く。`
  - Actual: exclude_self=set()

- **#633** `stage_dest_missing`: destination=stage expected for ステージに置く/登場させる
  - Text: `{{icon_energy.png|E}}{{icon_energy.png|E}}このメンバーをステージから控え室に置く：自分の控え室からコスト15以下の『蓮`
  - Actual: dests={'same_area', 'discard'}

- **#637** `multiple_targets_missing`: multiple_targets=True in tree
  - Text: `{{icon_energy.png|E}}支払ってもよい：自分のステージに『蓮ノ空』のメンバー1人を含むメンバーが2人以上おり、かつそれらのメンバーのユニット名`
  - Actual: multiple_targets=set()

- **#638** `multiple_targets_missing`: multiple_targets=True in tree
  - Text: `自分のステージに名前とコストが両方ともそれぞれ異なるメンバーが3人以上いる場合、このカードのスコアを＋１する。`
  - Actual: multiple_targets=set()

- **#643** `stage_dest_missing`: destination=stage expected for ステージに置く/登場させる
  - Text: `{{icon_energy.png|E}}{{icon_energy.png|E}}支払ってもよい：自分のステージにコスト9以上の『EdelNote』のメンバー`
  - Actual: dests={'empty_area'}

- **#645** `multiple_targets_missing`: multiple_targets=True in tree
  - Text: `自分のステージにグループ名がそれぞれ異なるメンバーが3人以上いる場合、ライブ終了時まで、自分のセンターエリアにいるメンバーは{{icon_all.png|ハート`
  - Actual: multiple_targets=set()

- **#664** `max_flag_missing`: max=True in tree
  - Text: `手札の『蓮ノ空』のメンバーカードを3枚まで控え室に置いてもよい：ライブ終了時まで、自分のステージのメンバー1人は、これにより控え室に置いたカード1枚につき、{{`
  - Actual: max=set()

- **#670** `stage_dest_missing`: destination=stage expected for ステージに置く/登場させる
  - Text: `手札を1枚控え室に置いてもよい：自分の控え室からコスト2以下の『Aqours』のメンバーカードを1枚、メンバーのいないエリアに登場させる。（この効果で登場したメ`
  - Actual: dests={'empty_area', 'discard'}

- **#672** `optional_flag_missing`: optional=True somewhere in tree
  - Text: `手札の『Aqours』のカードを1枚公開してもよい：これにより公開したカードをデッキの一番上か一番下に置き、ライブ終了時まで、{{icon_blade.png|`
  - Actual: optional=set()

- **#694** `stage_dest_missing`: destination=stage expected for ステージに置く/登場させる
  - Text: `このカードが自分の成功ライブカード置き場にあるかぎり、元々のコストが17以上の『μ's』のメンバーカードを自分の手札から登場させるためのコストは2減る。この効果`
  - Actual: dests=set()

- **#719** `stage_dest_missing`: destination=stage expected for ステージに置く/登場させる
  - Text: `{{icon_energy.png|E}}{{icon_energy.png|E}}{{icon_energy.png|E}}{{icon_energy.png`
  - Actual: dests={'empty_area'}

- **#748** `target_opponent_missing`: target=opponent in tree
  - Text: `このメンバーがエリアを移動したとき、ライブ終了時まで、{{icon_blade.png|ブレード}}を得る。
(対戦相手のカードの効果でも発動する。)`
  - Actual: targets=set()

- **#749** `target_opponent_missing`: target=opponent in tree
  - Text: `このメンバーがエリアを移動したとき、ライブ終了時まで、{{heart_02.png|heart02}}を得る。
(対戦相手のカードの効果でも発動する。)`
  - Actual: targets=set()

- **#751** `target_opponent_missing`: target=opponent in tree
  - Text: `このメンバーがエリアを移動したとき、ライブ終了時まで、{{heart_03.png|heart03}}を得る。
(対戦相手のカードの効果でも発動する。)`
  - Actual: targets=set()

## INFO (55)

- **#6** `discard_dest_missing`: destination=discard might be expected
  - Text: `このメンバーをウェイトにしてもよい：自分のデッキの上からカードを2枚見る。その中から好きな枚数を好きな順番でデッキの上に置き、残りを控え室に置く。（ウェイト状態`
  - Actual: dests={'deck_top'}

- **#7** `discard_dest_missing`: destination=discard might be expected
  - Text: `自分のデッキの上からカードを3枚見る。その中から好きな枚数を好きな順番でデッキの上に置き、残りを控え室に置く。`
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

- **#109** `conditional_sequential_not_parsed`: sequential with conditional=True expected
  - Text: `自分のメインフェイズの間、自分のカードが1枚以上いずれかの領域から控え室に置かれるたび、{{icon_energy.png|E}}支払ってもよい。そうした場合、`
  - Actual: top=[]

- **#126** `conditional_sequential_not_parsed`: sequential with conditional=True expected
  - Text: `自分のライブカード置き場にカードが2枚以上ある場合、その中から{{live_start.png|ライブ開始時}}能力を持たない『Aqours』のライブカードを1`
  - Actual: top=[]

- **#129** `discard_dest_missing`: destination=discard might be expected
  - Text: `自分のデッキの上から、自分のステージにいるメンバーの数に2を足した数に等しい枚数見る。その中から1枚をデッキの一番上に置き、残りを控え室に置く。`
  - Actual: dests={'deck_top'}

- **#133** `per_unit_not_parsed`: per_unit=True expected
  - Text: `手札にあるこのメンバーカードのコストは、自分のステージにいる『みらくらぱーく！』のメンバー1人につき、2少なくなる。`
  - Actual: per_unit=set()

- **#142** `duration_as_long_as_not_parsed`: duration=as_long_as expected
  - Text: `{{icon_energy.png|E}}{{icon_energy.png|E}}支払わないかぎり、自分の手札を2枚控え室に置く。`
  - Actual: duration=set()

- **#147** `discard_dest_missing`: destination=discard might be expected
  - Text: `{{icon_energy.png|E}}支払ってもよい：自分のデッキの上からカードを3枚見る。その中から1枚を手札に加え、残りを控え室に置く。`
  - Actual: dests={'hand'}

- **#164** `discard_dest_missing`: destination=discard might be expected
  - Text: `自分のデッキの上からカードを4枚見る。その中からハートに{{heart_04.png|heart04}}を2つ以上持つメンバーカードを1枚公開して手札に加えても`
  - Actual: dests={'hand'}

- **#169** `pay_energy_not_parsed`: pay_energy expected for energy payment
  - Text: `手札を1枚控え室に置く：自分の控え室にあるライブカードを1枚選び、そのカードのスコアに等しい数の{{icon_energy.png|E}}を支払ってもよい。そう`
  - Actual: actions=set()

- **#244** `discard_dest_missing`: destination=discard might be expected
  - Text: `{{icon_energy.png|E}}{{icon_energy.png|E}}支払ってもよい：自分のデッキの上からカードを7枚見る。その中から『Liell`
  - Actual: dests={'hand'}

- **#247** `kore_niyori_not_parsed`: conditional_on_result or condition with これにより expected
  - Text: `自分の控え室にある、カード名の異なるライブカードを2枚選ぶ。そうした場合、相手はそれらのカードのうち1枚を選ぶ。これにより相手に選ばれたカードを自分の手札に加え`
  - Actual: actions=set()

- **#255** `discard_dest_missing`: destination=discard might be expected
  - Text: `自分のデッキの上からカードを2枚見る。その中から好きな枚数を好きな順番でデッキの上に置き、残りを控え室に置く。`
  - Actual: dests={'deck_top'}

- **#266** `discard_dest_missing`: destination=discard might be expected
  - Text: `自分か相手を選ぶ。自分は、そのプレイヤーのデッキの上からカードを2枚見る。その中から好きな枚数を好きな順番でデッキの上に置き、残りを控え室に置く。`
  - Actual: dests={'deck_top'}

- **#292** `discard_dest_missing`: destination=discard might be expected
  - Text: `自分のステージにいる『虹ヶ咲』のメンバー1人につき、自分のデッキの上からカードを1枚見る。その中から1枚までをデッキの上に置き、残りを控え室に置く。その後、自分`
  - Actual: dests={'deck_top'}

- **#292** `kore_niyori_not_parsed`: conditional_on_result or condition with これにより expected
  - Text: `自分のステージにいる『虹ヶ咲』のメンバー1人につき、自分のデッキの上からカードを1枚見る。その中から1枚までをデッキの上に置き、残りを控え室に置く。その後、自分`
  - Actual: actions=set()

- **#316** `discard_dest_missing`: destination=discard might be expected
  - Text: `自分の成功ライブカード置き場にあるカードのスコアの合計が３以上の場合、自分のデッキの上からカードを5枚見る。その中から『μ's』のメンバーカードを1枚公開して手`
  - Actual: dests={'hand'}

- **#330** `discard_dest_missing`: destination=discard might be expected
  - Text: `このメンバーをウェイトにしてもよい：自分のデッキの上からカードを4枚見る。その中から必要ハートの合計が8以上の『Liella!』のライブカードを1枚公開して手札`
  - Actual: dests={'hand'}

- **#358** `discard_dest_missing`: destination=discard might be expected
  - Text: `自分のデッキの上からカードを2枚見る。その中から「朝香果林」のメンバーカードを1枚公開して手札に加えてもよい。残りを控え室に置く。`
  - Actual: dests={'hand'}

- **#360** `discard_dest_missing`: destination=discard might be expected
  - Text: `自分のデッキの上からカードを2枚見る。その中から「近江彼方」のメンバーカードを1枚公開して手札に加えてもよい。残りを控え室に置く。`
  - Actual: dests={'hand'}

- **#363** `discard_dest_missing`: destination=discard might be expected
  - Text: `自分のデッキの上からカードを2枚見る。その中から「天王寺璃奈」のメンバーカードを1枚公開して手札に加えてもよい。残りを控え室に置く。`
  - Actual: dests={'hand'}

- **#366** `discard_dest_missing`: destination=discard might be expected
  - Text: `自分のデッキの上からカードを2枚見る。その中から「鐘嵐珠」のメンバーカードを1枚公開して手札に加えてもよい。残りを控え室に置く。`
  - Actual: dests={'hand'}

- **#379** `conditional_sequential_not_parsed`: sequential with conditional=True expected
  - Text: `自分のステージにほかの『スリーズブーケ』のメンバーが登場するたび、{{icon_energy.png|E}}支払ってもよい。そうした場合、エネルギーを2枚アクテ`
  - Actual: top=[]

- **#396** `discard_dest_missing`: destination=discard might be expected
  - Text: `自分のデッキの上からカードを2枚見る。その中から好きな枚数を好きな順番でデッキの上に置き、残りを控え室に置く。`
  - Actual: dests={'deck_top'}

- **#402** `discard_dest_missing`: destination=discard might be expected
  - Text: `自分のデッキの上からカードを2枚見る。その中から能力を持たない『μ's』のカードか{{jyouji.png|常時}}能力を持つ『μ's』のカードを1枚公開して手`
  - Actual: dests={'hand'}

- **#409** `discard_dest_missing`: destination=discard might be expected
  - Text: `自分のデッキの上からカードを2枚見る。その中から{{heart_02.png|heart02}}と{{heart_04.png|heart04}}と{{hear`
  - Actual: dests={'hand'}

- **#428** `discard_dest_missing`: destination=discard might be expected
  - Text: `自分のデッキの上からカードを5枚見る。その中から『μ's』のライブカードを1枚公開して手札に加えてもよい。残りを控え室に置く。`
  - Actual: dests={'hand'}

- **#429** `kore_niyori_not_parsed`: conditional_on_result or condition with これにより expected
  - Text: `手札のライブカードを1枚公開してもよい：自分の成功ライブカード置き場にあるカードを1枚手札に加える。そうした場合、これにより公開したカードを自分の成功ライブカー`
  - Actual: actions=set()

- **#433** `discard_dest_missing`: destination=discard might be expected
  - Text: `自分のデッキの上からカードを3枚見る。その中から好きな枚数を好きな順番でデッキの上に置き、残りを控え室に置く。`
  - Actual: dests={'deck_top'}

- **#458** `per_unit_not_parsed`: per_unit=True expected
  - Text: `{{icon_energy.png|E}}{{icon_energy.png|E}}支払ってもよい：自分のステージに『虹ヶ咲』のメンバーがいる場合、このカードの`
  - Actual: per_unit=set()

- **#459** `per_unit_not_parsed`: per_unit=True expected
  - Text: `自分のライブ中のカードが3枚以上ある場合、このカードのスコアを＋２する。
(エールをすべて行った後、エールで出た{{icon_draw.png|ドロー}}1つに`
  - Actual: per_unit=set()

- **#460** `per_unit_not_parsed`: per_unit=True expected
  - Text: `ライブの合計スコアが相手より高い場合、自分のエネルギーデッキから、エネルギーカードを1枚ウェイト状態で置く。
(エールで出た{{icon_score.png|ス`
  - Actual: per_unit=set()

- **#464** `per_unit_not_parsed`: per_unit=True expected
  - Text: `自分のエネルギーが12枚以上ある場合、このカードのスコアを＋１する。
(エールをすべて行った後、エールで出た{{icon_draw.png|ドロー}}1つにつき`
  - Actual: per_unit=set()

- **#466** `per_unit_not_parsed`: per_unit=True expected
  - Text: `エールにより公開された自分のカードの中に『蓮ノ空』のメンバーカードが10枚以上ある場合、このカードのスコアを＋１する。
(エールをすべて行った後、エールで出た{`
  - Actual: per_unit=set()

- **#468** `discard_dest_missing`: destination=discard might be expected
  - Text: `自分のデッキの上からカードを5枚見る。その中から『虹ヶ咲』のライブカードを1枚まで公開して手札に加えてもよい。残りを控え室に置く。`
  - Actual: dests={'hand'}

- **#473** `per_unit_not_parsed`: per_unit=True expected
  - Text: `自分のステージにいるメンバーが持つ{{icon_blade.png|ブレード}}の合計が10以上の場合、このカードのスコアを＋１する。
(エールをすべて行った後`
  - Actual: per_unit=set()

- **#478** `discard_dest_missing`: destination=discard might be expected
  - Text: `{{icon_energy.png|E}}支払ってもよい：自分のエネルギーが9枚以上ある場合、自分のデッキの上からカードを5枚見る。その中から1枚を手札に加え、`
  - Actual: dests={'hand'}

- **#479** `per_unit_not_parsed`: per_unit=True expected
  - Text: `自分のエネルギーが9枚以上ある場合、このカードのスコアを＋１する。
(エールをすべて行った後、エールで出た{{icon_draw.png|ドロー}}1つにつき、`
  - Actual: per_unit=set()

- **#496** `discard_dest_missing`: destination=discard might be expected
  - Text: `このメンバーがステージから控え室に置かれたとき、自分のデッキの上からカードを5枚見る。その中からメンバーカードを1枚公開して手札に加えてもよい。残りを控え室に置`
  - Actual: dests={'hand'}

- **#497** `discard_dest_missing`: destination=discard might be expected
  - Text: `このメンバーがステージから控え室に置かれたとき、自分のデッキの上からカードを5枚見る。その中からライブカードを1枚公開して手札に加えてもよい。残りを控え室に置く`
  - Actual: dests={'hand'}

- **#501** `pay_energy_not_parsed`: pay_energy expected for energy payment
  - Text: `自分のメインフェイズの場合、{{icon_energy.png|E}}{{icon_energy.png|E}}支払ってもよい：自分の控え室からライブカードを1`
  - Actual: actions=set()

- **#507** `per_unit_not_parsed`: per_unit=True expected
  - Text: `手札にあるこのメンバーカードのコストは、このカード以外の自分の手札1枚につき、1少なくなる。`
  - Actual: per_unit=set()

- **#618** `kore_niyori_not_parsed`: conditional_on_result or condition with これにより expected
  - Text: `自分のステージに「中須かすみ」がいる場合、自分のデッキの上からカードを4枚公開する。自分はそれらの中から「中須かすみ」のカードを1枚選ぶ。ライブ終了時まで、自分`
  - Actual: actions=set()

- **#653** `kore_niyori_not_parsed`: conditional_on_result or condition with これにより expected
  - Text: `手札を2枚控え室に置いてもよい：自分のデッキの上からカードを5枚見る。その中からメンバーカードを1枚公開して手札に加えてもよい。残りを控え室に置く。これにより『`
  - Actual: actions=set()

- **#667** `discard_dest_missing`: destination=discard might be expected
  - Text: `自分のデッキの上からカードを5枚見る。その中から『Aqours』のライブカードを1枚公開して手札に加えてもよい。残りを控え室に置く。`
  - Actual: dests={'hand'}

- **#674** `discard_dest_missing`: destination=discard might be expected
  - Text: `自分のステージにいる『Aqours』のメンバー1人につき、カードを1枚引く。その後、これにより引いた枚数と同じ枚数を手札から控え室に置く。`
  - Actual: dests={'hand'}

- **#705** `discard_dest_missing`: destination=discard might be expected
  - Text: `控え室から登場している場合、自分のデッキの上からカードを3枚見る。その中から1枚を手札に加え、残りを控え室に置く。`
  - Actual: dests={'hand'}

- **#708** `kore_niyori_not_parsed`: conditional_on_result or condition with これにより expected
  - Text: `自分がエールしたとき、エールにより公開された自分のカードの中からブレードハートを持たない『Aqours』のメンバーカードを1枚まで控え室に置いてもよい。そうした`
  - Actual: actions=set()

- **#711** `kore_niyori_not_parsed`: conditional_on_result or condition with これにより expected
  - Text: `ライブ終了時まで、相手は余剰ハートをすべて失う。これにより相手が余剰ハートを2つ以上失っている場合、このカードのスコアを＋１する。`
  - Actual: actions=set()

- **#725** `kore_niyori_not_parsed`: conditional_on_result or condition with これにより expected
  - Text: `自分がエールしたとき、エールにより公開された自分のブレードハートを持たない『蓮ノ空』のカードを3枚まで控え室に置いてもよい。そうした場合、これにより控え室に置い`
  - Actual: actions=set()

- **#726** `discard_dest_missing`: destination=discard might be expected
  - Text: `このターン、自分が余剰ハートを1つ以上持っている場合、自分のデッキの上からカードを2枚見る。その中から好きな枚数を好きな順番でデッキの上に置き、残りを控え室に置`
  - Actual: dests={'deck_top'}

- **#731** `discard_dest_missing`: destination=discard might be expected
  - Text: `自分のデッキの上からカードを6枚見る。その中からカードを2枚手札に加え、残りを控え室に置く。`
  - Actual: dests={'hand'}

## Summary

- Total: 141
- Errors: 25
- Warnings: 61
- Infos: 55