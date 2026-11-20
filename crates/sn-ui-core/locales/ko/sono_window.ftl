# The sonogram work window: SA's top toolbar captions, the bottom toolbar hints,
# the right-click popup menu and the Correction of BR panel. SA's control names
# (Fl, Fu, ALC, HRes, Info, WaveFile, DataFile) are names: keep them.

## Top toolbar (SizeFFT, Shear, Image and WinF are in app.ftl).

sw-fl = Fl
    .hover = 표시된 가장 낮은 주파수, Hz (대역을 확대한 동안 파란색)
sw-fu = Fu
    .hover = 표시된 가장 높은 주파수, Hz (대역을 확대한 동안 파란색)
sw-alc = ALC
sw-hres = HRes
sw-info = Info
    .hover = 결과 표시를 켭니다 (Fu, Fc, Fl, dF, t1, t2, dT, S, Br, Sc)

## Bottom toolbar hints, left to right.

sw-hint-zoom-in = H 마커 사이의 대역을 창 전체로 확대
sw-hint-zoom-back = 확대 한 단계 뒤로
sw-hint-zoom-reset = 확대 초기화
sw-hint-abort = 중지
sw-hint-play = 재생
sw-hint-stop-play = 재생 정지
sw-hint-band-select = 대역 선택
sw-hint-area-delete = 영역 삭제
sw-hint-band-reject = 대역 제거
sw-hint-am = 직교 AM 검파기
sw-hint-modified-am = 수정 AM 검파기
sw-hint-half-wave = 이중 반파 검파기
sw-hint-derivative = 1차 미분
sw-hint-power = N 거듭제곱 모듈
sw-hint-phase-plane = PhasePlane
sw-hint-acf = 자기상관
sw-hint-ccf = 상호상관
sw-hint-ofdm = OFDM 모듈
sw-hint-phase-detector = 위상 검파기
sw-hint-limiter = 하드 리미터
sw-hint-iq = I/Q를 실수로
sw-hint-shift-base = 대역을 기저대역으로 이동
sw-hint-shift-base-inverted = 대역을 기저대역으로 이동, 반전
sw-hint-inverse = 대역 내 반전 (H 마커가 겹쳐 있으면 스펙트럼 전체)
sw-hint-copy = 조각 복사
sw-hint-resample = 입력란의 레이트로 리샘플링
sw-hint-shift = 입력란의 값만큼 스펙트럼 이동 (Hz, 음수는 아래로)
sw-value = R / S
    .hover = R의 새 레이트 또는 S의 이동량, Hz (소수점은 쉼표나 마침표)
sw-value-presets = 미리 정한 값

## Why a button is disabled.

sw-need-history = 되돌아갈 확대 단계가 없습니다.
sw-need-running = 이 창에서 실행 중인 작업이 없습니다.
sw-need-playing = 재생 중이 아닙니다.

## Popup menu.

sw-popup-save-as = 다른 이름으로 저장
sw-popup-wave-file = WaveFile
sw-popup-data-file = DataFile
sw-popup-wave-file-float = WaveFile (32비트 부동소수점)
sw-popup-sigmf = SigMF
sw-popup-waveform = Wave Form 모듈
sw-popup-measure = 주파수 측정
sw-popup-adp = 매개변수 자동 결정
sw-popup-brc = BR 보정
sw-popup-cancel = 취소

## Correction of BR panel.

sw-brc-existing = 현재 보레이트
sw-brc-required = 필요한 보레이트
sw-brc-max-delta = 최대 변화: +/- 30%
sw-brc-yes = 예

## Messages.

sw-check-settings = 설정을 확인하십시오.
# Result boxes in the plot (SA's own text). $value: a number with two decimals.
sw-label-freq = Freq: { $value } Hz
sw-label-brate = Brate: { $value } Hz
sw-rs-resample-invalid = R/S 값 "{ $text }"은(는) 양의 샘플 레이트가 아닙니다. 8000처럼 Hz 단위로 입력하세요.
sw-rs-shift-invalid = R/S 값 "{ $text }"은(는) 숫자가 아닙니다. -150처럼 Hz 단위로 이동량을 입력하세요.
sw-brc-existing-invalid = Existent Baud Rate "{ $text }"은(는) 숫자가 아닙니다. 측정한 Br을 Bd로 입력하거나 먼저 ADP를 실행하세요.
sw-brc-required-invalid = Required Baud Rate "{ $text }"은(는) 숫자가 아닙니다. 원하는 Br을 Bd로 입력하세요.
sw-brc-not-positive = 두 보 레이트 모두 0 Bd보다 커야 합니다.
sw-brc-too-far = Required Br은 { $lo }에서 { $hi } Bd 사이여야 합니다 (기존 값에서 최대 30%).
sw-brc-measured = ADP로 측정한 Br로 미리 채움
sw-brc-not-measured = 아직 측정된 Br이 없습니다. 입력하거나 먼저 ADP를 실행하세요
