# FSK-2 / 汎用 MFSK 復調器ウィンドウ (SA_UI_SPEC.md 第 8 節)。

# Text panel.
demod-heading =
    SA - MFSK dem
    Levels: { $levels }
    Fd: { $fs }
    Br: { $br }
    Data as: { $form }
    Data:
demod-as-default = 標準の対応表
demod-as-raw = 生データ
demod-as-bits = ビット (自然符号)
demod-as-gray = ビット (グレイ符号)
demod-as-c4fm = APCO-25 C4FM ダイビット
demod-polarity-inverted = [極性: 反転、反転 C4FM 表でビット化]
demod-polarity-normal = [極性: 正常]
demod-polarity-unknown = [極性: 不明、通常の C4FM 表でビット化]
demod-truncated = (先頭 20000 文字。コピーと名前を付けて保存は全体を対象)
demod-nothing-yet = まだ何も復調されていません。Br、レベル、マーカーを確認してください。

# Jobs.
demod-job-track = 復調器: 周波数軌跡
demod-job = FSK/MFSK を復調中

# Top bar.
demod-rrc = RRC
    .hover = 判定前のルートレイズドコサインフィルタ。欄はそのカットオフ (Hz、既定は Br)
demod-cutoff = カットオフ
    .hover = RRC のカットオフ (Hz)。値を入力するか一覧から選択
demod-alpha = アルファ
    .hover = RRC のロールオフ、0,05 から 1 (Enter か欄を離れると適用)
demod-br = Br
    .hover = 変調速度。Enter で適用
demod-br-step = 刻み
    .hover = Br を { $step } 増減
demod-auto = A
    .hover = 位相とビットの自動同期
demod-auto-button = A
    .hover = 今すぐ位相を自動で求めて固定 (自動モードは解除)
demod-exactly = <= Exactly
    .hover = 細かいサンプリング位相 (1 シンボルの +-{ $span } %)
demod-roughly = <= Roughly
    .hover = 1 シンボル内の粗いサンプリング位相
demod-levels = Levels
    .hover = レベル数、2 から 256
demod-raw = Raw
    .hover = レベルを x 区切りの 16 進バイトで出力

# Bottom bar.
demod-back-fast = 10 戻る
    .hover = 10 ページ戻る
demod-back = 戻る
    .hover = 前のページ
demod-forward = 進む
    .hover = 次のページ
demod-forward-fast = 10 進む
    .hover = 10 ページ進む
demod-page = ページ
    .hover = 表示中のページ (全 { $count })。Enter で移動
demod-scale-up = 拡大
    .hover = 縦の倍率を上げる
demod-scale-down = 縮小
    .hover = 縦の倍率を下げる
demod-progress = 進行
    .hover = 計算の進行状況 (右クリックで停止)
demod-stop = 停止
demod-filter = フィルタ
    .hover = RRC フィルタを適用
demod-revert = 元に戻す
    .hover = フィルタなしの信号に戻す
demod-d = D
    .hover = ビット列をテキスト欄に出力

# Plot.
demod-plot-name = 復調器のレベル
demod-info = Br = { $baud } | クロック { $clock } % | 品質 { $quality } | アイ { $eye } | シンボル数 { $count }
demod-menu-tones = 検出したトーンをレベルに ({ $count })
    .hover = 等間隔の区間の代わりに、WF の Sum で見つけたトーンをレベルとして使用
    .disabled = 先にトーンを検出してください: WF、Sum、トーン検出
demod-menu-average = 平均化フィルタ
    .hover = RRC がオフのとき半シンボルで積分 (矩形 FSK)
demod-menu-adaptive = 適応レベル
    .hover = レベル中心をデータのゆっくりしたドリフトに追従させる
demod-menu-info = Info
demod-menu-reset-view = 縦の倍率をリセット
demod-menu-cancel = キャンセル

# Text panel popup.
demod-menu-copy = コピー
demod-menu-save = 名前を付けて保存...
demod-menu-data-as = データ形式
demod-save-title = 復調データを保存
demod-save-filter = テキスト
demod-save-failed = { $path } に書き込めません: { $error }
demod-cutoff-invalid = RRC のカットオフ "{ $text }" は正の数ではありません。Br のように Hz で入力してください。
demod-waiting-cutoff = 有効な RRC カットオフを待っています。表示中の結果は前の設定のものです。
demod-br-invalid = Br "{ $text }" は正の数ではありません。使用中の Br をそのまま使います。
