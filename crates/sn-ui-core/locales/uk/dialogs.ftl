# Help > About and the error box.

about-title = Про програму SigNexora
about-text = Вільна повторна реалізація інтерфейсу та методів аналізу Signals Analyzer 6.2.6.7. Не пов'язана з його автором.
# $licence: the licence expression, e.g. "MIT OR Apache-2.0".
about-licence = Ліцензія: { $licence }
about-credits = Подяки
about-credit-sa = Signals Analyzer, автор SergUA6: оригінальна програма, її посібник і описи версій.
about-credit-egui = egui та eframe (MIT або Apache-2.0).
about-credit-noto = Шрифти Noto Sans JP і Noto Sans KR, SIL Open Font License 1.1.
about-credit-crates = rustfft, symphonia, rodio, hound, rhai та інші пакети Rust з Cargo.lock, на умовах їхніх ліцензій.
dialog-ok = OK
dialog-cancel = Скасувати

## SA's question before an ACF or CCF (6.2.4.2) and its limits.

# $seconds: the expected time; $megabytes: the memory needed (both with decimals).
dialogs-corr-confirm = Очікуваний час цієї операції ~ { $seconds } с,
    потрібно ~ { $megabytes } МБ вільної пам'яті.
    Добре подумайте й натисніть 'Скасувати', або 'OK', щоб продовжити.
# $megabytes: the size over the limit; $limit: SA's limit, both in mBytes.
dialogs-corr-limit = Вибачте, потрібно ~ { $megabytes } МБ вільної пам'яті.
    Межа { $limit } МБ, змініть розмір фрагмента!
# $count: SigNexora's largest selection in samples (for the plural); $samples: the
# same number with digit grouping.
dialogs-corr-too-long = { $count ->
    [one] Вибачте, SigNexora тут обробляє не більше { $samples } відліку, змініть розмір фрагмента!
    [few] Вибачте, SigNexora тут обробляє не більше { $samples } відліків, змініть розмір фрагмента!
    [many] Вибачте, SigNexora тут обробляє не більше { $samples } відліків, змініть розмір фрагмента!
   *[other] Вибачте, SigNexora тут обробляє не більше { $samples } відліку, змініть розмір фрагмента!
}
