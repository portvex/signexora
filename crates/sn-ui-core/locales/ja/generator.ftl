# File > Signal generator (OCG): the generator window, its preview and the
# OFDM calculators. Keep units (Hz, Bd, s, ms, dB, ppm), modulation names
# (FSK, MSK, GMSK, PSK, QAM, C4FM, OFDM, BPSK, ...), BT, Es/N0, SNR, RC, RRC,
# LU, LG, LS, K, Br, Fd and the model letters A, B, C untranslated.

gen-menu = 信号ジェネレーター (OCG)...
    .hover = パラメーターが正確にわかっているテスト信号を作成します: トーン、ノイズ、FSK、PSK/QAM、C4FM、OFDM、チャネル劣化付き
gen-keys-open = 信号ジェネレーターの表示/非表示
gen-title = 信号ジェネレーター (OCG)
gen-job = { $name } を生成中

gen-kind = 種類
gen-kind-tones = トーン
gen-kind-noise = ノイズ
gen-kind-fsk = FSK/MFSK
gen-kind-linear = PSK/QAM
gen-kind-c4fm = C4FM
gen-kind-ofdm = OFDM
gen-preset = プリセット
    .hover = 用意された信号を読み込みます
gen-preset-defaults = 既定値
    .hover = 現在の種類を既定値に戻します

gen-section-signal = 信号
gen-section-common = 共通
gen-section-channel = チャネル劣化
gen-section-derived = 派生値
gen-section-preview = プレビュー
gen-section-calc = 計算ツール

gen-complex = 複素 (I/Q) 出力
    .hover = 実数信号の代わりに I/Q サンプルを作成します。周波数は 0 Hz からのオフセットになります
gen-amplitude-rms = RMS 振幅
gen-amplitude-peak = ピーク振幅
gen-phase-rad = 位相 (rad)
gen-remove = 削除
gen-baud = ボーレート
gen-carrier = 搬送波
gen-center = 中心
gen-alpha = ロールオフ

gen-tones-frequency = 周波数
gen-tones-add = トーンを追加
gen-tones-last = トーンが 1 つ以上必要です

gen-fsk-order = トーン数
gen-fsk-spacing = 間隔
gen-fsk-shaping = 整形
gen-fsk-shaping-none = なし
gen-fsk-shaping-gaussian = ガウス
gen-fsk-bt = BT
gen-fsk-msk = MSK
    .hover = 2 トーン、間隔はボーレートの半分、整形なし
gen-fsk-gmsk = GMSK
    .hover = BT 0.3 のガウスフィルター付き MSK (GSM と同じ)

gen-lin-constellation = コンスタレーション
gen-lin-shaping = パルス形状
gen-lin-rect = 矩形
gen-lin-rc = レイズドコサイン (RC)
gen-lin-rrc = ルートレイズドコサイン (RRC)
gen-lin-differential = 差動
    .hover = 各シンボルを前のシンボルからの位相変化として送ります
gen-lin-differential-qam = QAM は位相だけでなく振幅も運ぶため、差動にできません
gen-lin-differential-pi4 = pi/4-DQPSK は常に差動です

gen-c4fm-pulse = パルス
gen-c4fm-shaped = 整形
    .hover = オフ: 周波数が 4 つのレベル間で跳びます
gen-c4fm-apco = APCO-25
    .hover = TIA-102 のパルス。積分ダンプ受信機向け
gen-c4fm-note = 偏移は +-600 Hz と +-1800 Hz に固定です。C4FM の出力は常に実数です。

gen-ofdm-lu = LU
    .hover = サンプルレートでの有効長 (FFT サイズ、サンプル数)
gen-ofdm-lg = LG
    .hover = ガード (サイクリックプレフィックス) 長 (サンプル数)
gen-ofdm-channels = チャネル数
gen-ofdm-first-bin = 最初のビン
    .hover = 最初のチャネルのビン。ビン 0 が中心です
gen-ofdm-modulation = 変調
gen-ofdm-unused = 未使用位置
    .hover = 何も送らないチャネル位置 (0 から数える)。例: 3, 7, 10-12
gen-ofdm-overrides = 個別指定
    .hover = 個々の位置の変調。例: 0: pilot, 5-6: QAM-16
gen-ofdm-model = モデル
gen-ofdm-model-a = A: 全チャネルが絶対値を送る
gen-ofdm-model-b = B: 全チャネルがシンボル間で差動
gen-ofdm-model-c = C: パイロットが基準を運び、データはそれに対する絶対値
gen-ofdm-pilots = パイロット
gen-ofdm-pilots-none = なし
gen-ofdm-pilots-list = 位置
    .hover = パイロット位置。例: 0, 8, 16
gen-ofdm-pilots-every = N ごと
gen-ofdm-step = 間隔
gen-ofdm-offset = 開始
gen-ofdm-reference = パイロット基準
    .hover = モデル C でパイロットが運ぶもの
    .disabled = パイロット基準を使うのはモデル C のみです
gen-ofdm-reference-constant = 一定
gen-ofdm-reference-differential = ランダムな共通位相
gen-ofdm-center = 中心
    .hover = ビン 0 の位置。複素出力では 0 Hz からのオフセット

gen-label = ラベル
    .hover = 新しい信号の名前。空欄なら説明を使います
gen-sample-rate = サンプルレート
gen-length = 長さ
gen-length-seconds = 秒
gen-length-symbols = シンボル
    .disabled = トーンとノイズにはシンボルがありません
gen-seed = シード
gen-new-seed = 新しいシード
    .hover = ランダムなシードを選びます: 新しいデータと新しいノイズ
gen-center-frequency = 中心周波数
    .hover = 複素出力の 0 Hz に相当する RF 周波数 (信号と共に保存)
    .disabled = 中心周波数があるのは複素出力のみです

gen-ch-offset = 周波数オフセット
gen-ch-drift = ドリフト
gen-ch-clock = クロックオフセット
    .hover = サンプルクロックの誤差。正の値は受信側のクロックが速いことを意味します
gen-ch-noise = ノイズ
gen-ch-noise-none = なし
gen-ch-noise-full = 全帯域の SNR
gen-ch-noise-inband = 帯域内 SNR
gen-ch-noise-esn0 = Es/N0
gen-ch-bandwidth = 帯域幅
    .hover = オフ: 信号の占有帯域幅
gen-ch-clip = クリップ
    .hover = ピーク振幅を制限します。複素サンプルは位相を保ちます
gen-ch-multipath = マルチパスタップ
    .hover = タップがチャネル全体です: 直接波 (遅延 0、利得 1) も含めてください
gen-ch-delay = 遅延
gen-ch-gain = 利得
gen-ch-add-tap = タップを追加

gen-d-none = パラメーターを修正すると派生値が表示されます。
gen-d-symbol-rate = シンボルレート
gen-d-sps = シンボルあたりのサンプル数
gen-d-symbols = シンボル数
gen-d-bandwidth = 占有帯域幅
gen-d-samples = サンプル数
gen-d-duration = 長さ
gen-d-index = 変調指数 h
gen-d-ls = LS
gen-d-k = K = LG/LU
gen-d-spacing = 間隔
gen-d-br = Br

gen-preview = ジェネレーターのプレビュー
gen-preview-spectrum = スペクトル
gen-preview-wave = 波形
gen-preview-name = 生成信号のプレビュー
gen-preview-building = プレビューを作成中...
gen-preview-invalid = パラメーターが有効になるまでプレビューはありません
gen-preview-failed = プレビューに失敗しました: { $error }
gen-preview-short = スペクトルには短すぎます
gen-preview-axis = 平均スペクトル、{ $lo } から { $hi } Hz、範囲 80 dB
gen-preview-wave-axis = 最初の { $ms } ms、ピーク { $peak }

gen-generate = 生成
    .hover = 信号を新しい録音としてセッションに追加します
gen-generate-save = 生成して保存
    .hover = 生成後、新しい信号を選択した形式で保存します
gen-busy = 信号を生成中です
gen-fix-first = 先にパラメーターを修正してください
gen-invalid = 生成できません: { $error }
gen-field-error = { $field }: { $error }
gen-ready = 準備完了: { $samples } サンプル、{ $duration } s
gen-list-bad = "{ $item }" は位置でも 10-12 のような範囲でもありません
gen-list-too-long = 位置が多すぎます (最大 { $max })
gen-overrides-bad = "{ $item }" は 5: QAM-16 のような位置と変調の組ではありません

gen-calc-cp = CP 付き OFDM
gen-calc-cp-lengths = Fd、LU、LG から
gen-calc-spacing = 間隔
gen-calc-symbol = シンボル
gen-calc-useful = 有効部分
gen-calc-guard = ガード
gen-calc-use = OFDM に適用
    .hover = これらの長さ (と Fd) を OFDM エディターにコピーします
    .disabled = LU と LG はサンプル数の整数でなければなりません
# Why Use in OFDM is disabled. $max: the generator's limit in samples.
gen-calc-lu-not-whole = LU は 2 以上の整数のサンプル数でなければなりません。
gen-calc-lg-not-whole = LG は整数のサンプル数でなければなりません。
gen-calc-lu-too-large = LU は { $max } サンプル以下でなければなりません。
gen-calc-lg-too-large = LG は { $max } サンプル以下でなければなりません。
gen-calc-use-k = OFDM に適用
    .hover = この K を OFDM エディターにコピーします: 現在の LU 以上で K に合う最小の LU
gen-calc-inverse = Fd での Br と間隔から: 最も近い整数の LU と LG
gen-calc-exact = 正確
gen-calc-error = 誤差
gen-calc-candidates = Br と K に対する整数の LU と LG
gen-calc-lu-range = LU の開始値
gen-calc-tolerance = 許容誤差
gen-calc-found = { $count ->
   *[other] { $count } 件の候補、最初の { $shown } 件を表示
}
gen-calc-klist = K の一覧
gen-calc-k-from-rates = Br と間隔から
gen-calc-k-direct = K から
gen-calc-uncertainty = 不確かさ
    .hover = 測定した各レートの誤差の大きさ (レートに対するパーセント)。リストには、そこから求まる K の不確かさの範囲内で最も簡単な分数を表示します。K を直接入力した場合は、この誤差を持つ 2 つのレートから測定した K の許容誤差を使います。
gen-calc-uncertainty-bad = 不確かさは 0 から { $max } % の範囲で指定してください。
gen-calc-k-none = 不確かさの範囲内に分数がありません。不確かさまたは最大の分母を大きくしてください。
gen-calc-max-den = 最大の分母
gen-calc-count = 個数
gen-calc-value = 値
gen-calc-fd = Fd
    .hover = この Br でこれらの長さが正確になるサンプルレート (Hz): Br x (LU + LG)
gen-calc-k = K
    .hover = K = LG/LU、これらの整数の長さが実際に持つガード長と有効長の比
gen-calc-lg-error = 誤差 (サンプル)
    .hover = LG - K x LU: 整数のガード長が正確な値からどれだけ離れているか (サンプル単位)
gen-calc-k-fraction = K
    .hover = 既約分数 p/q で表した K
gen-calc-k-error = 誤差
    .hover = 分数の値から測定した K を引いた値 (符号付き。K に単位がないため、この値にも単位はありません)
gen-preview-symbol-too-long = プレビューなし: 1 シンボルが { $samples } サンプルでプレビューより長いです。ボーレートを上げるかサンプルレートを下げてください。
gen-calc-fraction-too-big = K = { $k } は収まりません。LU は { $lu } 以下、LG は { $lg } サンプル以下にしてください。より小さい LU か K を選んでください。
