# OFDM module window, SA-OFDM (SA_UI_SPEC.md section 10).

# Top bar (SA's control names).
ofdm-min-br = Min Br-LS
    .hover = 探索するシンボルレートの下限、Bd (整数)。隣の欄は最長の LS
ofdm-max-br = Max Br-LS
    .hover = 探索するシンボルレートの上限、Bd (整数)。隣の欄は最短の LS
ofdm-total-ls = Total LS
    .hover = 選択範囲に入る最長 (Min Br-LS) のシンボル数
ofdm-current-ls = Current LS
    .hover = 表示中のシンボル (Find CT 中は 0)
ofdm-survey = Survey
    .hover = 概観: 下のバーで動かす 1024 点のスペクトル窓が Find CT の開始位置を決める
ofdm-list-k = List of K
    .hover = Magic K 計算: 最も確からしい K の値と本来のサンプリングレート
    .disabled = 先に Find CT と ADP を実行してください

# Controls (SA's control names).
ofdm-symbols = Symbols
    .hover = 相関三角形 (と ADP のブロック) に重ねるシンボル数。20 から 40 が良く、6 未満は不可
ofdm-scale-fft = Scale FFT
    .hover = LU スペクトルのチャネル棒の幅
ofdm-shift = Shift Frequency
    .hover = FFT 前に差し引く周波数 (Hz)。Enter で適用
ofdm-vshift = VShift
    .hover = 1 チャネル分の仮想シフト (格子の正確な刻み、制限なし)
ofdm-shift-step = 刻み
    .hover = Shift Frequency を { $step } Hz 増減
ofdm-br = Br
    .hover = シンボルレート: ADP の値、または入力して Enter
ofdm-adp = ADP
    .hover = 正確なシンボルレートと周波数位置 (SA の Get Br)
    .disabled = 先に Find CT を実行してください
ofdm-show = Show
    .hover = 印を付けたチャネルをシンボル同期付きで 1 シンボルずつ表示 (Break で停止)
ofdm-break = Break
ofdm-find-ct = Find CT
    .hover = 相関三角形を探す: LS、LG とシンボル開始位置
ofdm-mode = Mode
    .hover = コンスタレーションの信号モデル
ofdm-model-a = A
    .hover = モデル A: 各チャネルは生成されたまま (絶対値)
ofdm-model-b = B
    .hover = モデル B: 各チャネルが自身のパイロット (相対値)
ofdm-model-c = C
    .hover = モデル C: 赤マーカーのパイロットを基準とするデータ
ofdm-each = Each
    .hover = N シンボルごとに表示
ofdm-start-plus = Start+
    .hover = Show で最初に表示するシンボル

# Plot texts (SA's formats).
ofdm-info-br = Br = { $br }
ofdm-info-sh = Sh = { $sh }
ofdm-info-magic = Magic K = { $num }/{ $den }
ofdm-info-k = * { $k } *
ofdm-info-lg = LG(CP) = { $lg }
ofdm-info-lu = LU = { $lu }
ofdm-info-up = CH_Up = { $ch }, Freq = { $freq }
ofdm-info-down = CH_Down = { $ch }, Freq = { $freq }
ofdm-info-sel = CH_Sel = { $count }, dFreqs = { $df }
ofdm-info-vshift = VShift = { $bins }
ofdm-line = LS={ $ls }; LG={ $lg }; Br={ $br }; Sh={ $sh }
ofdm-k-line = SR={ $sr }; LS={ $ls }; LU={ $lu }; LG={ $lg }; K={ $num }/{ $den }
ofdm-finish = 正常終了、時間 { $secs } 秒。
ofdm-stopped = ユーザーが停止、時間 { $secs } 秒。
ofdm-wait = あわてないで! お待ちください...
ofdm-wait-ct = 相関三角形を探索中
ofdm-wait-adp = 正確な Br を計算中
ofdm-wait-k = "Magic K" を計算中
ofdm-wait-left = 残り時間: { $secs } 秒

# Plots.
ofdm-survey-plot = 概観スペクトル
ofdm-triangle-plot = 相関三角形
ofdm-triangle = 三角形
    .hover = 良い (左の) 斜面をクリックして LU 窓を置く。頂点は親 LU
ofdm-spectrum-plot = LU スペクトル
ofdm-spectrum = スペクトル
    .hover = チャネルをクリックで近い方のマーカーを移動、ダブルクリックで両方を置く
ofdm-pane = チャネル
    .hover = チャネル { $ch }、シンボル数 { $count }
ofdm-k = K
    .hover = K = { $k }、LU 誤差 { $err }、ICI { $ici } dB、{ $lu }
ofdm-k-lu-even = 偶数 LU
ofdm-k-lu-odd = 奇数 LU

# Popup menus.
ofdm-sort-sr = SR で並べ替え
ofdm-sort-k = K で並べ替え
ofdm-sort-am = AM で並べ替え
ofdm-k-close = 閉じる
ofdm-menu-cancel = キャンセル
ofdm-menu-reset = 結果を消去
ofdm-menu-around = Br = { $rate } の前後を範囲に
    .hover = Min Br-LS と Max Br-LS を測定した Br の 0,8 倍と 1,25 倍に設定
ofdm-resample = { $rate } Hz にリサンプル
    .hover = 信号をこの本来のレートにリサンプル (新しいウィンドウ)

# Jobs and messages.
ofdm-job-ct = OFDM: Find CT
ofdm-job-br = OFDM: ADP
ofdm-job-k = OFDM: List of K
ofdm-job-show = OFDM: Show
ofdm-needs-ct = 先に Find CT を実行してください。
ofdm-needs-br = 先に ADP を実行してください。
ofdm-baseband-freed = その間にウィンドウが閉じられました。Find CT をもう一度実行してください。
ofdm-check-settings = 設定を確認してください! Show には Symbols が 6 以上、Start+ の後に 10 シンボル以上が必要です。
ofdm-failed = 検索に失敗しました。時間 { $secs } 秒。
ofdm-pick-busy = 実行中の処理を待つか Break を押してから行を選んでください。
ofdm-br-invalid = Br "{ $text }" は使えません。0 より大きく { $max } Bd (fs/LU) 以下の数を入力してください。
ofdm-br-invalid-nogrid = Br "{ $text }" は使えません。0 より大きい数を入力してください。
ofdm-plot-empty = データがありません。Find CT を実行してください。
ofdm-shift-invalid = Shift Frequency "{ $text }" は数値ではありません。12.5 のように Hz で入力してください。
