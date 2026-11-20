# The VMW mode of the WF window. SA's control names (Y scale, As time, Bright,
# WaveForm, TS, Info) and the units S, ms, us, ns are names: keep them.

vmw-strobe = Strobe
    .hover = ストロボ: N 周期ごとに表示 (1 から 1000)。小さな周期誤差を傾きとして見せる
vmw-y-scale = Y scale
    .hover = ラスタ 1 行の高さ (ピクセル)
vmw-as-time = As time
    .hover = ライン周期を WF の V マーカー間隔に合わせる
    .disabled = 先に WF で V マーカーを設定してください
vmw-waveform = WaveForm
    .hover = WF に戻る
vmw-info = Info
    .hover = dT、S、ライン周波数 TSs を表示
vmw-ts = TS
    .hover = ライン周期 S:ms.us.ns。数字を左クリックで +1、右クリックで -1
vmw-period-slider = TS
    .hover = ライン周期 (粗調整、対数)
vmw-status = dT
    .hover = V マーカー間隔とその周波数

## Popup menu (SigNexora tools).

vmw-popup-start-v1 = WF の最初の V マーカーからラスタを開始
vmw-popup-from-dt = ライン周期 = 最後に測定した dT
vmw-popup-colours = カラーマップ
vmw-popup-sa-colours = Signals Analyzer
