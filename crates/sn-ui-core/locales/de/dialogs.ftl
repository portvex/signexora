# Help > About and the error box.

about-title = Über SigNexora
about-text = Eine freie Neuimplementierung der Oberfläche und Analysemethoden von Signals Analyzer 6.2.6.7. Nicht mit dessen Autor verbunden.
# $licence: the licence expression, e.g. "MIT OR Apache-2.0".
about-licence = Lizenz: { $licence }
about-credits = Danksagungen
about-credit-sa = Signals Analyzer von SergUA6: das Originalprogramm, sein Handbuch und seine Versionshinweise.
about-credit-egui = egui und eframe (MIT oder Apache-2.0).
about-credit-noto = Schriften Noto Sans JP und Noto Sans KR, SIL Open Font License 1.1.
about-credit-crates = rustfft, symphonia, rodio, hound, rhai und die übrigen Rust-Crates aus Cargo.lock, unter ihren eigenen Lizenzen.
dialog-ok = OK
dialog-cancel = Abbrechen

## SA's question before an ACF or CCF (6.2.4.2) and its limits.

# $seconds: the expected time; $megabytes: the memory needed (both with decimals).
dialogs-corr-confirm = Erwartete Laufzeit dieses Vorgangs ~ { $seconds } Sekunden,
    benötigt ~ { $megabytes } MB freien Speicher.
    Bitte gut überlegen und 'Abbrechen' drücken, oder 'OK' zum Fortfahren.
# $megabytes: the size over the limit; $limit: SA's limit, both in mBytes.
dialogs-corr-limit = Leider werden ~ { $megabytes } MB freier Speicher benötigt.
    Die Grenze ist { $limit } MB, bitte die Größe des Fragments ändern!
# $count: SigNexora's largest selection in samples (for the plural); $samples: the
# same number with digit grouping.
dialogs-corr-too-long = { $count ->
    [one] Leider verarbeitet SigNexora hier höchstens { $samples } Sample, bitte die Größe des Fragments ändern!
   *[other] Leider verarbeitet SigNexora hier höchstens { $samples } Samples, bitte die Größe des Fragments ändern!
}
