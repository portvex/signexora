# Main window (MDI frame): messages, File menu extras, the WF window's toolbar
# and popup menu, and the confirmation dialogs.

## View kinds (child windows)

app-tab-sonogram = 소노그램
app-tab-waveform = 파형
app-tab-raster = 래스터 (VMW)
app-tab-phase = PhasePlane
app-tab-demod = FSK/MFSK 복조기
app-tab-ofdm = OFDM

## Lists of names: "A", "A and B", "A, B and C"

app-list-separator = { ", " }
app-list-and = { $rest } 및 { $last }

## Background jobs (shown in the status bar while they run)

app-job-loading = 불러오는 중: { $name }
app-job-overview = 파형 개요 생성 중
app-job-saving = 저장 중: { $path }
app-job-measure-line = 라인 주파수 측정 중
app-job-adp = 보 레이트 추정 중 (ADP)
app-job-writing = 쓰는 중: { $path }
app-job-replay = 세션 재실행 중
app-job-script = 스크립트 실행 중
app-job-find-tones = 톤 찾는 중
app-job-playback = 재생 준비 중
app-job-demodulator = 복조기

## Status messages

app-ready = 준비됨
app-signal-gone = 해당 신호가 더 이상 존재하지 않습니다.
app-set-h-markers-first = 먼저 라인 주위에 H 마커를 설정하십시오.
app-sc-unmappable = { $edit } 단계를 거쳐 Sc를 매핑할 수 없습니다. 출력이 입력과 주파수 관계가 없습니다. { $edit } 단계 이전에 반송파를 측정하십시오.
app-recorded = { $kind } = { $value } ({ $node }, { $tool })
app-recorded-uncertain = { $kind } = { $value } +/- { $uncertainty } ({ $node }, { $tool })
app-not-recorded = { $kind } 기록 안 됨: { $error }
app-mapped-onto = { $from }, { $node }에 매핑됨
app-session-save-running = 세션 저장이 이미 진행 중입니다
app-export-report-failed = 보고서 내보내기: { $error }
app-save-session-failed = 세션 저장: { $error }
app-open-session-failed = 세션 열기: { $error }
app-script-error = 오류: { $error }
app-replay-skipped = { $what ->
    [markers] { $node } ({ $label }): 저장된 마커를 읽을 수 없어 건너뛰었습니다: { $error }
    [tones] { $node } ({ $label }): 저장된 톤을 읽을 수 없어 건너뛰었습니다: { $error }
    [grid] { $node } ({ $label }): 저장된 톤 그리드를 읽을 수 없어 건너뛰었습니다: { $error }
   *[adp] { $node } ({ $label }): 저장된 ADP 후보를 읽을 수 없어 건너뛰었습니다: { $error }
}
app-session-restored = 세션 복원됨
app-session-restored-problems = 세션 복원됨, 문제 { $count }개 (창 참조)
app-wrote-report = 보고서 작성됨: { $path }
app-saved = 저장됨: { $path }
app-saved-session = 세션 저장됨: { $path }
app-saved-session-replaced = 세션 저장됨: { $path } (파일에 있던 세션을 덮어썼습니다)
app-cancel-too-late = { $name }: 취소가 적용되기 전에 완료되어 결과를 유지했습니다.
app-job-cancelled = { $name }: 취소됨
app-job-failed = { $name }: { $error }
app-playback-cancelled = 재생 취소됨
app-derived-opened = { $label }: 새 신호 { $node }, { $view }에서 열림
app-derived-opened-mapped = { $label }: 새 신호 { $node }, { $view }에서 열림. 마커를 새 축으로 옮겼습니다
app-derived-opened-cleared = { $label }: 새 신호 { $node }, { $view }에서 열림. 새 신호의 시간 축 또는 주파수 축이 달라 일부 마커를 지웠습니다
app-derive-failed = { $label }: { $error }
app-measured-line = { $kind } = { $value } +/- { $uncertainty } ({ $node }, 라인 측정; { $level } dBFS, 대역보다 { $snr } dB 높음)
app-measured-line-mapped = { $kind } = { $value } +/- { $uncertainty } ({ $node }, 라인 측정; { $level } dBFS, 대역보다 { $snr } dB 높음), { $line } Hz 라인을 거듭제곱 N = { $power } 기준으로 역매핑
app-loaded = 불러옴: { $label }, 샘플 { $samples }개, { $rate } Hz
app-loaded-notes = 불러옴: { $label }, 샘플 { $samples }개, { $rate } Hz. { $notes }
app-script-no-signals = 스크립트에서 생성된 신호가 없습니다: { $path }
app-script-finished = 스크립트 완료: { $path }
app-adp-none = ADP가 선택 영역에서 키잉 속도를 찾지 못했습니다
app-tones-found = { $node }에서 톤 { $count }개를 찾았습니다
app-markers-restored = 마커 복원됨: { $node }
app-closed-tab-stopped = { $name } 창을 닫아 실행 중이던 { $jobs } 작업을 중지했습니다. 창을 다시 열고 다시 실행하십시오.
app-nothing-to-play = 재생할 내용이 없습니다. 소노그램을 열거나 V 마커를 설정하십시오.
app-find-tones-failed = 톤 찾기: { $error }

## Confirmation before work is lost

app-confirm-open-title = 세션 열기
app-confirm-open-question = 현재 세션을 바꾸시겠습니까?
app-confirm-replace = 바꾸기
app-confirm-script-title = 스크립트 실행
app-confirm-script-question = 스크립트의 신호가 현재 세션을 대체합니다.
app-confirm-quit-title = 종료
app-confirm-quit-unsaved = 종료하기 전에 세션을 저장하시겠습니까?
app-confirm-quit-without-saving = 저장하지 않고 종료
app-confirm-quit-writing = 아직 파일을 쓰는 중입니다.
app-confirm-quit-anyway = 그래도 종료
app-confirm-lost = 저장하지 않은 신호 { $signals }개와 측정값 { $measurements }개가 닫힙니다. 이 작업은 되돌릴 수 없습니다.
app-confirm-write-running = { $name } 작업이 아직 실행 중입니다. 지금 종료하면 중단됩니다.
app-confirm-save-first = 먼저 세션 저장
app-confirm-save-first-ask = 먼저 세션 저장...
app-confirm-wait = 기다리기
app-confirm-keep-open = 앱 계속 열어 두기
app-cancel = 취소

## File dialogs

app-dialog-save-as = { $format } 형식으로 저장
app-dialog-save-session-as = 다른 이름으로 세션 저장
app-filter-session = SigNexora 세션
app-dialog-export-report = 분석 보고서 내보내기
app-filter-report = 보고서
app-dialog-open-session = 세션 열기
app-dialog-run-script = 스크립트 실행
app-filter-rhai = Rhai 스크립트

## File menu

app-menu-file = 파일
app-menu-open-session = 세션 열기...
    .hover = 세션의 신호가 현재 세션을 대체합니다
app-nothing-to-save = 저장할 내용 없음
app-menu-save-session = 세션 저장
app-menu-save-session-ask = 세션 저장...
app-menu-save-session-choose = 세션을 저장할 파일 선택
app-menu-save-session-to = 저장 위치: { $path }
app-menu-save-session-as = 다른 이름으로 세션 저장...
app-menu-export-report = 분석 보고서 내보내기
app-menu-report-html = HTML...
app-menu-report-markdown = Markdown...

## File menu extras and the WF popup menu

app-menu-run-script = 스크립트 실행 (Rhai)...
    .hover = 스크립트를 실행합니다. 스크립트의 신호가 현재 세션을 대체합니다
app-resynth = FSK/MFSK 재합성
    .hover = 톤 찾기로 얻은 톤과 Br로 깨끗한 신호를 다시 생성합니다
    .disabled = 먼저 톤을 찾고(파형 합산) Br을 측정하십시오.
app-menu-measure = 측정
app-measure-hint = 라인 주위에 H 마커를 설정하십시오(구간은 V 마커로).
app-measure-br = 라인을 보 레이트로 측정 (Br)
app-measure-sc = 라인을 반송파로 측정 (Sc)
    .hover = 거듭제곱 N 이후의 라인을 반송파로 역매핑합니다(N으로 나누고 이동을 되돌림)
app-measure-adp = 보 레이트 자동 추정 (ADP)
    .hover = V 마커 구간 또는 전체 신호를 사용합니다
app-measure-dt-disabled = 먼저 V 마커를 설정하십시오.

## Hints

app-no-signals-hint = 녹음을 여십시오 (파일 > 파일 열기... ({ $shortcut }), 또는 창에 파일을 끌어 놓기).
app-open-in = 열기...
    .hover = 이 신호를 다른 보기나 도구에서 엽니다

## Sonogram toolbar

app-sono-shear = Shear
    .hover = FFT 프레임 사이 간격(FFT 크기에 대한 비율). 작을수록 프레임이 더 겹쳐 시간 해상도가 높아지지만 계산이 느려집니다
app-sono-image = Image
    .hover = 가장 강한 성분 아래로 표시할 동적 범위
app-sono-hres-hover = 주파수 재할당으로 스펙트럼 라인을 선명하게 합니다(약한 클록 라인을 찾는 데 유용). 톤의 레벨은 유지되며, 빈 폭 기준으로 표시되므로 잡음은 HRes를 껐을 때보다 약 2~6 dB 낮게 보입니다(윈도우에 따라 다름).
app-sono-alc-hover = 각 열을 가장 강한 성분 기준으로 정규화합니다(표시 전용)
app-sono-combine = 축소 시
    .hover = 축소 보기에서 한 이미지 열에 들어가는 FFT 프레임을 합치는 방법
app-sono-peak = 피크
    .hover = 축소 시: 짧은 버스트를 보이게 유지
app-sono-mean = 평균
    .hover = 축소 시: 평균, 잡음이 더 부드러움

## Waveform toolbar

app-wave-show-q = Q 표시
app-find-tones = 톤 찾기
    .hover = 히스토그램의 피크: 위상 검파기 출력의 톤 주파수

## Zoom and playback

app-play = 재생
    .hover = V 마커 구간 또는 보이는 구간을 재생합니다 ({ $shortcut })
app-play-stop = 정지
    .hover = 재생 정지 ({ $shortcut })

## Child windows

app-tab-signal-gone = 이 신호는 더 이상 존재하지 않습니다.

## Windows

app-replay-problems-title = 세션 복원 중 문제 발생
app-script-output = 스크립트 출력: { $path }
app-script-no-output = (출력 없음)
# $lines: the lines kept; $dropped: the lines left out.
app-script-output-truncated = 출력이 { $lines }줄에서 잘렸습니다. 나머지 { $dropped }줄은 보관되지 않았습니다.

## Closing a window, the sonogram toolbar

app-confirm-close-tab-title = 창을 닫을까요?
# $name: the window title; $jobs: the job names, joined.
app-confirm-close-tab-question = { $name }을(를) 닫으면 실행 중인 { $jobs } 작업이 중지됩니다. 그래도 닫으시겠습니까?
app-confirm-close-and-stop = 닫고 중지
app-confirm-keep-tab = 열어 두기
# $prev, $next: the keys that show the previous and next signal.
app-select-signal-first = 먼저 창을 앞으로 가져오십시오 ({ $prev } 또는 { $next } 키로 신호를 차례로 표시).
app-sono-fft = SizeFFT
    .hover = FFT 크기: 클수록 주파수 분해능은 세밀해지고 시간 분해능은 거칠어집니다
app-sono-window = WinF
    .hover = 각 FFT의 창 함수(SA의 WinF)
app-sono-colormap = 색상
    .hover = 소노그램 이미지의 색상표
app-confirm-close-all-title = 모든 창 닫기
app-confirm-close-all-question = 세션에서 모든 신호와 측정값이 제거됩니다.
app-confirm-close-all = 모두 닫기
app-confirm-close-all-stops = { $name }이(가) 아직 실행 중입니다. 모든 창을 닫으면 중지됩니다.
app-replay-session-skipped = { $what ->
    [windows] 저장된 창 배치를 읽을 수 없어 결과마다 창을 엽니다: { $error }
   *[counters] 저장된 이름 카운터를 읽을 수 없어 새 이름의 번호가 겹칠 수 있습니다: { $error }
}
app-replay-windows-missing = 신호를 재생하지 못해 저장된 창 { $count }개를 열지 않았습니다
app-drop-loader-open = 파일 대화 상자가 열려 있습니다. 완료하거나 취소한 뒤 파일을 다시 놓으세요
app-drop-first-only = 파일 { $count }개를 놓았습니다. 첫 번째 파일 { $name }만 엽니다
app-sono-target = { $active ->
    [yes] 활성 소노그램 창을 변경합니다.
   *[no] 새 소노그램 창의 기본값을 변경합니다.
}
app-edit-unrecordable = 재생을 위해 세션에 기록할 수 없습니다 ({ $error })
app-empty-open = 녹음 열기... ({ $key })
app-empty-drop = 또는 여기에 파일을 놓으세요 (File > Open file...로도 열 수 있음).
app-empty-guide = 작업 안내 ({ $key })에서 분석을 단계별로 볼 수 있습니다.
# $action: what the chooser was for, e.g. "Open session"; $error: why it failed.
app-chooser-failed = { $action }: 파일 선택 창을 표시하지 못했습니다 ({ $error }). 다시 시도하거나 시스템 파일 대화 상자 서비스(데스크톱 포털)가 실행 중인지 확인하십시오.

## The tools that record a measurement, by the id stored in the session.

measure-tool-adp = 매개변수 자동 결정 (ADP)
measure-tool-adp-coarse = 매개변수 자동 결정, 대략값 (FFT 빈의 절반 이내)
measure-tool-measure-line = 주파수 측정 (측정선)
measure-tool-v-markers = V 마커 (dT)
measure-tool-ofdm-adp = OFDM 도구, Get Br (ADP)
measure-tool-script-record = 스크립트, record()
