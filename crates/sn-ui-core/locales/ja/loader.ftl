# SA's file loader, the "SA Open file" window, and its "SA - select channel"
# question. Keep the window titles, the SA control captions (Use options, Data
# start, Channels, Bits, Freq rate, Wave tag, Bits per sample, Data size, Ext.
# Info) and the list entries (All - I/Q, Ch - 1, 16, 24-A, ...) untranslated.

loader-title = SA Open file
loader-ok = OK
loader-cancel = キャンセル

## Toolbar buttons.

loader-back = 戻る
    .hover = 前のフォルダーに戻る
loader-up = 上へ
    .hover = 一つ上のフォルダーへ (Backspace)
loader-view = 表示モード
    .hover = 表示モードを切り替え: 一覧または詳細
loader-refresh = 更新
    .hover = 一覧を更新

## The list.

loader-col-name = 名前
loader-col-type = 種類/拡張子
loader-col-size = サイズ
loader-col-date = 日付
loader-col-time = 時刻
loader-type-folder = フォルダー
loader-type-link = F-Link
loader-popup-update = 更新
loader-popup-auto-preview = 自動プレビュー
loader-popup-copy = コピー
loader-popup-paste = 貼り付け
loader-popup-cancel = キャンセル
loader-search = クイック検索
    .hover = 名前の一部を入力すると、それを含む最初のファイルを選択します
loader-filter-wav-mp3 = wav & mp3
loader-filter-wav = wav
loader-filter-mp3 = mp3
loader-filter-supported = 対応する全形式
loader-filter-all = *.*
# $path: what was typed or pasted.
loader-path-not-found = パスが見つかりません: { $path }

## The info panel and the options.

loader-folder-link = フォルダーへのリンク
# $value: a number from the file header.
loader-info-tag = Wave tag = { $value }
loader-info-channels = Channels = { $value }
loader-info-rate = Freq rate = { $value }
loader-info-bits = Bits per sample = { $value }
loader-info-size = Data size = { $value }
loader-info-no-header = ヘッダーなし
loader-info-unreadable = 不明な形式
# $codec: the codec name of the file, e.g. "Intel DVI ADPCM (IMA ADPCM)".
loader-ext-info = Ext. Info: { $codec }
loader-use-options = Use options
    .hover = ヘッダーの代わりにこの設定でファイルを読みます (ヘッダーのないファイルに必要)
loader-use-options-compressed = 圧縮ファイルはヘッダーに従ってデコードされます。手動の設定ではノイズとして読まれます。
loader-use-options-headerless = ヘッダーのないデータはこれらの設定でのみ読み込めます。
loader-data-start = Data start
    .hover = サンプルの前に読み飛ばすバイト数
loader-channels = Channels
    .hover = 読み込むチャンネル。All は最初の 2 つを I/Q として読みます
loader-bits = Bits
    .hover = サンプル形式とバイト順
loader-freq-rate = Freq rate
    .hover = サンプルレート (Hz)
loader-in-file = ファイルのチャンネル数
    .hover = ヘッダーのないこのファイルでインターリーブされたチャンネル数
# $error: why the file cannot be read.
loader-cannot-read = このファイルを読めません: { $error }
loader-check-settings = 設定を確認してください。

## The preview.

loader-preview-building = プレビューを作成中...
loader-prescaler = プリスケーラー
    .hover = プリスケーラー: { $gain } dB。フルスケール (赤い線) を超えるサンプルはクリップされます。

## SA - select channel.

loader-select-channel-title = SA - select channel
loader-select-channel-group = このファイルはステレオです。選択してください
loader-channel-1 = チャンネル - 1
loader-channel-2 = チャンネル - 2
loader-all-samples = 全サンプル
loader-break = 中断
loader-needs-options = このファイルには読めるヘッダーがありません。Use options をオンにして形式を設定してください。
loader-no-file = 先にファイルを選択してください。
loader-rate-invalid = Freq rate "{ $text }" は無効です。48000 のように 0 より大きく 1000000000 Hz 以下の値を入力してください。
loader-data-start-invalid = Data start "{ $text }" は無効です。0 や 44 のように整数のバイト数を入力してください。
loader-list-error = フォルダー { $path } を読み込めません: { $error }。権限を確認するか、別のフォルダーを選んでください。
