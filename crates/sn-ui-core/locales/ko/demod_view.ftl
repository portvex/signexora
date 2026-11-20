# FSK-2 / 범용 MFSK 복조기 창 (SA_UI_SPEC.md 8절).

# Text panel.
demod-heading =
    SA - MFSK dem
    Levels: { $levels }
    Fd: { $fs }
    Br: { $br }
    Data as: { $form }
    Data:
demod-as-default = 기본 대응표
demod-as-raw = 원시 데이터
demod-as-bits = 비트, 자연 부호
demod-as-gray = 비트, 그레이 부호
demod-as-c4fm = APCO-25 C4FM 다이비트
demod-polarity-inverted = [극성: 반전, 반전 C4FM 표로 비트화]
demod-polarity-normal = [극성: 정상]
demod-polarity-unknown = [극성: 알 수 없음, 정상 C4FM 표로 비트화]
demod-truncated = (처음 20000자; 복사와 다른 이름으로 저장은 전체 대상)
demod-nothing-yet = 아직 복조된 것이 없습니다. Br, 레벨, 마커를 확인하세요.

# Jobs.
demod-job-track = 복조기: 주파수 궤적
demod-job = FSK/MFSK 복조 중

# Top bar.
demod-rrc = RRC
    .hover = 판정 전 루트 레이즈드 코사인 필터; 칸은 차단 주파수 Hz (기본값 Br)
demod-cutoff = 차단
    .hover = RRC 차단 주파수 Hz: 값을 입력하거나 목록에서 선택
demod-alpha = 알파
    .hover = RRC 롤오프, 0,05에서 1 (Enter 또는 칸을 벗어나면 적용)
demod-br = Br
    .hover = 변조 속도; Enter로 적용
demod-br-step = 단계
    .hover = Br을 { $step } 만큼 증감
demod-auto = A
    .hover = 위상과 비트 자동 동기
demod-auto-button = A
    .hover = 지금 위상을 자동으로 찾아 유지 (자동 모드 해제)
demod-exactly = <= Exactly
    .hover = 미세 샘플링 위상 (한 심볼의 +-{ $span } %)
demod-roughly = <= Roughly
    .hover = 한 심볼 안의 대략적인 샘플링 위상
demod-levels = Levels
    .hover = 레벨 수, 2에서 256
demod-raw = Raw
    .hover = 레벨을 x로 구분한 16진 바이트로 출력

# Bottom bar.
demod-back-fast = 10 뒤로
    .hover = 10쪽 뒤로
demod-back = 뒤로
    .hover = 이전 쪽
demod-forward = 앞으로
    .hover = 다음 쪽
demod-forward-fast = 10 앞으로
    .hover = 10쪽 앞으로
demod-page = 쪽
    .hover = 표시 중인 쪽 (전체 { $count }); Enter로 이동
demod-scale-up = 확대
    .hover = 세로 배율 확대
demod-scale-down = 축소
    .hover = 세로 배율 축소
demod-progress = 진행
    .hover = 계산 진행 상황 (오른쪽 클릭으로 중지)
demod-stop = 중지
demod-filter = 필터
    .hover = RRC 필터 적용
demod-revert = 되돌리기
    .hover = 필터 없는 신호로 되돌리기
demod-d = D
    .hover = 비트열을 텍스트 칸에 출력

# Plot.
demod-plot-name = 복조기 레벨
demod-info = Br = { $baud } | 클럭 { $clock } % | 품질 { $quality } | 아이 { $eye } | 심볼 수 { $count }
demod-menu-tones = 찾은 톤을 레벨로 ({ $count })
    .hover = 균등한 구역 대신 WF의 Sum으로 찾은 톤을 레벨로 사용
    .disabled = 먼저 톤을 찾으세요: WF, Sum, 톤 찾기
demod-menu-average = 평균 필터
    .hover = RRC가 꺼져 있을 때 반 심볼 동안 적분 (사각 FSK)
demod-menu-adaptive = 적응 레벨
    .hover = 레벨 중심이 데이터의 느린 드리프트를 따라가게 함
demod-menu-info = Info
demod-menu-reset-view = 세로 배율 초기화
demod-menu-cancel = 취소

# Text panel popup.
demod-menu-copy = 복사
demod-menu-save = 다른 이름으로 저장...
demod-menu-data-as = 데이터 형식
demod-save-title = 복조 데이터 저장
demod-save-filter = 텍스트
demod-save-failed = { $path }에 쓸 수 없습니다: { $error }
demod-cutoff-invalid = RRC 차단 주파수 "{ $text }"은(는) 양수가 아닙니다. Br처럼 Hz로 입력하세요.
demod-waiting-cutoff = 올바른 RRC 차단 주파수를 기다리는 중입니다. 표시된 결과는 이전 설정의 결과입니다.
demod-br-invalid = Br "{ $text }"은(는) 양수가 아닙니다. 사용 중인 Br을 유지합니다.
