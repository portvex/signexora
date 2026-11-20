# Main window (MDI frame): messages, File menu extras, the WF window's toolbar
# and popup menu, and the confirmation dialogs.

## View kinds (child windows)

app-tab-sonogram = ソノグラム
app-tab-waveform = 波形
app-tab-raster = ラスタ (VMW)
app-tab-phase = PhasePlane
app-tab-demod = FSK/MFSK 復調器
app-tab-ofdm = OFDM

## Lists of names: "A", "A and B", "A, B and C"

app-list-separator = { "、" }
app-list-and = { $rest }、{ $last }

## Background jobs (shown in the status bar while they run)

app-job-loading = { $name } を読み込み中
app-job-overview = 波形の概観を作成中
app-job-saving = { $path } を保存中
app-job-measure-line = ライン周波数を測定中
app-job-adp = ボーレートを推定中 (ADP)
app-job-writing = { $path } を書き込み中
app-job-replay = セッションを再現中
app-job-script = スクリプトを実行中
app-job-find-tones = トーンを検出中
app-job-playback = 再生を準備中
app-job-demodulator = 復調器

## Status messages

app-ready = 準備完了
app-signal-gone = その信号はもう存在しません。
app-set-h-markers-first = 先にラインの両側に H マーカーを設定してください。
app-sc-unmappable = Sc を { $edit } を通して対応付けできません。出力に入力との周波数関係がありません。{ $edit } の前の段階で搬送波を測定してください。
app-recorded = { $kind } = { $value } ({ $node }、{ $tool })
app-recorded-uncertain = { $kind } = { $value } +/- { $uncertainty } ({ $node }、{ $tool })
app-not-recorded = { $kind } は記録されませんでした: { $error }
app-mapped-onto = { $from }、{ $node } に対応付け
app-session-save-running = セッションの保存はすでに実行中です
app-export-report-failed = レポートの書き出し: { $error }
app-save-session-failed = セッションの保存: { $error }
app-open-session-failed = セッションを開く: { $error }
app-script-error = エラー: { $error }
app-replay-skipped = { $what ->
    [markers] { $node } ({ $label }): 保存されたマーカーを読み込めないため、スキップしました: { $error }
    [tones] { $node } ({ $label }): 保存されたトーンを読み込めないため、スキップしました: { $error }
    [grid] { $node } ({ $label }): 保存されたトーングリッドを読み込めないため、スキップしました: { $error }
   *[adp] { $node } ({ $label }): 保存された ADP 候補を読み込めないため、スキップしました: { $error }
}
app-session-restored = セッションを復元しました
app-session-restored-problems = セッションを復元しました。問題 { $count } 件 (ウィンドウを参照)
app-wrote-report = レポート { $path } を書き出しました
app-saved = { $path } を保存しました
app-saved-session = セッション { $path } を保存しました
app-saved-session-replaced = セッション { $path } を保存しました (以前のセッションは置き換えられました)
app-cancel-too-late = キャンセルが反映される前に { $name } が完了したため、結果を残しました。
app-job-cancelled = { $name }: キャンセルしました
app-job-failed = { $name }: { $error }
app-playback-cancelled = 再生をキャンセルしました
app-derived-opened = { $label }: 新しい信号 { $node } を { $view } で開きました
app-derived-opened-mapped = { $label }: 新しい信号 { $node } を { $view } で開きました。マーカーを新しい軸に移動しました
app-derived-opened-cleared = { $label }: 新しい信号 { $node } を { $view } で開きました。時間軸または周波数軸が異なるため、一部のマーカーを解除しました
app-derive-failed = { $label }: { $error }
app-measured-line = { $kind } = { $value } +/- { $uncertainty } ({ $node }、ライン測定。{ $level } dBFS、帯域より { $snr } dB 高い)
app-measured-line-mapped = { $kind } = { $value } +/- { $uncertainty } ({ $node }、ライン測定。{ $level } dBFS、帯域より { $snr } dB 高い)。{ $line } Hz のラインを { $power } 乗から逆算
app-loaded = { $label } を読み込みました: { $samples } サンプル、{ $rate } Hz
app-loaded-notes = { $label } を読み込みました: { $samples } サンプル、{ $rate } Hz。{ $notes }
app-script-no-signals = スクリプト { $path } は信号を生成しませんでした
app-script-finished = スクリプト { $path } が完了しました
app-adp-none = ADP は選択範囲でキーイングレートを検出できませんでした
app-tones-found = { $node } で { $count } 個のトーンを検出しました
app-markers-restored = { $node } のマーカーを復元しました
app-closed-tab-stopped = { $name } を閉じたため、実行中の { $jobs } ジョブを停止しました。ウィンドウを開き直して再実行してください。
app-nothing-to-play = 再生するものがありません。ソノグラムを開くか、V マーカーを設定してください。
app-find-tones-failed = トーン検出: { $error }

## Confirmation before work is lost

app-confirm-open-title = セッションを開く
app-confirm-open-question = 現在のセッションを置き換えますか?
app-confirm-replace = 置き換える
app-confirm-script-title = スクリプトを実行
app-confirm-script-question = スクリプトの信号で現在のセッションが置き換えられます。
app-confirm-quit-title = 終了
app-confirm-quit-unsaved = 終了する前にセッションを保存しますか?
app-confirm-quit-without-saving = 保存せずに終了
app-confirm-quit-writing = ファイルを書き込み中です。
app-confirm-quit-anyway = それでも終了
app-confirm-lost = 保存されていない信号 { $signals } 個と測定値 { $measurements } 件が閉じられます。この操作は元に戻せません。
app-confirm-write-running = { $name } は実行中です。今終了すると中断されます。
app-confirm-save-first = 先にセッションを保存
app-confirm-save-first-ask = 先にセッションを保存...
app-confirm-wait = 待つ
app-confirm-keep-open = 終了しない
app-cancel = キャンセル

## File dialogs

app-dialog-save-as = { $format } で保存
app-dialog-save-session-as = セッションに名前を付けて保存
app-filter-session = SigNexora セッション
app-dialog-export-report = 解析レポートの書き出し
app-filter-report = レポート
app-dialog-open-session = セッションを開く
app-dialog-run-script = スクリプトを実行
app-filter-rhai = Rhai スクリプト

## File menu

app-menu-file = ファイル
app-menu-open-session = セッションを開く...
    .hover = 信号で現在のセッションが置き換えられます
app-nothing-to-save = 保存するものがありません
app-menu-save-session = セッションを保存
app-menu-save-session-ask = セッションを保存...
app-menu-save-session-choose = セッションの保存先を選択
app-menu-save-session-to = { $path } に保存
app-menu-save-session-as = セッションに名前を付けて保存...
app-menu-export-report = 解析レポートの書き出し
app-menu-report-html = HTML...
app-menu-report-markdown = Markdown...

## File menu extras and the WF popup menu

app-menu-run-script = スクリプトを実行 (Rhai)...
    .hover = スクリプトを実行します。その信号で現在のセッションが置き換えられます
app-resynth = FSK/MFSK を再合成
    .hover = トーン検出で得たトーンで、Br のクリーンな信号を再生成します
    .disabled = 先にトーンを検出し (波形の Sum)、Br を測定してください。
app-menu-measure = 測定
app-measure-hint = ラインの両側に H マーカーを設定してください (区間は V マーカー)。
app-measure-br = ラインをボーレートとして測定 (Br)
app-measure-sc = ラインを搬送波として測定 (Sc)
    .hover = N 乗の後、ラインは搬送波に逆算されます (N で割り、シフトを戻します)
app-measure-adp = ボーレートを自動推定 (ADP)
    .hover = V マーカーの区間、または信号全体を使います
app-measure-dt-disabled = 先に V マーカーを設定してください。

## Hints

app-no-signals-hint = 録音を開いてください (ファイル > ファイルを開く... ({ $shortcut })、またはウィンドウにファイルをドロップ)。
app-open-in = 開く...
    .hover = この信号を別のビューやツールで開きます

## Sonogram toolbar

app-sono-shear = Shear
    .hover = FFT フレーム間の間隔 (FFT サイズに対する比)。小さいほどフレームの重なりが増えて時間方向が細かくなりますが、計算は遅くなります
app-sono-image = Image
    .hover = 最強成分から下に表示するダイナミックレンジ
app-sono-hres-hover = 周波数再割り当てでスペクトルラインを鮮明にします (微弱なクロックラインの発見に有効)。トーンのレベルは変わりませんが、ノイズはビン幅あたりで表示されるため、HRes オフ時より約 2 から 6 dB 低く表示されます (窓関数による)。
app-sono-alc-hover = 各列を最強成分で正規化します (表示のみ)
app-sono-combine = 縮小時
    .hover = 縮小表示で 1 つの画像列に入る FFT フレームをまとめる方法
app-sono-peak = ピーク
    .hover = 縮小時: 短いバーストを表示し続けます
app-sono-mean = 平均
    .hover = 縮小時: 平均化し、ノイズを滑らかにします

## Waveform toolbar

app-wave-show-q = Q を表示
app-find-tones = トーン検出
    .hover = ヒストグラムのピーク: 位相検波器出力上のトーン周波数

## Zoom and playback

app-play = 再生
    .hover = V マーカーの区間、または表示区間を再生します ({ $shortcut })
app-play-stop = 停止
    .hover = 再生を停止します ({ $shortcut })

## Child windows

app-tab-signal-gone = この信号はもう存在しません。

## Windows

app-replay-problems-title = セッションを復元しましたが、問題があります
app-script-output = スクリプト出力: { $path }
app-script-no-output = (出力なし)
# $lines: the lines kept; $dropped: the lines left out.
app-script-output-truncated = 出力は { $lines } 行で切り詰められました。残りの { $dropped } 行は保持されていません。

## Closing a window, the sonogram toolbar

app-confirm-close-tab-title = ウィンドウを閉じますか?
# $name: the window title; $jobs: the job names, joined.
app-confirm-close-tab-question = { $name } を閉じると、実行中の { $jobs } が停止します。閉じますか?
app-confirm-close-and-stop = 閉じて停止
app-confirm-keep-tab = 開いたままにする
# $prev, $next: the keys that show the previous and next signal.
app-select-signal-first = 先にウィンドウを前面に出してください ({ $prev } または { $next } で信号を順に表示)。
app-sono-fft = SizeFFT
    .hover = FFT サイズ: 大きいほど周波数分解能は細かく、時間分解能は粗くなります
app-sono-window = WinF
    .hover = 各 FFT の窓関数 (SA の WinF)
app-sono-colormap = 配色
    .hover = ソノグラム画像のカラーマップ
app-confirm-close-all-title = すべてのウィンドウを閉じる
app-confirm-close-all-question = セッションからすべての信号と測定値が削除されます。
app-confirm-close-all = すべて閉じる
app-confirm-close-all-stops = { $name } は実行中です。すべてのウィンドウを閉じると停止します。
app-replay-session-skipped = { $what ->
    [windows] 保存されたウィンドウ配置を読み込めないため、結果ごとにウィンドウを開きます: { $error }
   *[counters] 保存された名前カウンターを読み込めないため、新しい名前の番号が重複する可能性があります: { $error }
}
app-replay-windows-missing = 信号を再生できなかったため、保存されたウィンドウ { $count } 個を開きませんでした
app-drop-loader-open = ファイルダイアログが開いています。完了またはキャンセルしてから、もう一度ファイルをドロップしてください
app-drop-first-only = { $count } 個のファイルがドロップされました。最初の { $name } だけを開きます
app-sono-target = { $active ->
    [yes] アクティブなソノグラムウィンドウを変更します。
   *[no] 新しいソノグラムウィンドウの既定値を変更します。
}
app-edit-unrecordable = 再生用にセッションへ記録できませんでした ({ $error })
app-empty-open = 録音を開く... ({ $key })
app-empty-drop = またはここにファイルをドロップ (File > Open file... でも開けます)。
app-empty-guide = ワークフローガイド ({ $key }) で解析を順に確認できます。
# $action: what the chooser was for, e.g. "Open session"; $error: why it failed.
app-chooser-failed = { $action }: ファイル選択ダイアログを表示できませんでした ({ $error })。もう一度試すか、システムのファイルダイアログサービス (デスクトップポータル) が動作しているか確認してください。

## The tools that record a measurement, by the id stored in the session.

measure-tool-adp = パラメーター自動決定 (ADP)
measure-tool-adp-coarse = パラメーター自動決定、粗い値 (FFT ビンの半分以内)
measure-tool-measure-line = 周波数の測定 (測定線)
measure-tool-v-markers = V マーカー (dT)
measure-tool-ofdm-adp = OFDM ツール、Get Br (ADP)
measure-tool-script-record = スクリプト、record()
