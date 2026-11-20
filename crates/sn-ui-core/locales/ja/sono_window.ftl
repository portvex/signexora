# The sonogram work window: SA's top toolbar captions, the bottom toolbar hints,
# the right-click popup menu and the Correction of BR panel. SA's control names
# (Fl, Fu, ALC, HRes, Info, WaveFile, DataFile) are names: keep them.

## Top toolbar (SizeFFT, Shear, Image and WinF are in app.ftl).

sw-fl = Fl
    .hover = 表示中の最低周波数、Hz (帯域を拡大中は青)
sw-fu = Fu
    .hover = 表示中の最高周波数、Hz (帯域を拡大中は青)
sw-alc = ALC
sw-hres = HRes
sw-info = Info
    .hover = 結果表示を出します (Fu, Fc, Fl, dF, t1, t2, dT, S, Br, Sc)

## Bottom toolbar hints, left to right.

sw-hint-zoom-in = H マーカー間の帯域をウィンドウ全体に拡大
sw-hint-zoom-back = ズームを 1 段戻す
sw-hint-zoom-reset = ズームをリセット
sw-hint-abort = 中止
sw-hint-play = 再生
sw-hint-stop-play = 再生を停止
sw-hint-band-select = 帯域選択
sw-hint-area-delete = 領域削除
sw-hint-band-reject = 帯域除去
sw-hint-am = 直交 AM 検波器
sw-hint-modified-am = 修正 AM 検波器
sw-hint-half-wave = 両半波検波器
sw-hint-derivative = 1 次微分
sw-hint-power = N 乗モジュール
sw-hint-phase-plane = PhasePlane
sw-hint-acf = 自己相関
sw-hint-ccf = 相互相関
sw-hint-ofdm = OFDM モジュール
sw-hint-phase-detector = 位相検波器
sw-hint-limiter = ハードリミッター
sw-hint-iq = I/Q を実数に変換
sw-hint-shift-base = 帯域をベースへシフト
sw-hint-shift-base-inverted = 帯域をベースへシフト (反転)
sw-hint-inverse = 帯域内反転 (H マーカーが重なっているときはスペクトル全体)
sw-hint-copy = 断片をコピー
sw-hint-resample = 欄のレートにリサンプル
sw-hint-shift = 欄の値だけスペクトルをシフト (Hz、負の値で下へ)
sw-value = R / S
    .hover = R の新しいレート、または S のシフト量 (Hz、小数点はカンマかピリオド)
sw-value-presets = 既定値

## Why a button is disabled.

sw-need-history = 戻るズーム段階がありません。
sw-need-running = このウィンドウで実行中の処理はありません。
sw-need-playing = 再生中ではありません。

## Popup menu.

sw-popup-save-as = 名前を付けて保存
sw-popup-wave-file = WaveFile
sw-popup-data-file = DataFile
sw-popup-wave-file-float = WaveFile (32 ビット浮動小数点)
sw-popup-sigmf = SigMF
sw-popup-waveform = Wave Form モジュール
sw-popup-measure = 周波数の測定
sw-popup-adp = パラメーター自動決定
sw-popup-brc = BR の補正
sw-popup-cancel = キャンセル

## Correction of BR panel.

sw-brc-existing = 現在のボーレート
sw-brc-required = 必要なボーレート
sw-brc-max-delta = 最大変化量: +/- 30%
sw-brc-yes = はい

## Messages.

sw-check-settings = 設定を確認してください。
# Result boxes in the plot (SA's own text). $value: a number with two decimals.
sw-label-freq = Freq: { $value } Hz
sw-label-brate = Brate: { $value } Hz
sw-rs-resample-invalid = R/S の値 "{ $text }" は正のサンプルレートではありません。8000 のように Hz で入力してください。
sw-rs-shift-invalid = R/S の値 "{ $text }" は数値ではありません。-150 のように Hz でシフト量を入力してください。
sw-brc-existing-invalid = Existent Baud Rate "{ $text }" は数値ではありません。測定した Br を Bd で入力するか、先に ADP を実行してください。
sw-brc-required-invalid = Required Baud Rate "{ $text }" は数値ではありません。必要な Br を Bd で入力してください。
sw-brc-not-positive = 両方のボーレートは 0 Bd より大きくなければなりません。
sw-brc-too-far = Required Br は { $lo } から { $hi } Bd の間にしてください (既存値から 30% 以内)。
sw-brc-measured = ADP で測定した Br を入力済み
sw-brc-not-measured = Br はまだ測定されていません。入力するか先に ADP を実行してください
