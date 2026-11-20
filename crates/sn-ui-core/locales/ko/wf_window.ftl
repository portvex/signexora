# The WF (waveform) window: SA's top toolbar, bottom toolbar hints, popup menu and
# result box. SA's control names (LPF, RRC, Sum, Info, VMW, Br, Z+, Z-) and the
# result box text are names: keep them.

## Top toolbar, left to right.

wf-lpf = LPF
    .hover = 저역 통과 차단 주파수(Hz) 또는 RRC 필터의 보 레이트(입력하거나 선택)
wf-copy-br = <==
    .hover = 측정한 Br을 LPF 칸에 복사
wf-rrc = RRC
    .hover = RRC 필터 켜기 또는 끄기(켜면 LPF 단추가 실행)
wf-rrc-alpha = RRC
    .hover = RRC 롤오프
wf-freq = 0-0
    .hover = 클릭: V 마커 사이 짧은 조각의 주파수와 온전한 주기 수
wf-vmw = VMW
    .hover = VMW로 전환(신호를 줄 주기로 접어 표시)
wf-sum = Sum
    .hover = 값의 히스토그램 표시(다음에 팝업 메뉴의 주파수 세기)
wf-info = Info
    .hover = 결과 표시(t1, t2, dT, S, Br)

## Bottom toolbar hints, left to right.

wf-hint-zoom-in = 확대(시간)
wf-hint-zoom-out = 축소(시간)
wf-hint-scale-up = 세로 배율 크게
wf-hint-scale-down = 세로 배율 작게
wf-hint-zoom-plus = Z+: V 마커 사이로 확대
wf-hint-zoom-minus = Z-: 확대를 한 단계 되돌리기
wf-hint-lpf = LPF 필터 실행(RRC가 켜져 있으면 RRC 필터)
wf-hint-revert = 원래대로 되돌리기
wf-hint-synth = FSK/MFSK 역합성
wf-hint-save = 16비트 WAV로 저장 (오른쪽 클릭: float WAV, DataFile, SigMF)
wf-hint-copy = V 마커 사이 조각을 새 창에 복사
wf-hint-demod = 조각을 FSK/MFSK 복조기로 보내기
wf-hint-fit = 세로 배율을 신호에 맞추기
wf-hint-acf = 조각의 자기 상관(새 창)
wf-hint-ccf = 조각과 전체 신호의 교차 상관(여기에 그림)

## Why a button is disabled.

wf-need-revert = 되돌릴 것이 없습니다. 필터도 CCF도 없습니다.
wf-need-derived = 이 창은 이미 저장된 열린 파일을 보여 줍니다. 저장은 여기서 만든 결과용입니다.

## Popup menu.

wf-popup-count-freqs = 주파수 세기
wf-popup-more = SigNexora 도구
    .hover = SA의 WF 창에 없던 SigNexora 도구
wf-popup-operations = 작업

## The Count freqs result box (SA's text, not translated). $size: a whole number;
## $value: a number with two decimals.

wf-label-grid = Freq grid: { $size }
wf-label-delta = Delta: { $value } Hz
wf-count-needs-h = 먼저 톤 주위에 H 마커를 놓으세요.
wf-count-tones-busy = Find tones가 아직 실행 중입니다. 끝난 뒤 Count freqs를 다시 실행하세요.
wf-count-freqs-failed = 주파수 그리드를 찾지 못했습니다 ({ $error }). 톤 주위의 H 마커를 넓히거나 축소해 더 많은 톤을 보이게 하세요.
wf-copy-br-value = Br = { $br } Bd를 LPF 칸에 복사
wf-copy-br-missing = 먼저 Br을 측정하세요 (소노그램 메뉴: Auto define params)
