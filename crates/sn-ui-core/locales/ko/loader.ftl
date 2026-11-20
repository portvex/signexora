# SA's file loader, the "SA Open file" window, and its "SA - select channel"
# question. Keep the window titles, the SA control captions (Use options, Data
# start, Channels, Bits, Freq rate, Wave tag, Bits per sample, Data size, Ext.
# Info) and the list entries (All - I/Q, Ch - 1, 16, 24-A, ...) untranslated.

loader-title = SA Open file
loader-ok = 확인
loader-cancel = 취소

## Toolbar buttons.

loader-back = 뒤로
    .hover = 이전 폴더로 돌아가기
loader-up = 위로
    .hover = 한 폴더 위로 (Backspace)
loader-view = 보기 모드
    .hover = 보기 모드 변경: 목록 또는 자세히
loader-refresh = 새로 고침
    .hover = 목록 새로 고침

## The list.

loader-col-name = 이름
loader-col-type = 형식/확장자
loader-col-size = 크기
loader-col-date = 날짜
loader-col-time = 시간
loader-type-folder = 폴더
loader-type-link = F-Link
loader-popup-update = 새로 고침
loader-popup-auto-preview = 자동 미리 보기
loader-popup-copy = 복사
loader-popup-paste = 붙여넣기
loader-popup-cancel = 취소
loader-search = 빠른 검색
    .hover = 이름의 일부를 입력하면 그것을 포함한 첫 파일을 선택합니다
loader-filter-wav-mp3 = wav & mp3
loader-filter-wav = wav
loader-filter-mp3 = mp3
loader-filter-supported = 지원하는 모든 형식
loader-filter-all = *.*
# $path: what was typed or pasted.
loader-path-not-found = 경로를 찾을 수 없습니다: { $path }

## The info panel and the options.

loader-folder-link = 폴더 링크
# $value: a number from the file header.
loader-info-tag = Wave tag = { $value }
loader-info-channels = Channels = { $value }
loader-info-rate = Freq rate = { $value }
loader-info-bits = Bits per sample = { $value }
loader-info-size = Data size = { $value }
loader-info-no-header = 헤더 없음
loader-info-unreadable = 알 수 없는 형식
# $codec: the codec name of the file, e.g. "Intel DVI ADPCM (IMA ADPCM)".
loader-ext-info = Ext. Info: { $codec }
loader-use-options = Use options
    .hover = 헤더 대신 이 설정으로 파일을 읽습니다 (헤더 없는 파일에 필요)
loader-use-options-compressed = 압축 파일은 헤더에 따라 디코딩됩니다. 수동 설정으로 읽으면 잡음이 됩니다.
loader-use-options-headerless = 헤더가 없는 데이터는 이 설정으로만 읽을 수 있습니다.
loader-data-start = Data start
    .hover = 샘플 앞에서 건너뛸 바이트 수
loader-channels = Channels
    .hover = 불러올 채널, All은 처음 두 채널을 I/Q로 읽음
loader-bits = Bits
    .hover = 샘플 형식과 바이트 순서
loader-freq-rate = Freq rate
    .hover = 샘플 레이트, Hz
loader-in-file = 파일의 채널 수
    .hover = 헤더가 없는 이 파일에 교차 저장된 채널 수
# $error: why the file cannot be read.
loader-cannot-read = 이 파일을 읽을 수 없습니다: { $error }
loader-check-settings = 설정을 확인하십시오.

## The preview.

loader-preview-building = 미리 보기를 만드는 중...
loader-prescaler = 프리스케일러
    .hover = 프리스케일러: { $gain } dB. 최대 범위 (빨간 선) 를 넘는 샘플은 잘립니다.

## SA - select channel.

loader-select-channel-title = SA - select channel
loader-select-channel-group = 이 파일은 스테레오입니다. 선택하십시오
loader-channel-1 = 채널 - 1
loader-channel-2 = 채널 - 2
loader-all-samples = 모든 샘플
loader-break = 중단
loader-needs-options = 이 파일에는 읽을 수 있는 헤더가 없습니다. Use options를 켜고 형식을 설정하세요.
loader-no-file = 먼저 파일을 선택하세요.
loader-rate-invalid = Freq rate "{ $text }"이(가) 올바르지 않습니다. 48000처럼 0보다 크고 1000000000 Hz 이하로 입력하세요.
loader-data-start-invalid = Data start "{ $text }"이(가) 올바르지 않습니다. 0이나 44처럼 정수 바이트 수를 입력하세요.
loader-list-error = 폴더 { $path }을(를) 읽을 수 없습니다: { $error }. 권한을 확인하거나 다른 폴더를 선택하세요.
