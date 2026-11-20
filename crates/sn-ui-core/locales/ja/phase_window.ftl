# The PhasePlane form (SA_UI_SPEC.md section 6): spectrum mode toolbar, ISD
# corrector, phase viewer, status bar, demodulator output and the LoadFile dialog.
# The control captions are SA's own English names (SA showed them in English under
# every Windows language): keep them as they are; translate the hints and dialogs.

pp-fmin = Fmin
    .hover = 表示中の最低周波数 (Hz)
pp-fmax = Fmax
    .hover = 表示中の最高周波数 (Hz)
pp-scale = Scale
    .hover = 表示する帯域の割合。スペクトルの下のスクロールバーで移動します
pp-fft-size = FFTSize
    .hover = 平均スペクトルの FFT サイズ。大きいほど細かくなります (約 1 Hz まで)
pp-low-level = LowLevel
    .hover = 表示する最低レベル
pp-stretch = Stretch
    .hover = スペクトルを高さいっぱいに伸ばす
pp-mode = Mode
    .hover = AM: 包絡線で補正、X^N: 信号の N 乗で補正
pp-blocks = Blocks
    .hover = 補正器が平均する信号ブロック数
pp-corr-off = Corr-OFF
    .hover = ISD 補正器はオフです。Auto を押して信号を補正します
pp-corr-on = Corr-ON
    .hover = ISD 補正器はオンです (SaveAs で補正済み信号を保存)
pp-save-as = SaveAs
    .hover = 補正した信号をファイルに保存
pp-auto = Auto
    .hover = 選んだブロックで補正を 1 段階実行
pp-break = Break
pp-reset = Reset
    .hover = 補正器をリセット (オフにする)
pp-carrier = Carrier
    .hover = 搬送波周波数、Hz (最後の Sc 測定値)
pp-br = BR
    .hover = 変調速度、Bd (最後の Br 測定値)
pp-nary = n-Ary
    .hover = 変調の次数、2-32 (1: 搬送波追従なし)
pp-synch-br = Synch BR
    .hover = クロック同期
pp-nonstop = NonStop
    .hover = 連続動作: ファイルの終わりで最初から
pp-rrc = RRC filter
    .hover = ルートレイズドコサインフィルター。スライダーで alpha を設定
pp-pll = PLL
    .hover = キャリアループの速度
pp-pll-slow = Slow
pp-pll-medium = Medium
pp-pll-quick = Quick
pp-pll-hard = Hard
pp-auto-reset = AutoReset
    .hover = NonStop 時: ファイルの終わりで発振器と PLL ループをリセット (ドリフトする信号用)
pp-transitions = Transitions
    .hover = 点の間の遷移 (軌跡) を描く
pp-life = Life
    .hover = 動的表示: 最後の Points... 個のシンボルだけを残す
pp-diff = Diff
    .hover = 0: 絶対角、1-3: 位相差 (pi/4 DQPSK などの相対方式)
pp-offset-detected = Offset mode detected
pp-angles = Angles
    .hover = 補正器の表示に角度ヒストグラムを作成 (Synch BR が必要)
pp-vectors = Vectors of dem
    .hover = 復調器の位置を表示。ゼロベクトルは水色
pp-output-dem = Output Dem as
    .hover = 復調したストリームを絶対角のコードでテキスト欄に出力
pp-trimming = Trimming { $value }
    .hover = 位相の微調整、度
pp-delay = Delay { $value }
    .hover = シンボルごとの実遅延、秒 (非常に短い信号用)
pp-points = Points... { $value }
    .hover = Life オン時に同時に表示する点数
pp-contrast = Contrast { $value }
    .hover = 点の明るさ
pp-start = Start
    .hover = コンスタレーションを開始または停止。停止後はファイルの先頭から再開
pp-clear = Clear
    .hover = 表示を消去
pp-load-file = LoadFile
    .hover = 開いているファイルの一つを位相平面に読み込む
pp-status-f = F = { $value }
pp-status-fl = Fl = { $value }
pp-status-fu = Fu = { $value }
pp-status-df = dF = { $value }
pp-status-angle-d = Angle_D = { $value }
pp-status-angle-i = Angle_I = { $value }
pp-dem-view-mode = View mode: diff { $diff }
pp-dem-mode = Dem mode: { $mode }
pp-dem-absolute = Data presented as absolute angles!
pp-dem-coding = The coding positions of angles:
pp-dem-data = Data:

pp-isd-n = 補正器が使う信号の高調波
    .hover = 補正器が使う信号の高調波
pp-isd-b = オン: 改良アルゴリズム、オフ: やや劣るが高速 (AM モードでは無効)
    .hover = オン: 改良アルゴリズム、オフ: やや劣るが高速 (AM モードでは無効)
pp-isd-break = 現在の段階を中止
    .hover = 現在の段階を中止
pp-alpha = RRC フィルターのロールオフ alpha
    .hover = RRC フィルターのロールオフ alpha
pp-quality = コンスタレーションの規約上の SN 比、dB
    .hover = コンスタレーションの規約上の SN 比、dB
pp-load-title = SA - PhasePlane のファイルを選択
pp-ok = OK
pp-cancel = キャンセル
pp-check-settings = 設定を確認してください
pp-run-limited = 最初の { $symbols } シンボルのみ復調されます。後の部分を解析するには、ソノグラムでその部分に V マーカーを置き、{ sw-hint-copy } を使い、コピーで { sw-hint-phase-plane } を開いてください。
pp-copy-panel = テキストをコピー
pp-copy-codes = Start 以降の全コードをコピー
pp-eye = アイパターン
    .hover = コンスタレーションの代わりにアイパターン (I、2 シンボル) を表示
pp-job-corrector = ISD 補正器のスペクトル
pp-job-isd = ISD 補正器
pp-job-spectrum = PhasePlane スペクトル
# $count symbols, $offset Hz, $rings radii, $iq_offset symbols.
pp-quality-details = { $count } シンボル、ループのずれ { $offset } Hz、リング [{ $rings }]、I/Q のずれ { $iq_offset } シンボル
pp-source-gone = 読み込んだファイルはもうセッションにありません。LoadFile で作業ファイルを読み込み直してください。
pp-load-gone = 選んだファイルは閉じられました。別の作業ファイルを選んでください。
