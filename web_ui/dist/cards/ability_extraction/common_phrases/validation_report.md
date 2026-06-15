# Parser Validation Report

Generated: 2026-05-26T10:56:03.719809

Total abilities: 645

## ERROR (6)

- **#208** `draw_card_no_count`: draw_card must have count or dynamic_count
  - Text: `手札を3枚まで控え室に置いてもよい：これにより置いた枚数分カードを引く。`
  - Actual: text=これにより置いた枚数分カードを引く

- **#406** `move_cards_incomplete`: move_cards must have source + destination
  - Text: `{{center.png|センター}}手札にあるコスト2以下の『μ's』のメンバーカードを1枚公開し、このメンバーの下に置いてもよい。そうした場合、好きなハート`
  - Actual: source=None dest=under_member text=手札にあるコスト2以下の『μ's』のメンバーカードを1枚公開し、このメンバーの下

- **#415** `move_cards_incomplete`: move_cards must have source + destination
  - Text: `『Aqours』のライブカードが自分のライブカード置き場から控え室に置かれたとき、そのライブカードをデッキの一番上か一番下に置いてもよい。`
  - Actual: source=None dest=deck_top_or_bottom text=そのライブカードをデッキの一番上か一番下に置いてもよい

- **#710** `move_cards_incomplete`: move_cards must have source + destination
  - Text: `このカードを成功ライブカード置き場に置く場合、代わりに自分の控え室にある『μ's』のライブカードを1枚置いてもよい。`
  - Actual: source=discard dest=None text=自分の控え室にある『μ's』のライブカードを1枚置いてもよい

- **#718** `modify_score_incomplete`: modify_score must have operation + value
  - Text: `自分がエールしたとき、エールにより公開された自分のカードの中からブレードハートを持たない『Aqours』のメンバーカードを1枚まで控え室に置いてもよい。そうした`
  - Actual: op=add val=None text=これにより控え室に置いたカードのコスト5につき、追加で1枚エールを行う。この能力

- **#735** `modify_score_incomplete`: modify_score must have operation + value
  - Text: `自分がエールしたとき、エールにより公開された自分のブレードハートを持たない『蓮ノ空』のカードを3枚まで控え室に置いてもよい。そうした場合、これにより控え室に置い`
  - Actual: op=add val=None text=これにより控え室に置いた数に等しい枚数のエールを追加で行う

## WARNING (71)

- **#23** `any_number_missing`: any_number=True in tree
  - Text: `手札にあるメンバーカードを好きな枚数公開する：公開したカードのコストの合計が、10、20、30、40、50のいずれかの場合、ライブ終了時まで、「{{jyouji`
  - Actual: any_number=set()

- **#53** `stage_dest_missing`: destination=stage expected for ステージに置く/登場させる
  - Text: `{{center.png|センター}}このメンバーをウェイトにし、手札を1枚控え室に置く：このメンバー以外の『Aqours』のメンバー1人を自分のステージから控`
  - Actual: dests={'discard', 'same_area'}

- **#71** `optional_flag_missing`: optional=True somewhere in tree
  - Text: `このカードのプレイに際し、2人のメンバーとバトンタッチしてもよい。`
  - Actual: optional=set()

- **#72** `stage_dest_missing`: destination=stage expected for ステージに置く/登場させる
  - Text: `{{center.png|センター}}『Liella!』のメンバー2人からバトンタッチして登場している場合、カードを2枚引き、自分の控え室にあるコスト4以下の『`
  - Actual: dests={'hand', 'empty_area'}

- **#96** `choice_not_parsed`: action=choice expected
  - Text: `{{icon_energy.png|E}}支払ってもよい：以下から1つを選ぶ。
・相手のステージにいるコスト4以下のメンバー1人をウェイトにする。
・カードを1`
  - Actual: actions=set()

- **#106** `multiple_targets_missing`: multiple_targets=True in tree
  - Text: `自分のステージにコストがそれぞれ異なるメンバーが3人以上いるかぎり、{{heart_05.png|heart05}}{{icon_blade.png|ブレード}`
  - Actual: multiple_targets=set()

- **#107** `stage_dest_missing`: destination=stage expected for ステージに置く/登場させる
  - Text: `{{icon_energy.png|E}}{{icon_energy.png|E}}：自分の控え室からコスト2以下のメンバーカードを1枚、メンバーのいないエリア`
  - Actual: dests={'empty_area'}

- **#112** `deck_top_source_missing`: source=deck_top in tree
  - Text: `手札を1枚控え室に置く：好きなハートの色を1つ指定する。その後、自分のデッキの上からカードを5枚公開する。公開されたカードの中に指定した色のハートを持つメンバー`
  - Actual: sources={'hand'}

- **#124** `hand_to_discard_not_found`: source=hand AND dest=discard somewhere in tree
  - Text: `以下から1つを選ぶ。
・カードを1枚引き、手札を1枚控え室に置く。
・相手のステージにいるすべてのコスト2以下のメンバーをウェイトにする。`
  - Actual: sources={'deck'} dests={'hand'}

- **#124** `choice_not_parsed`: action=choice expected
  - Text: `以下から1つを選ぶ。
・カードを1枚引き、手札を1枚控え室に置く。
・相手のステージにいるすべてのコスト2以下のメンバーをウェイトにする。`
  - Actual: actions=set()

- **#127** `target_opponent_missing`: target=opponent in tree
  - Text: `自分か相手のステージにコスト13以上のメンバーがいる場合、{{icon_blade.png|ブレード}}{{icon_blade.png|ブレード}}を得る。`
  - Actual: targets={'either'}

- **#128** `hand_to_discard_not_found`: source=hand AND dest=discard somewhere in tree
  - Text: `手札をすべて公開する：自分のステージにほかのメンバーがおり、かつこれにより公開した手札の中にライブカードがない場合、自分のデッキの上からカードを5枚見る。その中`
  - Actual: sources={'hand', 'deck_top'} dests={'hand'}

- **#149** `max_flag_missing`: max=True in tree
  - Text: `手札のブレードハートを持たないメンバーカードを2枚まで控え室に置いてもよい：自分の控え室から、これにより控え室に置いたカードと同じ枚数の『Aqours』のライブ`
  - Actual: max=set()

- **#150** `choice_not_parsed`: action=choice expected
  - Text: `以下から1つを選ぶ。
・自分のステージにいるこのメンバー以外の『Aqours』のメンバー1人は、ライブ終了時まで、{{icon_blade.png|ブレード}}`
  - Actual: actions=set()

- **#156** `target_both_missing`: target=both in tree
  - Text: `自分と相手のステージの中で、このメンバーがほかのすべてのメンバーより多くのハートを持つかぎり、ライブの合計スコアを＋１する。`
  - Actual: targets=set()

- **#156** `all_flag_missing`: all=True in tree
  - Text: `自分と相手のステージの中で、このメンバーがほかのすべてのメンバーより多くのハートを持つかぎり、ライブの合計スコアを＋１する。`
  - Actual: all=set()

- **#164** `choice_not_parsed`: action=choice expected
  - Text: `以下から1つを選ぶ。
・自分の控え室にカード名が異なるライブカードが3枚以上ある場合、自分の控え室からライブカードを1枚手札に加える。
・自分の控え室にグループ`
  - Actual: actions=set()

- **#166** `multiple_targets_missing`: multiple_targets=True in tree
  - Text: `手札を1枚控え室に置いてもよい：自分のデッキの上からカードを5枚見る。その中から各グループ名につき1枚ずつ公開し、3枚まで手札に加えてもよい。残りを控え室に置く`
  - Actual: multiple_targets=set()

- **#166** `max_flag_missing`: max=True in tree
  - Text: `手札を1枚控え室に置いてもよい：自分のデッキの上からカードを5枚見る。その中から各グループ名につき1枚ずつ公開し、3枚まで手札に加えてもよい。残りを控え室に置く`
  - Actual: max=set()

- **#180** `target_opponent_missing`: target=opponent in tree
  - Text: `このメンバーがエリアを移動したとき、ライブ終了時まで、{{heart_06.png|heart06}}を得る。
(対戦相手のカードの効果でも発動する。)`
  - Actual: targets=set()

- **#207** `stage_dest_missing`: destination=stage expected for ステージに置く/登場させる
  - Text: `{{icon_energy.png|E}}{{icon_energy.png|E}}、このメンバーをステージから控え室に置く：自分の控え室からコスト15以下の『`
  - Actual: dests={'discard', 'same_area'}

- **#208** `max_flag_missing`: max=True in tree
  - Text: `手札を3枚まで控え室に置いてもよい：これにより置いた枚数分カードを引く。`
  - Actual: max=set()

- **#217** `target_opponent_missing`: target=opponent in tree
  - Text: `このメンバーが登場か、エリアを移動するたび、ライブ終了時まで、{{icon_blade.png|ブレード}}{{icon_blade.png|ブレード}}を得る`
  - Actual: targets=set()

- **#222** `stage_dest_missing`: destination=stage expected for ステージに置く/登場させる
  - Text: `「鬼塚冬毬」以外の『Liella!』のメンバー1人をステージから控え室に置いてもよい：自分の控え室から、これにより控え室に置いたメンバーカードを1枚、そのメンバ`
  - Actual: dests={'discard', 'same_area'}

- **#246** `multiple_targets_missing`: multiple_targets=True in tree
  - Text: `自分のステージの右サイドエリアに「大沢瑠璃乃」が、左サイドエリアに「安養寺姫芽」が、センターエリアに「藤島慈」がそれぞれ登場している場合、このカードのスコアを＋`
  - Actual: multiple_targets=set()

- **#270** `max_flag_missing`: max=True in tree
  - Text: `手札を2枚まで控え室に置いてもよい：ライブ終了時まで、これによって控え室に置いたカード1枚につき、{{icon_blade.png|ブレード}}{{icon_b`
  - Actual: max=set()

- **#272** `max_flag_missing`: max=True in tree
  - Text: `手札を2枚まで控え室に置いてもよい：ライブ終了時まで、これによって控え室に置いたカード1枚につき、{{icon_blade.png|ブレード}}{{icon_b`
  - Actual: max=set()

- **#287** `stage_dest_missing`: destination=stage expected for ステージに置く/登場させる
  - Text: `{{icon_energy.png|E}}{{icon_energy.png|E}}このメンバーをステージから控え室に置く：自分の手札からコスト13以下の「優木`
  - Actual: dests={'under_member', 'discard', 'same_area'}

- **#289** `multiple_targets_missing`: multiple_targets=True in tree
  - Text: `相手のステージにいる「ミア・テイラー」以外のメンバーを1人選ぶ。そのメンバーが持つハートと、このメンバーが持つハートの中に同じ色のハートがある場合、ライブ終了時`
  - Actual: multiple_targets=set()

- **#299** `discard_source_missing`: source=discard expected for 控え室から手札に加える
  - Text: `手札を3枚控え室に置く：自分のステージにほかの『lilywhite』のメンバーがいる場合、自分の控え室から『μ's』のライブカードを1枚手札に加える。この能力を`
  - Actual: sources={'hand'}

- **#299** `hand_dest_missing`: destination=hand expected for 手札に加える/置く
  - Text: `手札を3枚控え室に置く：自分のステージにほかの『lilywhite』のメンバーがいる場合、自分の控え室から『μ's』のライブカードを1枚手札に加える。この能力を`
  - Actual: dests={'discard'}

- **#300** `max_flag_missing`: max=True in tree
  - Text: `メンバーを3人までウェイトにしてもよい：これによりウェイト状態にしたメンバー1人につき、カードを1枚引く。`
  - Actual: max=set()

- **#312** `stage_dest_missing`: destination=stage expected for ステージに置く/登場させる
  - Text: `自分と相手はそれぞれ、自身の控え室からコスト2以下のメンバーカードを1枚、メンバーのいないエリアにウェイト状態で登場させる。（この効果で登場したメンバーのいるエ`
  - Actual: dests={'empty_area'}

- **#345** `multiple_targets_missing`: multiple_targets=True in tree
  - Text: `自分のライブ中のライブカードの必要ハートの中に{{heart_01.png|heart01}}、{{heart_02.png|heart02}}、{{heart`
  - Actual: multiple_targets=set()

- **#349** `choice_not_parsed`: action=choice expected
  - Text: `以下から1つを選ぶ。
・エネルギーを1枚アクティブにする。
・自分の控え室にある『虹ヶ咲』のライブカードを2枚まで好きな順番でデッキの上に置く。`
  - Actual: actions=set()

- **#352** `exclude_self_missing`: exclude_self=True in tree
  - Text: `自分のステージにこのメンバー以外のコスト11のメンバーが登場したとき、自分のエネルギーデッキから、エネルギーカードを1枚ウェイト状態で置く。`
  - Actual: exclude_self=set()

- **#413** `original_value_missing`: original_value=True in tree
  - Text: `{{center.png|センター}}自分のステージの右サイドエリアと左サイドエリアに、元々持つ{{icon_blade.png|ブレード}}の数が2つのメンバ`
  - Actual: original_value=set()

- **#419** `stage_dest_missing`: destination=stage expected for ステージに置く/登場させる
  - Text: `{{icon_energy.png|E}}{{icon_energy.png|E}}手札を1枚控え室に置く：このメンバー以外の『Aqours』のメンバー1人を自`
  - Actual: dests={'discard', 'same_area'}

- **#423** `stage_dest_missing`: destination=stage expected for ステージに置く/登場させる
  - Text: `{{icon_energy.png|E}}{{icon_energy.png|E}}このメンバーをステージから控え室に置く：自分の控え室からコスト17以下の『A`
  - Actual: dests={'discard', 'same_area'}

- **#452** `optional_flag_missing`: optional=True somewhere in tree
  - Text: `直前のターンに相手がライブをし、それが成功していない場合、相手にエマパンチ打つ？と聞いてもよい。
回答がお願いしますの場合、自分は相手にエマパンチする。ライブ終`
  - Actual: optional=set()

- **#462** `original_value_missing`: original_value=True in tree
  - Text: `自分のステージに、元々持つハートの数より多い数のハートを持つメンバーがいる場合、カードを1枚引く。`
  - Actual: original_value=set()

- **#494** `target_opponent_missing`: target=opponent in tree
  - Text: `このメンバーがエリアを移動するたび、カードを1枚引く。
(対戦相手のカードの効果でも発動する。)`
  - Actual: targets=set()

- **#520** `any_number_missing`: any_number=True in tree
  - Text: `手札の「渡辺曜」と「鬼塚夏美」と「大沢瑠璃乃」を、好きな枚数控え室に置いてもよい：ライブ終了時まで、これによって控え室に置いた枚数1枚につき、{{icon_bl`
  - Actual: any_number=set()

- **#528** `deck_top_source_missing`: source=deck_top in tree
  - Text: `自分のデッキの上から、自分と相手のステージにいるメンバー1人につき、1枚公開する。それらの中にあるライブカード1枚につき、このカードのスコアを＋１する。その後、`
  - Actual: sources={'hand', 'revealed_cards'}

- **#539** `choice_not_parsed`: action=choice expected
  - Text: `自分のステージのセンターエリアにコスト9以上の『Aqours』のメンバーがいる場合、以下から1つを選ぶ。
・ライブ終了時まで、自分のステージにいるメンバー1人は`
  - Actual: actions=set()

- **#553** `target_opponent_missing`: target=opponent in tree
  - Text: `相手のステージにウェイト状態のメンバーがいる場合、このカードを成功させるための必要ハートを{{heart_00.png|heart0}}{{heart_00.p`
  - Actual: targets=set()

- **#579** `discard_source_missing`: source=discard expected for 控え室から手札に加える
  - Text: `以下から1つを選ぶ。自分の成功ライブカード置き場に『虹ヶ咲』のカードがある場合、代わりに1つ以上を選ぶ。
・自分のエネルギーデッキから、エネルギーカードを1枚ウ`
  - Actual: sources=set()

- **#579** `hand_dest_missing`: destination=hand expected for 手札に加える/置く
  - Text: `以下から1つを選ぶ。自分の成功ライブカード置き場に『虹ヶ咲』のカードがある場合、代わりに1つ以上を選ぶ。
・自分のエネルギーデッキから、エネルギーカードを1枚ウ`
  - Actual: dests=set()

- **#579** `choice_not_parsed`: action=choice expected
  - Text: `以下から1つを選ぶ。自分の成功ライブカード置き場に『虹ヶ咲』のカードがある場合、代わりに1つ以上を選ぶ。
・自分のエネルギーデッキから、エネルギーカードを1枚ウ`
  - Actual: actions=set()

- **#610** `multiple_targets_missing`: multiple_targets=True in tree
  - Text: `自分のステージにメンバーが1人以上いる場合、自分と相手はカードを1枚引き、手札を1枚控え室に置く。2人以上いる場合、さらに自分のステージにいる『μ's』のメンバ`
  - Actual: multiple_targets=set()

- **#617** `all_flag_missing`: all=True in tree
  - Text: `相手のステージにいるすべてのメンバーのそれぞれのコストよりコストが高いメンバーが自分のステージにいる場合、ライブ終了時まで、{{icon_blade.png|ブ`
  - Actual: all=set()

- **#617** `multiple_targets_missing`: multiple_targets=True in tree
  - Text: `相手のステージにいるすべてのメンバーのそれぞれのコストよりコストが高いメンバーが自分のステージにいる場合、ライブ終了時まで、{{icon_blade.png|ブ`
  - Actual: multiple_targets=set()

- **#633** `exclude_self_missing`: exclude_self=True in tree
  - Text: `このターン、自分のステージにいるほかのメンバーがエリアを移動している場合、カードを1枚引く。`
  - Actual: exclude_self=set()

- **#643** `stage_dest_missing`: destination=stage expected for ステージに置く/登場させる
  - Text: `{{icon_energy.png|E}}{{icon_energy.png|E}}このメンバーをステージから控え室に置く：自分の控え室からコスト15以下の『蓮`
  - Actual: dests={'discard', 'same_area'}

- **#647** `multiple_targets_missing`: multiple_targets=True in tree
  - Text: `{{icon_energy.png|E}}支払ってもよい：自分のステージに『蓮ノ空』のメンバー1人を含むメンバーが2人以上おり、かつそれらのメンバーのユニット名`
  - Actual: multiple_targets=set()

- **#648** `multiple_targets_missing`: multiple_targets=True in tree
  - Text: `自分のステージに名前とコストが両方ともそれぞれ異なるメンバーが3人以上いる場合、このカードのスコアを＋１する。`
  - Actual: multiple_targets=set()

- **#653** `stage_dest_missing`: destination=stage expected for ステージに置く/登場させる
  - Text: `{{icon_energy.png|E}}{{icon_energy.png|E}}支払ってもよい：自分のステージにコスト9以上の『EdelNote』のメンバー`
  - Actual: dests={'empty_area'}

- **#653** `choice_not_parsed`: action=choice expected
  - Text: `{{icon_energy.png|E}}{{icon_energy.png|E}}支払ってもよい：自分のステージにコスト9以上の『EdelNote』のメンバー`
  - Actual: actions=set()

- **#655** `multiple_targets_missing`: multiple_targets=True in tree
  - Text: `自分のステージにグループ名がそれぞれ異なるメンバーが3人以上いる場合、ライブ終了時まで、自分のセンターエリアにいるメンバーは{{icon_all.png|ハート`
  - Actual: multiple_targets=set()

- **#657** `choice_not_parsed`: action=choice expected
  - Text: `自分のステージに『A-RISE』のメンバーがいる場合、以下から1つを選ぶ。
・ウェイト状態のメンバー1人をアクティブにし、ライブ終了時まで、そのメンバーは{{i`
  - Actual: actions=set()

- **#674** `max_flag_missing`: max=True in tree
  - Text: `手札の『蓮ノ空』のメンバーカードを3枚まで控え室に置いてもよい：ライブ終了時まで、自分のステージのメンバー1人は、これにより控え室に置いたカード1枚につき、{{`
  - Actual: max=set()

- **#680** `stage_dest_missing`: destination=stage expected for ステージに置く/登場させる
  - Text: `手札を1枚控え室に置いてもよい：自分の控え室からコスト2以下の『Aqours』のメンバーカードを1枚、メンバーのいないエリアに登場させる。（この効果で登場したメ`
  - Actual: dests={'discard', 'empty_area'}

- **#695** `optional_flag_missing`: optional=True somewhere in tree
  - Text: `自分のステージにいるコスト10以上の『DOLLCHESTRA』のメンバー1人を選ぶ。そのメンバーの{{live_start.png|ライブ開始時}}能力1つを発`
  - Actual: optional=set()

- **#717** `choice_not_parsed`: action=choice expected
  - Text: `以下から1つを選ぶ。
・このカードは「{{live_success.png|ライブ成功時}}カードを1枚引く。」を得る。
・ライブ終了時まで、このターンにバトン`
  - Actual: actions=set()

- **#729** `stage_dest_missing`: destination=stage expected for ステージに置く/登場させる
  - Text: `{{icon_energy.png|E}}{{icon_energy.png|E}}{{icon_energy.png|E}}{{icon_energy.png`
  - Actual: dests={'empty_area'}

- **#742** `any_number_missing`: any_number=True in tree
  - Text: `手札の「南ことり」と「黒澤ダイヤ」と「徒町小鈴」を、好きな枚数控え室に置いてもよい：ライブ終了時まで、これにより控え室に置いたそれらのカードが持つハートの色1つ`
  - Actual: any_number=set()

- **#746** `choice_not_parsed`: action=choice expected
  - Text: `以下から1つを選ぶ。
・自分のデッキの上からカードを3枚控え室に置く。
・相手のステージにいるコスト2以下のメンバー1人をウェイトにする。`
  - Actual: actions=set()

- **#751** `choice_not_parsed`: action=choice expected
  - Text: `{{icon_energy.png|E}}支払ってもよい：以下から1つを選ぶ。
・自分の控え室からメンバーカードを1枚手札に加える。
・自分のライブカード置き場`
  - Actual: actions=set()

- **#758** `target_opponent_missing`: target=opponent in tree
  - Text: `このメンバーがエリアを移動したとき、ライブ終了時まで、{{icon_blade.png|ブレード}}を得る。
(対戦相手のカードの効果でも発動する。)`
  - Actual: targets=set()

- **#759** `target_opponent_missing`: target=opponent in tree
  - Text: `このメンバーがエリアを移動したとき、ライブ終了時まで、{{heart_02.png|heart02}}を得る。
(対戦相手のカードの効果でも発動する。)`
  - Actual: targets=set()

- **#761** `target_opponent_missing`: target=opponent in tree
  - Text: `このメンバーがエリアを移動したとき、ライブ終了時まで、{{heart_03.png|heart03}}を得る。
(対戦相手のカードの効果でも発動する。)`
  - Actual: targets=set()

## INFO (52)

- **#6** `discard_dest_missing`: destination=discard might be expected
  - Text: `このメンバーをウェイトにしてもよい：自分のデッキの上からカードを2枚見る。その中から好きな枚数を好きな順番でデッキの上に置き、残りを控え室に置く。（ウェイト状態`
  - Actual: dests={'deck_top'}

- **#7** `discard_dest_missing`: destination=discard might be expected
  - Text: `自分のデッキの上からカードを3枚見る。その中から好きな枚数を好きな順番でデッキの上に置き、残りを控え室に置く。`
  - Actual: dests={'deck_top'}

- **#33** `discard_dest_missing`: destination=discard might be expected
  - Text: `手札のライブカードを1枚公開し、デッキの一番下に置いてもよい：自分のデッキの上からカードを2枚見る。その中から好きな枚数を好きな順番でデッキの上に置き、残りを控`
  - Actual: dests={'deck_top', 'deck_bottom'}

- **#38** `kore_niyori_not_parsed`: conditional_on_result or condition with これにより expected
  - Text: `手札のコスト4以下の『Liella!』のメンバーカードを1枚控え室に置く：これにより控え室に置いたメンバーカードの{{toujyou.png|登場}}能力1つを`
  - Actual: actions=set()

- **#56** `kore_niyori_not_parsed`: conditional_on_result or condition with これにより expected
  - Text: `手札を2枚控え室に置いてもよい：自分のステージにいるこのメンバー以外のウェイト状態のメンバー1人をアクティブにする。そうした場合、ライブ終了時まで、これによりア`
  - Actual: actions=set()

- **#114** `conditional_sequential_not_parsed`: sequential with conditional=True expected
  - Text: `自分のライブカード置き場にカードが2枚以上ある場合、その中から{{live_start.png|ライブ開始時}}能力を持たない『Aqours』のライブカードを1`
  - Actual: top=[]

- **#121** `per_unit_not_parsed`: per_unit=True expected
  - Text: `手札にあるこのメンバーカードのコストは、自分のステージにいる『みらくらぱーく！』のメンバー1人につき、2少なくなる。`
  - Actual: per_unit=set()

- **#130** `duration_as_long_as_not_parsed`: duration=as_long_as expected
  - Text: `{{icon_energy.png|E}}{{icon_energy.png|E}}支払わないかぎり、自分の手札を2枚控え室に置く。`
  - Actual: duration=set()

- **#135** `discard_dest_missing`: destination=discard might be expected
  - Text: `{{icon_energy.png|E}}支払ってもよい：自分のデッキの上からカードを3枚見る。その中から1枚を手札に加え、残りを控え室に置く。`
  - Actual: dests={'hand'}

- **#152** `discard_dest_missing`: destination=discard might be expected
  - Text: `自分のデッキの上からカードを4枚見る。その中からハートに{{heart_04.png|heart04}}を2つ以上持つメンバーカードを1枚公開して手札に加えても`
  - Actual: dests={'hand'}

- **#157** `pay_energy_not_parsed`: pay_energy expected for energy payment
  - Text: `手札を1枚控え室に置く：自分の控え室にあるライブカードを1枚選び、そのカードのスコアに等しい数の{{icon_energy.png|E}}を支払ってもよい。そう`
  - Actual: actions=set()

- **#185** `pay_energy_not_parsed`: pay_energy expected for energy payment
  - Text: `手札を1枚控え室に置いてもよい：自分のデッキの上からカードを3枚見る。その中から1枚を手札に加え、残りを控え室に置く。{{live_start.png|ライブ開`
  - Actual: actions=set()

- **#232** `discard_dest_missing`: destination=discard might be expected
  - Text: `{{icon_energy.png|E}}{{icon_energy.png|E}}支払ってもよい：自分のデッキの上からカードを7枚見る。その中から『Liell`
  - Actual: dests={'hand'}

- **#235** `kore_niyori_not_parsed`: conditional_on_result or condition with これにより expected
  - Text: `自分の控え室にある、カード名の異なるライブカードを2枚選ぶ。そうした場合、相手はそれらのカードのうち1枚を選ぶ。これにより相手に選ばれたカードを自分の手札に加え`
  - Actual: actions=set()

- **#243** `discard_dest_missing`: destination=discard might be expected
  - Text: `自分のデッキの上からカードを2枚見る。その中から好きな枚数を好きな順番でデッキの上に置き、残りを控え室に置く。`
  - Actual: dests={'deck_top'}

- **#253** `discard_dest_missing`: destination=discard might be expected
  - Text: `自分か相手を選ぶ。自分は、そのプレイヤーのデッキの上からカードを2枚見る。その中から好きな枚数を好きな順番でデッキの上に置き、残りを控え室に置く。`
  - Actual: dests={'deck_top'}

- **#291** `kore_niyori_not_parsed`: conditional_on_result or condition with これにより expected
  - Text: `自分のステージにいる『虹ヶ咲』のメンバー1人につき、自分のデッキの上からカードを1枚見る。その中から1枚までをデッキの上に置き、残りを控え室に置く。その後、自分`
  - Actual: actions=set()

- **#311** `duration_as_long_as_not_parsed`: duration=as_long_as expected
  - Text: `このメンバーをウェイトにしてもよい：カードを1枚引く。その後、このメンバーが『Printemps』のメンバーからバトンタッチして登場していないかぎり、手札を1枚`
  - Actual: duration={'unless'}

- **#315** `discard_dest_missing`: destination=discard might be expected
  - Text: `自分の成功ライブカード置き場にあるカードのスコアの合計が３以上の場合、自分のデッキの上からカードを5枚見る。その中から『μ's』のメンバーカードを1枚公開して手`
  - Actual: dests={'hand'}

- **#329** `discard_dest_missing`: destination=discard might be expected
  - Text: `このメンバーをウェイトにしてもよい：自分のデッキの上からカードを4枚見る。その中から必要ハートの合計が8以上の『Liella!』のライブカードを1枚公開して手札`
  - Actual: dests={'hand'}

- **#357** `discard_dest_missing`: destination=discard might be expected
  - Text: `自分のデッキの上からカードを2枚見る。その中から「朝香果林」のメンバーカードを1枚公開して手札に加えてもよい。残りを控え室に置く。`
  - Actual: dests={'hand'}

- **#359** `discard_dest_missing`: destination=discard might be expected
  - Text: `自分のデッキの上からカードを2枚見る。その中から「近江彼方」のメンバーカードを1枚公開して手札に加えてもよい。残りを控え室に置く。`
  - Actual: dests={'hand'}

- **#362** `discard_dest_missing`: destination=discard might be expected
  - Text: `自分のデッキの上からカードを2枚見る。その中から「天王寺璃奈」のメンバーカードを1枚公開して手札に加えてもよい。残りを控え室に置く。`
  - Actual: dests={'hand'}

- **#365** `discard_dest_missing`: destination=discard might be expected
  - Text: `自分のデッキの上からカードを2枚見る。その中から「鐘嵐珠」のメンバーカードを1枚公開して手札に加えてもよい。残りを控え室に置く。`
  - Actual: dests={'hand'}

- **#395** `discard_dest_missing`: destination=discard might be expected
  - Text: `自分のデッキの上からカードを2枚見る。その中から好きな枚数を好きな順番でデッキの上に置き、残りを控え室に置く。`
  - Actual: dests={'deck_top'}

- **#405** `discard_dest_missing`: destination=discard might be expected
  - Text: `自分のデッキの上からカードを2枚見る。その中から能力を持たない『μ's』のカードか{{jyouji.png|常時}}能力を持つ『μ's』のカードを1枚公開して手`
  - Actual: dests={'hand'}

- **#420** `discard_dest_missing`: destination=discard might be expected
  - Text: `自分のデッキの上からカードを2枚見る。その中から{{heart_02.png|heart02}}と{{heart_04.png|heart04}}と{{hear`
  - Actual: dests={'hand'}

- **#439** `discard_dest_missing`: destination=discard might be expected
  - Text: `自分のデッキの上からカードを5枚見る。その中から『μ's』のライブカードを1枚公開して手札に加えてもよい。残りを控え室に置く。`
  - Actual: dests={'hand'}

- **#440** `kore_niyori_not_parsed`: conditional_on_result or condition with これにより expected
  - Text: `手札のライブカードを1枚公開してもよい：自分の成功ライブカード置き場にあるカードを1枚手札に加える。そうした場合、これにより公開したカードを自分の成功ライブカー`
  - Actual: actions=set()

- **#444** `discard_dest_missing`: destination=discard might be expected
  - Text: `自分のデッキの上からカードを3枚見る。その中から好きな枚数を好きな順番でデッキの上に置き、残りを控え室に置く。`
  - Actual: dests={'deck_top'}

- **#469** `per_unit_not_parsed`: per_unit=True expected
  - Text: `{{icon_energy.png|E}}{{icon_energy.png|E}}支払ってもよい：自分のステージに『虹ヶ咲』のメンバーがいる場合、このカードの`
  - Actual: per_unit=set()

- **#470** `per_unit_not_parsed`: per_unit=True expected
  - Text: `自分のライブ中のカードが3枚以上ある場合、このカードのスコアを＋２する。
(エールをすべて行った後、エールで出た{{icon_draw.png|ドロー}}1つに`
  - Actual: per_unit=set()

- **#471** `per_unit_not_parsed`: per_unit=True expected
  - Text: `ライブの合計スコアが相手より高い場合、自分のエネルギーデッキから、エネルギーカードを1枚ウェイト状態で置く。
(エールで出た{{icon_score.png|ス`
  - Actual: per_unit=set()

- **#475** `per_unit_not_parsed`: per_unit=True expected
  - Text: `自分のエネルギーが12枚以上ある場合、このカードのスコアを＋１する。
(エールをすべて行った後、エールで出た{{icon_draw.png|ドロー}}1つにつき`
  - Actual: per_unit=set()

- **#477** `per_unit_not_parsed`: per_unit=True expected
  - Text: `エールにより公開された自分のカードの中に『蓮ノ空』のメンバーカードが10枚以上ある場合、このカードのスコアを＋１する。
(エールをすべて行った後、エールで出た{`
  - Actual: per_unit=set()

- **#479** `discard_dest_missing`: destination=discard might be expected
  - Text: `自分のデッキの上からカードを5枚見る。その中から『虹ヶ咲』のライブカードを1枚まで公開して手札に加えてもよい。残りを控え室に置く。`
  - Actual: dests={'hand'}

- **#484** `per_unit_not_parsed`: per_unit=True expected
  - Text: `自分のステージにいるメンバーが持つ{{icon_blade.png|ブレード}}の合計が10以上の場合、このカードのスコアを＋１する。
(エールをすべて行った後`
  - Actual: per_unit=set()

- **#489** `discard_dest_missing`: destination=discard might be expected
  - Text: `{{icon_energy.png|E}}支払ってもよい：自分のエネルギーが9枚以上ある場合、自分のデッキの上からカードを5枚見る。その中から1枚を手札に加え、`
  - Actual: dests={'hand'}

- **#490** `per_unit_not_parsed`: per_unit=True expected
  - Text: `自分のエネルギーが9枚以上ある場合、このカードのスコアを＋１する。
(エールをすべて行った後、エールで出た{{icon_draw.png|ドロー}}1つにつき、`
  - Actual: per_unit=set()

- **#507** `discard_dest_missing`: destination=discard might be expected
  - Text: `このメンバーがステージから控え室に置かれたとき、自分のデッキの上からカードを5枚見る。その中からメンバーカードを1枚公開して手札に加えてもよい。残りを控え室に置`
  - Actual: dests={'hand'}

- **#508** `discard_dest_missing`: destination=discard might be expected
  - Text: `このメンバーがステージから控え室に置かれたとき、自分のデッキの上からカードを5枚見る。その中からライブカードを1枚公開して手札に加えてもよい。残りを控え室に置く`
  - Actual: dests={'hand'}

- **#512** `pay_energy_not_parsed`: pay_energy expected for energy payment
  - Text: `自分のメインフェイズの場合、{{icon_energy.png|E}}{{icon_energy.png|E}}支払ってもよい：自分の控え室からライブカードを1`
  - Actual: actions=set()

- **#518** `per_unit_not_parsed`: per_unit=True expected
  - Text: `手札にあるこのメンバーカードのコストは、このカード以外の自分の手札1枚につき、1少なくなる。`
  - Actual: per_unit=set()

- **#628** `kore_niyori_not_parsed`: conditional_on_result or condition with これにより expected
  - Text: `自分のステージに「中須かすみ」がいる場合、自分のデッキの上からカードを4枚公開する。自分はそれらの中から「中須かすみ」のカードを1枚選ぶ。ライブ終了時まで、自分`
  - Actual: actions=set()

- **#663** `kore_niyori_not_parsed`: conditional_on_result or condition with これにより expected
  - Text: `手札を2枚控え室に置いてもよい：自分のデッキの上からカードを5枚見る。その中からメンバーカードを1枚公開して手札に加えてもよい。残りを控え室に置く。これにより『`
  - Actual: actions=set()

- **#677** `discard_dest_missing`: destination=discard might be expected
  - Text: `自分のデッキの上からカードを5枚見る。その中から『Aqours』のライブカードを1枚公開して手札に加えてもよい。残りを控え室に置く。`
  - Actual: dests={'hand'}

- **#684** `discard_dest_missing`: destination=discard might be expected
  - Text: `自分のステージにいる『Aqours』のメンバー1人につき、カードを1枚引く。その後、これにより引いた枚数と同じ枚数を手札から控え室に置く。`
  - Actual: dests={'hand'}

- **#715** `discard_dest_missing`: destination=discard might be expected
  - Text: `控え室から登場している場合、自分のデッキの上からカードを3枚見る。その中から1枚を手札に加え、残りを控え室に置く。`
  - Actual: dests={'hand'}

- **#718** `kore_niyori_not_parsed`: conditional_on_result or condition with これにより expected
  - Text: `自分がエールしたとき、エールにより公開された自分のカードの中からブレードハートを持たない『Aqours』のメンバーカードを1枚まで控え室に置いてもよい。そうした`
  - Actual: actions=set()

- **#735** `kore_niyori_not_parsed`: conditional_on_result or condition with これにより expected
  - Text: `自分がエールしたとき、エールにより公開された自分のブレードハートを持たない『蓮ノ空』のカードを3枚まで控え室に置いてもよい。そうした場合、これにより控え室に置い`
  - Actual: actions=set()

- **#736** `discard_dest_missing`: destination=discard might be expected
  - Text: `このターン、自分が余剰ハートを1つ以上持っている場合、自分のデッキの上からカードを2枚見る。その中から好きな枚数を好きな順番でデッキの上に置き、残りを控え室に置`
  - Actual: dests={'deck_top'}

- **#741** `discard_dest_missing`: destination=discard might be expected
  - Text: `自分のデッキの上からカードを6枚見る。その中からカードを2枚手札に加え、残りを控え室に置く。`
  - Actual: dests={'hand'}

## Summary

- Total: 129
- Errors: 6
- Warnings: 71
- Infos: 52