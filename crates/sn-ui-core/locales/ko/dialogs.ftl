# Help > About and the error box.

about-title = SigNexora 정보
about-text = Signals Analyzer 6.2.6.7의 사용자 인터페이스와 분석 방법을 자유롭게 다시 구현한 프로그램입니다. 원저작자와 관계가 없습니다.
# $licence: the licence expression, e.g. "MIT OR Apache-2.0".
about-licence = 라이선스: { $licence }
about-credits = 감사의 말
about-credit-sa = SergUA6의 Signals Analyzer: 원래 프로그램, 그 설명서와 릴리스 노트.
about-credit-egui = egui와 eframe (MIT 또는 Apache-2.0).
about-credit-noto = Noto Sans JP와 Noto Sans KR 글꼴, SIL Open Font License 1.1.
about-credit-crates = rustfft, symphonia, rodio, hound, rhai 및 Cargo.lock의 다른 Rust 크레이트 (각자의 라이선스).
dialog-ok = 확인
dialog-cancel = 취소

## SA's question before an ACF or CCF (6.2.4.2) and its limits.

# $seconds: the expected time; $megabytes: the memory needed (both with decimals).
dialogs-corr-confirm = 이 처리의 예상 시간 약 { $seconds } 초,
    필요한 여유 메모리 약 { $megabytes } MB.
    잘 생각한 뒤 '취소'를, 계속하려면 '확인'을 누르십시오.
# $megabytes: the size over the limit; $limit: SA's limit, both in mBytes.
dialogs-corr-limit = 죄송합니다, 약 { $megabytes } MB 의 여유 메모리가 필요합니다.
    한도는 { $limit } MB 입니다. 조각의 크기를 바꾸십시오!
# $count: SigNexora's largest selection in samples (for the plural); $samples: the
# same number with digit grouping.
dialogs-corr-too-long = { $count ->
   *[other] 죄송합니다, SigNexora 는 여기서 최대 { $samples } 샘플까지 처리합니다. 조각의 크기를 바꾸십시오!
}
