# The WF (waveform) window: SA's top toolbar, bottom toolbar hints, popup menu and
# result box. SA's control names (LPF, RRC, Sum, Info, VMW, Br, Z+, Z-) and the
# result box text are names: keep them.

## Top toolbar, left to right.

wf-lpf = LPF
    .hover = ローパスの遮断周波数 (Hz)、または RRC フィルタのボーレート (入力または選択)
wf-copy-br = <==
    .hover = 測定した Br を LPF 欄にコピー
wf-rrc = RRC
    .hover = RRC フィルタのオンとオフ (オンのとき LPF ボタンで実行)
wf-rrc-alpha = RRC
    .hover = RRC のロールオフ
wf-freq = 0-0
    .hover = クリック: V マーカー間の短い断片の周波数と周期数
wf-vmw = VMW
    .hover = VMW に切り替え (信号をライン周期で折り返す)
wf-sum = Sum
    .hover = 値のヒストグラムを表示 (その後ポップアップの周波数を数える)
wf-info = Info
    .hover = 結果を表示 (t1, t2, dT, S, Br)

## Bottom toolbar hints, left to right.

wf-hint-zoom-in = 拡大 (時間)
wf-hint-zoom-out = 縮小 (時間)
wf-hint-scale-up = 縦方向の目盛りを大きく
wf-hint-scale-down = 縦方向の目盛りを小さく
wf-hint-zoom-plus = Z+: V マーカー間を拡大
wf-hint-zoom-minus = Z-: ズームを一段戻す
wf-hint-lpf = LPF フィルタを実行 (RRC がオンなら RRC フィルタ)
wf-hint-revert = 元に戻す
wf-hint-synth = FSK/MFSK の逆合成
wf-hint-save = 16 ビット WAV で保存 (右クリックで float WAV、DataFile、SigMF)
wf-hint-copy = V マーカー間の断片を新しいウィンドウにコピー
wf-hint-demod = 断片を FSK/MFSK 復調器に送る
wf-hint-fit = 縦方向の目盛りを信号に合わせる
wf-hint-acf = 断片の自己相関 (新しいウィンドウ)
wf-hint-ccf = 断片と信号全体の相互相関 (ここに描画)

## Why a button is disabled.

wf-need-revert = 元に戻すものはありません (フィルタも CCF もなし)。
wf-need-derived = このウィンドウは保存済みの開いたファイルを表示しています。保存はここで作った結果用です。

## Popup menu.

wf-popup-count-freqs = 周波数を数える
wf-popup-more = SigNexora のツール
    .hover = SA の WF ウィンドウになかった SigNexora のツール
wf-popup-operations = 操作

## The Count freqs result box (SA's text, not translated). $size: a whole number;
## $value: a number with two decimals.

wf-label-grid = Freq grid: { $size }
wf-label-delta = Delta: { $value } Hz
wf-count-needs-h = 先にトーンを囲むように H マーカーを置いてください。
wf-count-tones-busy = Find tones がまだ実行中です。終了後に Count freqs をもう一度実行してください。
wf-count-freqs-failed = 周波数グリッドが見つかりません ({ $error })。トーンを囲む H マーカーを広げるか、ズームアウトしてより多くのトーンを表示してください。
wf-copy-br-value = Br = { $br } Bd を LPF 欄にコピー
wf-copy-br-missing = 先に Br を測定してください (ソノグラムのメニュー: Auto define params)
