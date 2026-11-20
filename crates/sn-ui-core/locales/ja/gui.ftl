# Texts of the Windows 95 front end (sn-gui) that the egui app does not have:
# message box buttons, the appearance schemes, the system menu of a child
# window, credits and notices. An & marks the mnemonic letter (Alt+letter).

## Message box buttons.

gui-button-yes = はい(&Y)
gui-button-no = いいえ(&N)

## Options > Theme: SigNexora's themes as Windows appearance schemes.

gui-scheme-standard = Windows 標準
gui-scheme-high-contrast-black = ハイコントラスト黒
gui-scheme-high-contrast-white = ハイコントラスト白

## The system menu of a child window.

gui-sys-restore = 元のサイズに戻す(&R)
gui-sys-move = 移動(&M)
gui-sys-size = サイズ変更(&S)
gui-sys-minimize = 最小化(&N)
gui-sys-maximize = 最大化(&X)
gui-sys-close = 閉じる(&C)
gui-sys-next = 次へ(&T)

## Help > About: credits of the Windows 95 front end.

gui-about-credit-gpui = GPUI (Zed Industries、Apache-2.0)。
gui-about-credit-w95fa = W95FA フォント (Alina Sava、FontsArena)、SIL Open Font License 1.1。
gui-about-credit-selawik = Selawik フォント (Microsoft)、SIL Open Font License 1.1 (Segoe UI、Tahoma、Trebuchet MS がない場合)。
gui-about-credit-linux-fonts = Cantarell と Noto Sans フォント、SIL Open Font License 1.1。DejaVu Sans フォント、Bitstream Vera ライセンス (Linux スタイル)。

## Notices.

# $path: the settings file; $error: why it could not be written.
gui-prefs-not-saved = 設定を { $path } に保存できませんでした: { $error }。フォルダーに書き込みできるか確認してください。
# $path: the settings file.
gui-prefs-not-encoded = 設定を { $path } に保存できませんでした: ファイル形式に変換できませんでした。これはプログラムのエラーです。報告してください。以前の設定ファイルはそのまま残ります。

## The signal generator (OCG) window and its calculators.

gui-gen-preset = プリセット(&P)...
gui-gen-generate = 生成(&G)
gui-gen-generate-save = 生成して保存(&S)...
gui-gen-calculators = 計算ツール(&C)...
gui-gen-calc-title = OCG 計算ツール
# $text: what was typed in a number field.
gui-gen-not-number = "{ $text }" は数値ではありません
gui-gen-not-whole = "{ $text }" は整数ではありません
# $min, $max: the limits of the field.
gui-gen-range = { $min } から { $max } の範囲で指定してください

## Help > Use help: the Windows 95 keys.

gui-keys-menu = メニュー バーを選択
gui-keys-menu-letter = 下線付きの文字のメニューを開く、またはボタンを押す
gui-keys-next-window = 次のウィンドウを前面に表示 (Shift で前のウィンドウ)
