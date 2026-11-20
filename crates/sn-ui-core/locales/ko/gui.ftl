# Texts of the Windows 95 front end (sn-gui) that the egui app does not have:
# message box buttons, the appearance schemes, the system menu of a child
# window, credits and notices. An & marks the mnemonic letter (Alt+letter).

## Message box buttons.

gui-button-yes = 예(&Y)
gui-button-no = 아니요(&N)

## Options > Theme: SigNexora's themes as Windows appearance schemes.

gui-scheme-standard = Windows 표준
gui-scheme-high-contrast-black = 고대비 검정
gui-scheme-high-contrast-white = 고대비 흰색

## The system menu of a child window.

gui-sys-restore = 이전 크기로(&R)
gui-sys-move = 이동(&M)
gui-sys-size = 크기 조정(&S)
gui-sys-minimize = 최소화(&N)
gui-sys-maximize = 최대화(&X)
gui-sys-close = 닫기(&C)
gui-sys-next = 다음(&T)

## Help > About: credits of the Windows 95 front end.

gui-about-credit-gpui = GPUI (Zed Industries, Apache-2.0).
gui-about-credit-w95fa = W95FA 글꼴 (Alina Sava, FontsArena), SIL Open Font License 1.1.
gui-about-credit-selawik = Selawik 글꼴 (Microsoft), SIL Open Font License 1.1 (Segoe UI, Tahoma 또는 Trebuchet MS가 설치되지 않은 경우).
gui-about-credit-linux-fonts = Cantarell 및 Noto Sans 글꼴, SIL Open Font License 1.1, DejaVu Sans 글꼴, Bitstream Vera 라이선스 (Linux 스타일).

## Notices.

# $path: the settings file; $error: why it could not be written.
gui-prefs-not-saved = 설정을 { $path }에 저장하지 못했습니다: { $error }. 폴더에 쓸 수 있는지 확인하십시오.
# $path: the settings file.
gui-prefs-not-encoded = 설정을 { $path }에 저장하지 못했습니다: 파일 형식으로 변환할 수 없었습니다. 프로그램 오류이므로 보고해 주십시오. 이전 설정 파일은 그대로 유지됩니다.

## The signal generator (OCG) window and its calculators.

gui-gen-preset = 프리셋(&P)...
gui-gen-generate = 생성(&G)
gui-gen-generate-save = 생성 후 저장(&S)...
gui-gen-calculators = 계산기(&C)...
gui-gen-calc-title = OCG 계산기
# $text: what was typed in a number field.
gui-gen-not-number = "{ $text }"은(는) 숫자가 아닙니다
gui-gen-not-whole = "{ $text }"은(는) 정수가 아닙니다
# $min, $max: the limits of the field.
gui-gen-range = { $min }에서 { $max } 사이여야 합니다

## Help > Use help: the Windows 95 keys.

gui-keys-menu = 메뉴 모음 선택
gui-keys-menu-letter = 밑줄 친 글자의 메뉴를 열거나 버튼을 누름
gui-keys-next-window = 다음 창을 앞으로 가져오기 (Shift: 이전 창)
