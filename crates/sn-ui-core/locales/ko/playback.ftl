# Audio playback of a time span: errors and status messages.

playback-rate-too-low = 샘플링 레이트가 너무 낮아 재생할 수 없습니다.
# $rate: the sample rate in Hz.
playback-rate-too-high = 재생은 오디오 대역 신호용입니다. 이 신호의 샘플링 레이트는 { $rate } Hz입니다. 먼저 관심 대역을 아래로 이동하고 리샘플링하십시오.
playback-nothing = 재생할 내용이 없습니다. V 마커로 시간 구간을 선택하십시오.
# $got and $total: seconds; $name: the signal, e.g. "n3".
playback-short-read = { $name }의 { $total } s 구간 중 { $got } s만 읽을 수 있었습니다. 디스크의 파일이 변경되었을 수 있습니다. 녹음 파일을 다시 여십시오.
# $error: the audio system's error text.
playback-no-output = 오디오 출력을 열 수 없습니다: { $error }
playback-output-unavailable = 오디오 출력을 사용할 수 없습니다.
# $seconds: the length played.
playback-playing = 재생 중: { $seconds } s
# $seconds: the length played; $limit: the playback limit at this rate, in seconds.
playback-playing-first-markers = 처음 { $seconds } s만 재생합니다(이 레이트에서는 재생이 { $limit } s로 제한됨). 나머지를 들으려면 V1을 이 부분의 끝으로 옮기십시오.
playback-playing-first-view = 처음 { $seconds } s만 재생합니다(이 레이트에서는 재생이 { $limit } s로 제한됨). 나머지를 들으려면 V 마커를 나머지 부분에 설정하십시오.
playback-bad-samples = 숫자가 아닌 샘플 { $count }개는 무음으로 재생됩니다.
