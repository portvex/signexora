# OFDM module window, SA-OFDM (SA_UI_SPEC.md section 10).

# Top bar (SA's control names).
ofdm-min-br = Min Br-LS
    .hover = 찾을 심볼 속도의 하한, Bd (정수); 옆 칸은 가장 긴 LS
ofdm-max-br = Max Br-LS
    .hover = 찾을 심볼 속도의 상한, Bd (정수); 옆 칸은 가장 짧은 LS
ofdm-total-ls = Total LS
    .hover = 선택 구간에 든 가장 긴 길이 (Min Br-LS)의 심볼 수
ofdm-current-ls = Current LS
    .hover = 표시 중인 심볼 (Find CT 중에는 0)
ofdm-survey = Survey
    .hover = 개관: 아래 막대로 옮기는 1024점 스펙트럼 창이 Find CT의 시작 위치를 정함
ofdm-list-k = List of K
    .hover = Magic K 계산기: 가장 그럴듯한 K 값과 본래 샘플링 속도
    .disabled = 먼저 Find CT와 ADP를 실행하세요

# Controls (SA's control names).
ofdm-symbols = Symbols
    .hover = 상관 삼각형 (과 ADP 블록)에 겹치는 심볼 수; 20에서 40이 좋고 6 미만은 안 됨
ofdm-scale-fft = Scale FFT
    .hover = LU 스펙트럼 채널 막대의 너비
ofdm-shift = Shift Frequency
    .hover = FFT 전에 빼는 주파수, Hz; Enter로 적용
ofdm-vshift = VShift
    .hover = 한 채널만큼 가상 이동 (격자의 정확한 단계, 제한 없음)
ofdm-shift-step = 단계
    .hover = Shift Frequency를 { $step } Hz 증감
ofdm-br = Br
    .hover = 심볼 속도: ADP 값, 또는 입력 후 Enter
ofdm-adp = ADP
    .hover = 정확한 심볼 속도와 주파수 위치 (SA의 Get Br)
    .disabled = 먼저 Find CT를 실행하세요
ofdm-show = Show
    .hover = 표시한 채널을 심볼 동기와 함께 한 심볼씩 보기 (Break로 중지)
ofdm-break = Break
ofdm-find-ct = Find CT
    .hover = 상관 삼각형 찾기: LS, LG와 심볼 시작
ofdm-mode = Mode
    .hover = 성상도의 신호 모델
ofdm-model-a = A
    .hover = 모델 A: 각 채널은 생성된 그대로 (절대값)
ofdm-model-b = B
    .hover = 모델 B: 각 채널이 자기 파일럿 (상대값)
ofdm-model-c = C
    .hover = 모델 C: 빨간 마커의 파일럿을 기준으로 한 데이터
ofdm-each = Each
    .hover = N 심볼마다 하나씩 보기
ofdm-start-plus = Start+
    .hover = Show가 처음 보여 줄 심볼

# Plot texts (SA's formats).
ofdm-info-br = Br = { $br }
ofdm-info-sh = Sh = { $sh }
ofdm-info-magic = Magic K = { $num }/{ $den }
ofdm-info-k = * { $k } *
ofdm-info-lg = LG(CP) = { $lg }
ofdm-info-lu = LU = { $lu }
ofdm-info-up = CH_Up = { $ch }, Freq = { $freq }
ofdm-info-down = CH_Down = { $ch }, Freq = { $freq }
ofdm-info-sel = CH_Sel = { $count }, dFreqs = { $df }
ofdm-info-vshift = VShift = { $bins }
ofdm-line = LS={ $ls }; LG={ $lg }; Br={ $br }; Sh={ $sh }
ofdm-k-line = SR={ $sr }; LS={ $ls }; LU={ $lu }; LG={ $lg }; K={ $num }/{ $den }
ofdm-finish = 정상 종료, 시간 { $secs }초.
ofdm-stopped = 사용자가 중지, 시간 { $secs }초.
ofdm-wait = 당황하지 마세요! 기다려 주세요...
ofdm-wait-ct = 상관 삼각형을 찾는 중
ofdm-wait-adp = 정확한 Br 계산 중
ofdm-wait-k = "Magic K" 계산 중
ofdm-wait-left = 남은 시간: { $secs }초

# Plots.
ofdm-survey-plot = 개관 스펙트럼
ofdm-triangle-plot = 상관 삼각형
ofdm-triangle = 삼각형
    .hover = 좋은 (왼쪽) 경사를 클릭해 LU 창을 놓음; 꼭짓점은 부모 LU
ofdm-spectrum-plot = LU 스펙트럼
ofdm-spectrum = 스펙트럼
    .hover = 채널을 클릭하면 가까운 마커가 이동, 두 번 클릭하면 둘 다 놓임
ofdm-pane = 채널
    .hover = 채널 { $ch }, 심볼 수 { $count }
ofdm-k = K
    .hover = K = { $k }, LU 오차 { $err }, ICI { $ici } dB, { $lu }
ofdm-k-lu-even = 짝수 LU
ofdm-k-lu-odd = 홀수 LU

# Popup menus.
ofdm-sort-sr = SR로 정렬
ofdm-sort-k = K로 정렬
ofdm-sort-am = AM으로 정렬
ofdm-k-close = 닫기
ofdm-menu-cancel = 취소
ofdm-menu-reset = 결과 지우기
ofdm-menu-around = Br = { $rate } 주변을 범위로
    .hover = Min Br-LS와 Max Br-LS를 측정된 Br의 0,8배와 1,25배로 설정
ofdm-resample = { $rate } Hz로 리샘플
    .hover = 신호를 이 본래 속도로 리샘플 (새 창)

# Jobs and messages.
ofdm-job-ct = OFDM: Find CT
ofdm-job-br = OFDM: ADP
ofdm-job-k = OFDM: List of K
ofdm-job-show = OFDM: Show
ofdm-needs-ct = 먼저 Find CT를 실행하세요.
ofdm-needs-br = 먼저 ADP를 실행하세요.
ofdm-baseband-freed = 그 사이 창이 닫혔습니다. Find CT를 다시 실행하세요.
ofdm-check-settings = 설정을 확인하세요! Show에는 Symbols가 6 이상, Start+ 뒤에 10 심볼 이상이 필요합니다.
ofdm-failed = 검색 실패, 시간 { $secs }초.
ofdm-pick-busy = 실행 중인 작업이 끝나길 기다리거나 Break를 누른 뒤 줄을 고르세요.
ofdm-br-invalid = Br "{ $text }"은(는) 사용할 수 없습니다. 0보다 크고 { $max } Bd (fs/LU) 이하의 숫자를 입력하세요.
ofdm-br-invalid-nogrid = Br "{ $text }"은(는) 사용할 수 없습니다. 0보다 큰 숫자를 입력하세요.
ofdm-plot-empty = 데이터 없음: Find CT를 실행하세요.
ofdm-shift-invalid = Shift Frequency "{ $text }"은(는) 숫자가 아닙니다. 12.5처럼 Hz로 입력하세요.
