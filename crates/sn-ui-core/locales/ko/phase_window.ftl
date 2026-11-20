# The PhasePlane form (SA_UI_SPEC.md section 6): spectrum mode toolbar, ISD
# corrector, phase viewer, status bar, demodulator output and the LoadFile dialog.
# The control captions are SA's own English names (SA showed them in English under
# every Windows language): keep them as they are; translate the hints and dialogs.

pp-fmin = Fmin
    .hover = 표시된 최저 주파수, Hz
pp-fmax = Fmax
    .hover = 표시된 최고 주파수, Hz
pp-scale = Scale
    .hover = 표시할 대역의 비율. 스펙트럼 아래의 스크롤 막대로 이동합니다
pp-fft-size = FFTSize
    .hover = 평균 스펙트럼의 FFT 크기. 클수록 세밀합니다 (약 1 Hz까지)
pp-low-level = LowLevel
    .hover = 표시할 최저 레벨
pp-stretch = Stretch
    .hover = 스펙트럼을 전체 높이로 늘리기
pp-mode = Mode
    .hover = AM: 포락선으로 보정, X^N: 신호의 N제곱으로 보정
pp-blocks = Blocks
    .hover = 보정기가 평균하는 신호 블록 수
pp-corr-off = Corr-OFF
    .hover = ISD 보정기가 꺼져 있습니다. Auto를 눌러 신호를 보정하세요
pp-corr-on = Corr-ON
    .hover = ISD 보정기가 켜져 있습니다 (SaveAs가 보정된 신호를 저장)
pp-save-as = SaveAs
    .hover = 보정된 신호를 파일로 저장
pp-auto = Auto
    .hover = 선택한 블록으로 보정을 한 단계 실행
pp-break = Break
pp-reset = Reset
    .hover = 보정기 초기화 (끄기)
pp-carrier = Carrier
    .hover = 반송파 주파수, Hz (마지막 Sc 측정값)
pp-br = BR
    .hover = 변조 속도, Bd (마지막 Br 측정값)
pp-nary = n-Ary
    .hover = 변조 차수, 2-32 (1: 반송파 추적 없음)
pp-synch-br = Synch BR
    .hover = 클록 동기
pp-nonstop = NonStop
    .hover = 연속 동작: 파일 끝에서 처음부터 다시
pp-rrc = RRC filter
    .hover = 루트 레이즈드 코사인 필터. 슬라이더로 alpha 설정
pp-pll = PLL
    .hover = 반송파 루프 속도
pp-pll-slow = Slow
pp-pll-medium = Medium
pp-pll-quick = Quick
pp-pll-hard = Hard
pp-auto-reset = AutoReset
    .hover = NonStop 사용 시: 파일 끝에서 발진기와 PLL 루프 초기화 (드리프트 신호용)
pp-transitions = Transitions
    .hover = 점 사이의 천이 (궤적) 그리기
pp-life = Life
    .hover = 동적 표시: 마지막 Points... 개의 심볼만 남김
pp-diff = Diff
    .hover = 0: 절대 각도, 1-3: 위상차 (pi/4 DQPSK 같은 상대 방식)
pp-offset-detected = Offset mode detected
pp-angles = Angles
    .hover = 보정기 그래프에 각도 히스토그램 작성 (Synch BR 필요)
pp-vectors = Vectors of dem
    .hover = 복조기의 위치 표시. 영 벡터는 하늘색
pp-output-dem = Output Dem as
    .hover = 복조된 스트림을 절대 각도 코드로 텍스트 창에 출력
pp-trimming = Trimming { $value }
    .hover = 위상 미세 조정, 도
pp-delay = Delay { $value }
    .hover = 심볼당 실제 지연, 초 (매우 짧은 신호용)
pp-points = Points... { $value }
    .hover = Life 사용 시 동시에 표시할 점 수
pp-contrast = Contrast { $value }
    .hover = 점의 밝기
pp-start = Start
    .hover = 성상도 시작 또는 정지. 정지 후에는 파일 처음부터 다시 시작
pp-clear = Clear
    .hover = 표시 지우기
pp-load-file = LoadFile
    .hover = 열린 파일 중 하나를 위상 평면에 불러오기
pp-status-f = F = { $value }
pp-status-fl = Fl = { $value }
pp-status-fu = Fu = { $value }
pp-status-df = dF = { $value }
pp-status-angle-d = Angle_D = { $value }
pp-status-angle-i = Angle_I = { $value }
pp-dem-view-mode = View mode: diff { $diff }
pp-dem-mode = Dem mode: { $mode }
pp-dem-absolute = Data presented as absolute angles!
pp-dem-coding = The coding positions of angles:
pp-dem-data = Data:

pp-isd-n = 보정기가 사용하는 신호의 고조파
    .hover = 보정기가 사용하는 신호의 고조파
pp-isd-b = 켜기: 개선된 알고리즘, 끄기: 약간 떨어지지만 빠름 (AM 모드에서는 효과 없음)
    .hover = 켜기: 개선된 알고리즘, 끄기: 약간 떨어지지만 빠름 (AM 모드에서는 효과 없음)
pp-isd-break = 현재 단계 중지
    .hover = 현재 단계 중지
pp-alpha = RRC 필터의 롤오프 alpha
    .hover = RRC 필터의 롤오프 alpha
pp-quality = 성상도의 관례적 신호 대 잡음비, dB
    .hover = 성상도의 관례적 신호 대 잡음비, dB
pp-load-title = SA - PhasePlane 파일 선택
pp-ok = 확인
pp-cancel = 취소
pp-check-settings = 설정을 확인하십시오
pp-run-limited = 처음 { $symbols }개 심볼만 복조됩니다. 뒷부분을 분석하려면 소노그램에서 그 부분에 V 마커를 놓고 { sw-hint-copy }를 사용한 뒤 사본에서 { sw-hint-phase-plane }을(를) 여십시오.
pp-copy-panel = 텍스트 복사
pp-copy-codes = Start 이후 모든 코드 복사
pp-eye = 아이 다이어그램
    .hover = 성상도 대신 아이 다이어그램 (I, 두 심볼) 표시
pp-job-corrector = ISD 보정기 스펙트럼
pp-job-isd = ISD 보정기
pp-job-spectrum = PhasePlane 스펙트럼
# $count symbols, $offset Hz, $rings radii, $iq_offset symbols.
pp-quality-details = 심볼 { $count }개, 루프 오프셋 { $offset } Hz, 링 [{ $rings }], I/Q 오프셋 { $iq_offset } 심볼
pp-source-gone = 불러온 파일이 더 이상 세션에 없습니다. LoadFile로 작업 파일을 다시 불러오세요.
pp-load-gone = 선택한 파일이 닫혔습니다. 다른 작업 파일을 고르세요.
