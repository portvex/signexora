# The workflow guide: SA's analysis method as a checklist with buttons.

guide-title = 作業ガイド
guide-no-signal = 先に録音を開いてください (ファイル > ファイルを開く...)。
# $signal: the active window's signal, which the buttons act on.
guide-acts-on = ボタンの対象: { $signal }
# $key: a key name. Shown after a menu path only when that key acts on the guide's signal.
guide-key = 、キー { $key }

## Steps: the message is the step's title, .text its explanation.

guide-step-record = 1. 正しく録音する
    .text = 受信機の AGC をオフにし、クリッピングや編集のない 16 ビット PCM WAV (または I/Q) で録音します。圧縮ファイルや 8 ビットのファイルは開くときに警告されます。
guide-step-quality = 2. 録音品質の確認
    .text = WF ウィンドウを開きます (ソノグラムを右クリック > { sw-popup-waveform })。クリッピング、8 ビットの段差、直流成分、AGC の揺れ、欠落を確認します。Sum をオンにすると振幅分布が見えます。
guide-step-isolate = 3. 信号の切り出し
    .text = ソノグラムで H マーカーを信号の周りに置きます。次にツールバーの { sw-hint-band-select } と { sw-hint-shift-base } を使い、帯域が広ければ R で低いレートにリサンプルします。
guide-step-rate = 4. 変調速度を求める
    .text = 検波器をかけ、そのクロックラインをマーカーで囲み、ポップアップメニューの { sw-popup-measure } を使います{ $br }。PSK/QAM/FSK には { sw-hint-am }、MFSK には { sw-hint-modified-am }、または { sw-popup-adp } を試します。
guide-step-family = 5. 変調方式の解析
    .text = FSK/MFSK: { sw-hint-phase-detector }、次に WF ウィンドウで 1/3 シンボルの { edits-median }、Sum、トーン検出、周波数カウント。PSK/QAM: { sw-hint-power } で次数と搬送波を求め (そのウィンドウで { sw-popup-measure }{ $sc })、次に PhasePlane。歪んだ信号では先に ISD コレクター。OFDM: CT 検出、Br 取得、チャンネル、次に本来のレートのための K リスト。
guide-step-structure = 6. 構造を見つける
    .text = フレーム周期には { sw-hint-acf }、断片を全体から探すには { sw-hint-ccf }、次に V マーカーを 1 フレームに置き、ポップアップメニューの { sw-popup-measure } > { raster-dt-menu-item } を選んで dT (フレーム周期) を記録し、その周期で { guide-btn-raster } を開いてフレームと同期語を見ます。

## Step buttons.

guide-btn-waveform = 波形
guide-btn-sonogram = ソノグラム
guide-btn-am-detector = AM 検波器
guide-btn-modified-am = 修正 AM
guide-btn-phase-detector = 位相検波器
guide-btn-demod = FSK/MFSK 復調器
# $n: the power, a number.
guide-btn-power = { $n } 乗
    .hover = { $n } 乗した新しい信号を作成します。PSK-{ $n } 信号では搬送波の { $n } 倍の位置にラインが現れます
guide-btn-phase-plane = PhasePlane
guide-btn-raster = ラスタ (VMW)
guide-btn-adp = ADP
    .hover = V マーカーの区間 (または信号全体) からキーイングレートを自動推定します (ADP)
guide-btn-ofdm = OFDM
    .hover = OFDM ツールを開きます (Find CT、Get Br、K の一覧)
guide-open-view = アクティブなウィンドウの信号をこのビューで開きます
guide-new-signal = アクティブなウィンドウの信号から新しい信号を作ります (新しいウィンドウで開きます)
# $kind: Br, Sc or dT (untranslated).
guide-not-measured = { $kind }: まだ測定されていません
