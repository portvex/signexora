# Help > About and the error box.

about-title = About SigNexora
about-text = SigNexora is a signal intelligence tool for offline radio-signal analysis. A free reimplementation of the user interface and analysis methods of Signals Analyzer 6.2.6.7. Not affiliated with its author.
# $licence: the licence expression, e.g. "MIT OR Apache-2.0".
about-licence = Licence: { $licence }
about-credits = Credits
about-credit-sa = Signals Analyzer by SergUA6: the original program, its manual and release notes.
about-credit-egui = egui and eframe (MIT or Apache-2.0).
about-credit-noto = Noto Sans JP and Noto Sans KR fonts, SIL Open Font License 1.1.
about-credit-crates = rustfft, symphonia, rodio, hound, rhai and the other Rust crates in Cargo.lock, under their own licences.
dialog-ok = OK
dialog-cancel = Cancel

## SA's question before an ACF or CCF (6.2.4.2) and its limits.

# $seconds: the expected time; $megabytes: the memory needed (both with decimals).
dialogs-corr-confirm = Expected running time of this process ~ { $seconds } seconds,
    and needs ~ { $megabytes } mBytes free memory space.
    Please think it over carefully, and press 'Cancel', or 'OK' to continue.
# $megabytes: the size over the limit; $limit: SA's limit, both in mBytes.
dialogs-corr-limit = Sorry, needs ~ { $megabytes } mBytes free memory space.
    Limit is { $limit } mBytes, please change size of fragment!
# $count: SigNexora's largest selection in samples (for the plural); $samples: the
# same number with digit grouping.
dialogs-corr-too-long = { $count ->
    [one] Sorry, SigNexora works on at most { $samples } sample here, please change size of fragment!
   *[other] Sorry, SigNexora works on at most { $samples } samples here, please change size of fragment!
}
