# Help > About and the error box.

about-title = SigNexora について
about-text = Signals Analyzer 6.2.6.7 のユーザーインターフェースと解析手法を自由に再実装したものです。原作者とは関係ありません。
# $licence: the licence expression, e.g. "MIT OR Apache-2.0".
about-licence = ライセンス: { $licence }
about-credits = 謝辞
about-credit-sa = SergUA6 による Signals Analyzer: 原プログラム、その説明書とリリースノート。
about-credit-egui = egui と eframe (MIT または Apache-2.0)。
about-credit-noto = Noto Sans JP と Noto Sans KR フォント、SIL Open Font License 1.1。
about-credit-crates = rustfft、symphonia、rodio、hound、rhai ほか Cargo.lock にある Rust クレート (各ライセンスに従う)。
dialog-ok = OK
dialog-cancel = キャンセル

## SA's question before an ACF or CCF (6.2.4.2) and its limits.

# $seconds: the expected time; $megabytes: the memory needed (both with decimals).
dialogs-corr-confirm = この処理の予想時間 約 { $seconds } 秒、
    必要な空きメモリ 約 { $megabytes } MB。
    よく考えてから 'キャンセル' を、続けるには 'OK' を押してください。
# $megabytes: the size over the limit; $limit: SA's limit, both in mBytes.
dialogs-corr-limit = 申し訳ありません、約 { $megabytes } MB の空きメモリが必要です。
    上限は { $limit } MB です。断片の大きさを変えてください。
# $count: SigNexora's largest selection in samples (for the plural); $samples: the
# same number with digit grouping.
dialogs-corr-too-long = { $count ->
   *[other] 申し訳ありません、SigNexora がここで扱えるのは最大 { $samples } サンプルです。断片の大きさを変えてください。
}
