# File > Signal generator (OCG): the generator window, its preview and the
# OFDM calculators. Keep units (Hz, Bd, s, ms, dB, ppm), modulation names
# (FSK, MSK, GMSK, PSK, QAM, C4FM, OFDM, BPSK, ...), BT, Es/N0, SNR, RC, RRC,
# LU, LG, LS, K, Br, Fd and the model letters A, B, C untranslated.

gen-menu = 신호 발생기 (OCG)...
    .hover = 파라미터를 정확히 아는 테스트 신호를 만듭니다: 톤, 잡음, FSK, PSK/QAM, C4FM 또는 OFDM, 채널 열화 포함
gen-keys-open = 신호 발생기 표시/숨기기
gen-title = 신호 발생기 (OCG)
gen-job = { $name } 생성 중

gen-kind = 종류
gen-kind-tones = 톤
gen-kind-noise = 잡음
gen-kind-fsk = FSK/MFSK
gen-kind-linear = PSK/QAM
gen-kind-c4fm = C4FM
gen-kind-ofdm = OFDM
gen-preset = 프리셋
    .hover = 미리 만든 신호를 불러옵니다
gen-preset-defaults = 기본값
    .hover = 현재 종류를 기본값으로 되돌립니다

gen-section-signal = 신호
gen-section-common = 공통
gen-section-channel = 채널 열화
gen-section-derived = 파생 값
gen-section-preview = 미리 보기
gen-section-calc = 계산기

gen-complex = 복소 (I/Q) 출력
    .hover = 실수 신호 대신 I/Q 샘플을 만듭니다. 주파수는 0 Hz로부터의 오프셋이 됩니다
gen-amplitude-rms = RMS 진폭
gen-amplitude-peak = 피크 진폭
gen-phase-rad = 위상 (rad)
gen-remove = 제거
gen-baud = 보 레이트
gen-carrier = 반송파
gen-center = 중심
gen-alpha = 롤오프

gen-tones-frequency = 주파수
gen-tones-add = 톤 추가
gen-tones-last = 톤이 하나 이상 필요합니다

gen-fsk-order = 톤 수
gen-fsk-spacing = 간격
gen-fsk-shaping = 정형
gen-fsk-shaping-none = 없음
gen-fsk-shaping-gaussian = 가우시안
gen-fsk-bt = BT
gen-fsk-msk = MSK
    .hover = 톤 2개, 간격은 보 레이트의 절반, 정형 없음
gen-fsk-gmsk = GMSK
    .hover = BT 0.3 가우시안 필터를 적용한 MSK (GSM과 동일)

gen-lin-constellation = 성상도
gen-lin-shaping = 펄스 형태
gen-lin-rect = 사각형
gen-lin-rc = 상승 코사인 (RC)
gen-lin-rrc = 루트 상승 코사인 (RRC)
gen-lin-differential = 차동
    .hover = 각 심볼을 이전 심볼로부터의 위상 변화로 보냅니다
gen-lin-differential-qam = QAM은 위상뿐 아니라 진폭도 실으므로 차동으로 할 수 없습니다
gen-lin-differential-pi4 = pi/4-DQPSK는 항상 차동입니다

gen-c4fm-pulse = 펄스
gen-c4fm-shaped = 정형
    .hover = 끄면 주파수가 네 레벨 사이를 건너뜁니다
gen-c4fm-apco = APCO-25
    .hover = TIA-102 펄스, 적분 덤프 수신기용
gen-c4fm-note = 편이는 +-600 Hz와 +-1800 Hz로 고정됩니다. C4FM 출력은 항상 실수입니다.

gen-ofdm-lu = LU
    .hover = 샘플링 레이트 기준 유효 길이 (FFT 크기, 샘플 수)
gen-ofdm-lg = LG
    .hover = 가드 (순환 전치) 길이, 샘플 수
gen-ofdm-channels = 채널 수
gen-ofdm-first-bin = 첫 빈
    .hover = 첫 채널의 빈. 빈 0이 중심입니다
gen-ofdm-modulation = 변조
gen-ofdm-unused = 미사용 위치
    .hover = 아무것도 보내지 않는 채널 위치, 0부터 셈. 예: 3, 7, 10-12
gen-ofdm-overrides = 개별 지정
    .hover = 개별 위치의 변조. 예: 0: pilot, 5-6: QAM-16
gen-ofdm-model = 모델
gen-ofdm-model-a = A: 모든 채널이 절대값을 보냄
gen-ofdm-model-b = B: 모든 채널이 심볼 간 차동
gen-ofdm-model-c = C: 파일럿이 기준을 싣고, 데이터는 그에 대한 절대값
gen-ofdm-pilots = 파일럿
gen-ofdm-pilots-none = 없음
gen-ofdm-pilots-list = 위치
    .hover = 파일럿 위치. 예: 0, 8, 16
gen-ofdm-pilots-every = N개마다
gen-ofdm-step = 간격
gen-ofdm-offset = 시작
gen-ofdm-reference = 파일럿 기준
    .hover = 모델 C에서 파일럿이 싣는 것
    .disabled = 파일럿 기준은 모델 C에서만 사용합니다
gen-ofdm-reference-constant = 상수
gen-ofdm-reference-differential = 무작위 공통 위상
gen-ofdm-center = 중심
    .hover = 빈 0의 위치. 복소 출력에서는 0 Hz로부터의 오프셋

gen-label = 레이블
    .hover = 새 신호의 이름. 비워 두면 설명을 사용합니다
gen-sample-rate = 샘플링 레이트
gen-length = 길이
gen-length-seconds = 초
gen-length-symbols = 심볼
    .disabled = 톤과 잡음에는 심볼이 없습니다
gen-seed = 시드
gen-new-seed = 새 시드
    .hover = 무작위 시드 선택: 새 데이터와 새 잡음
gen-center-frequency = 중심 주파수
    .hover = 복소 출력의 0 Hz에 해당하는 RF 주파수 (신호와 함께 저장)
    .disabled = 중심 주파수는 복소 출력에만 있습니다

gen-ch-offset = 주파수 오프셋
gen-ch-drift = 드리프트
gen-ch-clock = 클록 오프셋
    .hover = 샘플 클록 오차. 양수이면 수신기 클록이 빠릅니다
gen-ch-noise = 잡음
gen-ch-noise-none = 없음
gen-ch-noise-full = 전체 대역 SNR
gen-ch-noise-inband = 대역 내 SNR
gen-ch-noise-esn0 = Es/N0
gen-ch-bandwidth = 대역폭
    .hover = 끄면 신호의 점유 대역폭
gen-ch-clip = 클리핑
    .hover = 피크 진폭을 제한합니다. 복소 샘플은 위상을 유지합니다
gen-ch-multipath = 다중 경로 탭
    .hover = 탭이 채널 전체입니다: 직접 경로(지연 0, 이득 1)도 포함하십시오
gen-ch-delay = 지연
gen-ch-gain = 이득
gen-ch-add-tap = 탭 추가

gen-d-none = 파라미터를 수정하면 파생 값이 표시됩니다.
gen-d-symbol-rate = 심볼 속도
gen-d-sps = 심볼당 샘플 수
gen-d-symbols = 심볼 수
gen-d-bandwidth = 점유 대역폭
gen-d-samples = 샘플 수
gen-d-duration = 길이
gen-d-index = 변조 지수 h
gen-d-ls = LS
gen-d-k = K = LG/LU
gen-d-spacing = 간격
gen-d-br = Br

gen-preview = 발생기 미리 보기
gen-preview-spectrum = 스펙트럼
gen-preview-wave = 파형
gen-preview-name = 생성 신호 미리 보기
gen-preview-building = 미리 보기 생성 중...
gen-preview-invalid = 파라미터가 유효해야 미리 보기가 표시됩니다
gen-preview-failed = 미리 보기 실패: { $error }
gen-preview-short = 스펙트럼을 만들기에 너무 짧습니다
gen-preview-axis = 평균 스펙트럼, { $lo } ~ { $hi } Hz, 범위 80 dB
gen-preview-wave-axis = 처음 { $ms } ms, 피크 { $peak }

gen-generate = 생성
    .hover = 신호를 새 녹음으로 세션에 추가합니다
gen-generate-save = 생성 후 저장
    .hover = 생성한 다음 새 신호를 선택한 형식으로 저장합니다
gen-busy = 신호를 생성하는 중입니다
gen-fix-first = 먼저 파라미터를 수정하십시오
gen-invalid = 생성할 수 없음: { $error }
gen-field-error = { $field }: { $error }
gen-ready = 준비됨: 샘플 { $samples }개, { $duration } s
gen-list-bad = "{ $item }"은(는) 위치도 10-12 같은 범위도 아닙니다
gen-list-too-long = 위치가 너무 많습니다 (최대 { $max }개)
gen-overrides-bad = "{ $item }"은(는) 5: QAM-16 같은 위치와 변조 쌍이 아닙니다

gen-calc-cp = CP 포함 OFDM
gen-calc-cp-lengths = Fd, LU, LG로부터
gen-calc-spacing = 간격
gen-calc-symbol = 심볼
gen-calc-useful = 유효 부분
gen-calc-guard = 가드
gen-calc-use = OFDM에 적용
    .hover = 이 길이(와 Fd)를 OFDM 편집기로 복사합니다
    .disabled = LU와 LG는 정수 샘플 수여야 합니다
# Why Use in OFDM is disabled. $max: the generator's limit in samples.
gen-calc-lu-not-whole = LU는 2 이상의 정수 샘플 수여야 합니다.
gen-calc-lg-not-whole = LG는 정수 샘플 수여야 합니다.
gen-calc-lu-too-large = LU는 최대 { $max } 샘플이어야 합니다.
gen-calc-lg-too-large = LG는 최대 { $max } 샘플이어야 합니다.
gen-calc-use-k = OFDM에 적용
    .hover = 이 K를 OFDM 편집기로 복사합니다: 현재 LU 이상에서 K에 맞는 가장 작은 LU
gen-calc-inverse = Fd에서의 Br과 간격으로부터: 가장 가까운 정수 LU와 LG
gen-calc-exact = 정확
gen-calc-error = 오차
gen-calc-candidates = Br과 K에 대한 정수 LU와 LG
gen-calc-lu-range = LU 시작값
gen-calc-tolerance = 허용 오차
gen-calc-found = { $count ->
   *[other] 후보 { $count }개, 처음 { $shown }개 표시
}
gen-calc-klist = K 목록
gen-calc-k-from-rates = Br과 간격으로부터
gen-calc-k-direct = K로부터
gen-calc-uncertainty = 불확도
    .hover = 측정한 각 속도가 벗어날 수 있는 정도 (속도에 대한 백분율). 목록에는 이로부터 나오는 K 불확도 안에서 가장 간단한 분수가 표시됩니다. K를 직접 입력하면 이만큼 벗어난 두 속도로 측정한 K의 허용 오차를 사용합니다.
gen-calc-uncertainty-bad = 불확도는 0에서 { $max } % 사이여야 합니다.
gen-calc-k-none = 불확도 안에 있는 분수가 없습니다. 불확도나 최대 분모를 늘리십시오.
gen-calc-max-den = 최대 분모
gen-calc-count = 개수
gen-calc-value = 값
gen-calc-fd = Fd
    .hover = 이 Br에서 이 길이들이 정확해지는 샘플링 속도(Hz): Br x (LU + LG)
gen-calc-k = K
    .hover = K = LG/LU, 이 정수 길이들이 실제로 갖는 보호 구간 대 유효 길이 비율
gen-calc-lg-error = 오차 (샘플)
    .hover = LG - K x LU: 정수 보호 구간 길이가 정확한 값에서 얼마나 떨어져 있는지 (샘플 단위)
gen-calc-k-fraction = K
    .hover = 기약분수 p/q로 나타낸 K
gen-calc-k-error = 오차
    .hover = 분수 값에서 측정한 K를 뺀 값 (부호 있음. K에 단위가 없으므로 이 값에도 단위가 없습니다)
gen-preview-symbol-too-long = 미리보기 없음: 한 심볼이 { $samples } 샘플로 미리보기보다 깁니다. 보 레이트를 높이거나 샘플 레이트를 낮추세요.
gen-calc-fraction-too-big = K = { $k }은(는) 맞지 않습니다. LU는 최대 { $lu }, LG는 최대 { $lg } 샘플이어야 합니다. 더 작은 LU나 K를 고르세요.
