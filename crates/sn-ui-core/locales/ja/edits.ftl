# The Signal menu, the parameter dialog of its operations and the save formats.

edits-save-wav-pcm16 = WAV、16 ビット PCM
edits-save-wav-float = WAV、32 ビット浮動小数点
edits-save-sigmf = SigMF (小数のレートを保持)
edits-save-raw-f32 = Raw、32 ビット浮動小数点

# $from: where a measured value came from, e.g. "n4, measure line".
edits-measured-other-axis = { $from } (別の軸、要確認)

## Notes under the dialog fields: where a pre-filled value came from.

edits-note-cutoff-raised = カットオフ: { $from } の Br (最小値 30 Hz に引き上げ。Br は { $br } Bd)
edits-note-cutoff-lowered = カットオフ: { $from } の Br (サンプルレートの半分未満に引き下げ。Br は { $br } Bd)
edits-note-cutoff-br = カットオフ: { $from } の Br
edits-note-cutoff-guessed = カットオフ: 推定値 (fs/8)。先に Br を測定してください
# $count: the longest window offered, in samples.
edits-note-window-capped = 窓: { $from } の Br で 1/3 シンボル ({ $count } サンプルで制限。正確な 1/3 シンボルにはダウンリサンプルしてください)
edits-note-window-br = 窓: { $from } の Br で 1/3 シンボル
edits-note-window-default = 窓: 既定の 5 サンプル。Br を測定すると 1/3 シンボルになります
edits-note-measured-br = { $from } で測定した Br
edits-note-measured-s = 測定値: S = V マーカーの 1/dT
edits-note-carrier-sc = 搬送波: { $from } の Sc
edits-note-carrier-h = 搬送波: H マーカーの中心
edits-note-carrier-guessed = 搬送波: 推定値 (fs/4)。先に Sc を測定してください
edits-note-br-from = Br: { $from }
edits-note-br-guessed-8 = Br: 推定値 (fs/8)。先に Br を測定してください
edits-note-br-s = Br: S = V マーカーの 1/dT
edits-note-br-guessed-10 = Br: 推定値 (fs/10)。先に Br を測定してください

## Why a menu item is disabled.

edits-need-h = 先に H マーカーを設定してください (ソノグラム上でドラッグ)。
edits-need-v = 先に V マーカーを設定してください (ソノグラムまたは波形上でドラッグ)。
edits-need-vh = 先に V マーカーと H マーカーを設定してください (ソノグラム上でドラッグ)。
edits-need-complex = この信号はすでに実数です。
edits-need-baud = 先に Br を測定する (測定メニュー) か、1 シンボルに V マーカーを設定してください。

## Signal menu sections and items.

edits-section-spectrum = スペクトル (H マーカー)
edits-band-select = 帯域選択
edits-band-reject = 帯域除去
edits-area-delete = 領域削除
edits-shift-to-base = 帯域をベースへシフト
edits-shift-to-base-inverted = 帯域をベースへシフト (反転)
edits-invert-band = 帯域内反転
edits-section-whole = 信号全体
edits-shift = スペクトルをシフト...
edits-invert = スペクトル反転
edits-resample = リサンプル...
edits-iq-to-real = I/Q を実数に変換
edits-baud-correction = ボーレート補正...
edits-derivative = 1 次微分
edits-limiter = ハードリミッター
edits-section-isd = ISD 補正器
edits-isd-am = ISD 補正器、AM モード (ブラインド)...
edits-isd-xn = ISD 補正器、X^N モード...
edits-section-detectors = 検波器
edits-am-detector = AM 検波器 (PSK/QAM/FSK のクロック)
edits-modified-am = 修正 AM 検波器 (MFSK のクロック)
edits-half-wave = 両半波検波器
edits-phase-detector = 位相検波器 (周波数 Hz)
edits-power = N 乗...
edits-acf = 選択範囲の ACF
edits-ccf = CCF: 選択範囲を全体から検索
edits-section-time = 時間 (V マーカー)
edits-copy = 選択範囲を新しい信号にコピー
edits-section-filters = フィルター
edits-lowpass = ローパス...
edits-median = メディアン...
edits-rrc = RRC (整合フィルター)...

## パラメーターのダイアログ (SA の小さなモーダルダイアログ)。

# $operation: the dialog's operation; $signal: the window name, e.g. "test(0)(u1)".
edits-dialog-title = SA - { $operation }: { $signal }
edits-title-shift = スペクトルをシフト
edits-title-resample = リサンプル
edits-title-baud = BR の補正
edits-title-lowpass = ローパス
edits-title-median = メディアンフィルター
edits-title-rrc = RRC フィルター
edits-title-power = N 乗
edits-title-isd-am = ISD 補正器 (AM モード)
edits-title-isd-xn = ISD 補正器 (X^N モード)

edits-field-carrier = 搬送波 (Hz)
edits-field-br = Br、シンボルレート (Bd)
edits-field-blocks = ブロック数
edits-field-power-psk = N 乗 (PSK の次数)
edits-field-window = 窓 (サンプル数、奇数)
edits-field-shift = シフト (Hz、+ で上へ)
edits-field-rate = 新しいサンプルレート (Hz)
edits-field-measured-br = 現在のボーレート
edits-field-required-br = 必要なボーレート
edits-field-cutoff = カットオフ (Hz)
edits-presets = プリセット
edits-field-power = N 乗
edits-field-alpha = ロールオフ alpha
# $lo, $hi: the limits of a field.
edits-range = { $lo } から { $hi } まで
# $max: the sample rate in Hz.
edits-hint-shift = +/- { $max } Hz まで、0 以外、小数も可
# $max: the longest window, in samples.
edits-hint-window = 1 から { $max } まで、偶数は次の奇数として使います
edits-ok = OK
edits-cancel = キャンセル
edits-field-invalid = { $field } "{ $text }" は無効です。{ $range }。
edits-range-open = { $lo } より大きく { $hi } 未満
