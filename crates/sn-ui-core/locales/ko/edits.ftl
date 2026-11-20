# The Signal menu, the parameter dialog of its operations and the save formats.

edits-save-wav-pcm16 = WAV, 16비트 PCM
edits-save-wav-float = WAV, 32비트 부동소수점
edits-save-sigmf = SigMF (분수 레이트 유지)
edits-save-raw-f32 = Raw, 32비트 부동소수점

# $from: where a measured value came from, e.g. "n4, measure line".
edits-measured-other-axis = { $from } (다른 축, 확인 필요)

## Notes under the dialog fields: where a pre-filled value came from.

edits-note-cutoff-raised = 차단 주파수: Br 출처 { $from } (최소값 30 Hz로 올림, Br = { $br } Bd)
edits-note-cutoff-lowered = 차단 주파수: Br 출처 { $from } (샘플 레이트의 절반 미만으로 낮춤, Br = { $br } Bd)
edits-note-cutoff-br = 차단 주파수: Br 출처 { $from }
edits-note-cutoff-guessed = 차단 주파수: 추정값 (fs/8), 먼저 Br을 측정하십시오
# $count: the longest window offered, in samples.
edits-note-window-capped = 윈도우: Br 출처 { $from } 기준 1/3 심볼 (샘플 { $count }개로 제한, 정확한 1/3 심볼을 원하면 다운샘플링하십시오)
edits-note-window-br = 윈도우: Br 출처 { $from } 기준 1/3 심볼
edits-note-window-default = 윈도우: 기본값 5 샘플, 1/3 심볼을 얻으려면 Br을 측정하십시오
edits-note-measured-br = 측정된 Br 출처: { $from }
edits-note-measured-s = 측정값: S = V 마커의 1/dT
edits-note-carrier-sc = 반송파: Sc 출처 { $from }
edits-note-carrier-h = 반송파: H 마커의 중심
edits-note-carrier-guessed = 반송파: 추정값 (fs/4), 먼저 Sc를 측정하십시오
edits-note-br-from = Br: 출처 { $from }
edits-note-br-guessed-8 = Br: 추정값 (fs/8), 먼저 Br을 측정하십시오
edits-note-br-s = Br: S = V 마커의 1/dT
edits-note-br-guessed-10 = Br: 추정값 (fs/10), 먼저 Br을 측정하십시오

## Why a menu item is disabled.

edits-need-h = 먼저 H 마커를 설정하십시오(소노그램에서 드래그).
edits-need-v = 먼저 V 마커를 설정하십시오(소노그램 또는 파형에서 드래그).
edits-need-vh = 먼저 V 마커와 H 마커를 설정하십시오(소노그램에서 드래그).
edits-need-complex = 신호가 이미 실수입니다.
edits-need-baud = 먼저 Br을 측정하거나(측정 메뉴) V 마커를 한 심볼에 맞추십시오.

## Signal menu sections and items.

edits-section-spectrum = 스펙트럼 (H 마커)
edits-band-select = 대역 선택
edits-band-reject = 대역 제거
edits-area-delete = 영역 삭제
edits-shift-to-base = 대역을 기저대역으로 이동
edits-shift-to-base-inverted = 대역을 기저대역으로 이동, 반전
edits-invert-band = 대역 내 반전
edits-section-whole = 전체 신호
edits-shift = 스펙트럼 이동...
edits-invert = 스펙트럼 반전
edits-resample = 리샘플링...
edits-iq-to-real = I/Q를 실수로
edits-baud-correction = 보 레이트 보정...
edits-derivative = 1차 미분
edits-limiter = 하드 리미터
edits-section-isd = ISD 보정기
edits-isd-am = ISD 보정기, AM 모드 (블라인드)...
edits-isd-xn = ISD 보정기, X^N 모드...
edits-section-detectors = 검파기
edits-am-detector = AM 검파기 (PSK/QAM/FSK 클록)
edits-modified-am = 수정 AM 검파기 (MFSK 클록)
edits-half-wave = 이중 반파 검파기
edits-phase-detector = 위상 검파기 (주파수, Hz)
edits-power = 거듭제곱 N...
edits-acf = 선택 영역 ACF
edits-ccf = CCF: 선택 영역을 전체에서 찾기
edits-section-time = 시간 (V 마커)
edits-copy = 선택 영역을 새 신호로 복사
edits-section-filters = 필터
edits-lowpass = 저역 통과...
edits-median = 중앙값 필터...
edits-rrc = RRC (정합 필터)...

## 매개변수 대화 상자 (SA의 작은 모달 대화 상자).

# $operation: the dialog's operation; $signal: the window name, e.g. "test(0)(u1)".
edits-dialog-title = SA - { $operation }: { $signal }
edits-title-shift = 스펙트럼 이동
edits-title-resample = 리샘플링
edits-title-baud = BR 보정
edits-title-lowpass = 저역 통과
edits-title-median = 중앙값 필터
edits-title-rrc = RRC 필터
edits-title-power = 거듭제곱 N
edits-title-isd-am = ISD 보정기 (AM 모드)
edits-title-isd-xn = ISD 보정기 (X^N 모드)

edits-field-carrier = 반송파 (Hz)
edits-field-br = Br, 심볼 속도 (Bd)
edits-field-blocks = 블록 수
edits-field-power-psk = 거듭제곱 N (PSK 차수)
edits-field-window = 창 (샘플, 홀수)
edits-field-shift = 이동 (Hz, + 는 위로)
edits-field-rate = 새 샘플 레이트 (Hz)
edits-field-measured-br = 현재 보레이트
edits-field-required-br = 필요한 보레이트
edits-field-cutoff = 차단 주파수 (Hz)
edits-presets = 프리셋
edits-field-power = 거듭제곱 N
edits-field-alpha = 롤오프 alpha
# $lo, $hi: the limits of a field.
edits-range = { $lo }부터 { $hi }까지
# $max: the sample rate in Hz.
edits-hint-shift = +/- { $max } Hz까지, 0 제외, 소수 허용
# $max: the longest window, in samples.
edits-hint-window = 1부터 { $max }까지, 짝수는 다음 홀수로 사용
edits-ok = 확인
edits-cancel = 취소
edits-field-invalid = { $field } "{ $text }"은(는) 올바르지 않습니다. { $range }.
edits-range-open = { $lo } 초과 { $hi } 미만
