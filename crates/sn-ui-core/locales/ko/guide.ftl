# The workflow guide: SA's analysis method as a checklist with buttons.

guide-title = 작업 가이드
guide-no-signal = 먼저 녹음을 여십시오 (파일 > 파일 열기...).
# $signal: the active window's signal, which the buttons act on.
guide-acts-on = 버튼 대상: { $signal }
# $key: a key name. Shown after a menu path only when that key acts on the guide's signal.
guide-key = , 키 { $key }

## Steps: the message is the step's title, .text its explanation.

guide-step-record = 1. 올바르게 녹음
    .text = 수신기 AGC를 끄고 클리핑이나 편집 없이 16비트 PCM WAV(또는 I/Q)로 녹음합니다. 압축 파일이나 8비트 파일은 열 때 경고가 표시됩니다.
guide-step-quality = 2. 녹음 품질 확인
    .text = WF 창을 엽니다 (소노그램을 오른쪽 클릭 > { sw-popup-waveform }). 클리핑, 8비트 계단, 직류 성분, AGC 출렁임, 끊김을 확인합니다. Sum을 켜면 진폭 분포가 보입니다.
guide-step-isolate = 3. 신호 분리
    .text = 소노그램에서 H 마커를 신호 주위에 놓습니다. 그다음 도구 모음의 { sw-hint-band-select }와 { sw-hint-shift-base }를 쓰고, 대역이 넓으면 R로 더 낮은 레이트로 리샘플링합니다.
guide-step-rate = 4. 변조 속도 찾기
    .text = 검파기를 적용하고 클록 라인을 마커로 둘러싼 뒤 팝업 메뉴의 { sw-popup-measure }를 사용합니다{ $br }. PSK/QAM/FSK에는 { sw-hint-am }, MFSK에는 { sw-hint-modified-am }, 또는 { sw-popup-adp }를 시도합니다.
guide-step-family = 5. 변조 방식 분석
    .text = FSK/MFSK: { sw-hint-phase-detector }, 그다음 WF 창에서 1/3 심볼의 { edits-median }, Sum, 톤 찾기, 주파수 세기. PSK/QAM: { sw-hint-power }로 차수와 반송파를 찾고 (그 창에서 { sw-popup-measure }{ $sc }), 그다음 PhasePlane. 왜곡된 신호에는 먼저 ISD 보정기. OFDM: CT 찾기, Br 얻기, 채널, 그다음 원래 레이트를 위한 K 목록.
guide-step-structure = 6. 구조 찾기
    .text = 프레임 주기에는 { sw-hint-acf }, 조각을 전체에서 찾으려면 { sw-hint-ccf }, 그다음 V 마커를 한 프레임에 놓고 팝업 메뉴에서 { sw-popup-measure } > { raster-dt-menu-item }을(를) 선택해 dT(프레임 주기)를 기록하고 그 주기로 { guide-btn-raster }를 열어 프레임과 동기어를 봅니다.

## Step buttons.

guide-btn-waveform = 파형
guide-btn-sonogram = 소노그램
guide-btn-am-detector = AM 검파기
guide-btn-modified-am = 수정 AM
guide-btn-phase-detector = 위상 검파기
guide-btn-demod = FSK/MFSK 복조기
# $n: the power, a number.
guide-btn-power = 거듭제곱 { $n }
    .hover = { $n }제곱한 새 신호를 만듭니다. PSK-{ $n } 신호는 반송파의 { $n }배 위치에 라인이 나타납니다
guide-btn-phase-plane = PhasePlane
guide-btn-raster = 래스터 (VMW)
guide-btn-adp = ADP
    .hover = V 마커 구간(또는 신호 전체)에서 키잉 속도를 자동으로 추정합니다(ADP)
guide-btn-ofdm = OFDM
    .hover = OFDM 도구를 엽니다(CT 찾기, Br 구하기, K 목록)
guide-open-view = 활성 창의 신호를 이 보기로 엽니다
guide-new-signal = 활성 창의 신호로 새 신호를 만듭니다 (새 창에서 열림)
# $kind: Br, Sc or dT (untranslated).
guide-not-measured = { $kind }: 아직 측정되지 않음
