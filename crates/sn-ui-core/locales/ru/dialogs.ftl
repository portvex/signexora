# Help > About and the error box.

about-title = О программе SigNexora
about-text = Свободная повторная реализация интерфейса и методов анализа Signals Analyzer 6.2.6.7. Не связана с его автором.
# $licence: the licence expression, e.g. "MIT OR Apache-2.0".
about-licence = Лицензия: { $licence }
about-credits = Благодарности
about-credit-sa = Signals Analyzer, автор SergUA6: исходная программа, её руководство и описания версий.
about-credit-egui = egui и eframe (MIT или Apache-2.0).
about-credit-noto = Шрифты Noto Sans JP и Noto Sans KR, SIL Open Font License 1.1.
about-credit-crates = rustfft, symphonia, rodio, hound, rhai и другие пакеты Rust из Cargo.lock, на условиях их лицензий.
dialog-ok = OK
dialog-cancel = Отмена

## SA's question before an ACF or CCF (6.2.4.2) and its limits.

# $seconds: the expected time; $megabytes: the memory needed (both with decimals).
dialogs-corr-confirm = Ожидаемое время этой операции ~ { $seconds } с,
    нужно ~ { $megabytes } МБ свободной памяти.
    Хорошо подумайте и нажмите 'Отмена', или 'OK', чтобы продолжить.
# $megabytes: the size over the limit; $limit: SA's limit, both in mBytes.
dialogs-corr-limit = Извините, нужно ~ { $megabytes } МБ свободной памяти.
    Предел { $limit } МБ, измените размер фрагмента!
# $count: SigNexora's largest selection in samples (for the plural); $samples: the
# same number with digit grouping.
dialogs-corr-too-long = { $count ->
    [one] Извините, SigNexora здесь обрабатывает не более { $samples } отсчёта, измените размер фрагмента!
    [few] Извините, SigNexora здесь обрабатывает не более { $samples } отсчётов, измените размер фрагмента!
    [many] Извините, SigNexora здесь обрабатывает не более { $samples } отсчётов, измените размер фрагмента!
   *[other] Извините, SigNexora здесь обрабатывает не более { $samples } отсчёта, измените размер фрагмента!
}
