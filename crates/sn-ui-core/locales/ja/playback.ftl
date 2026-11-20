# Audio playback of a time span: errors and status messages.

playback-rate-too-low = サンプルレートが低すぎて再生できません。
# $rate: the sample rate in Hz.
playback-rate-too-high = 再生はオーディオレートの信号用です。この信号のサンプルレートは { $rate } Hz です。目的の帯域を下にシフトし、先にリサンプルしてください。
playback-nothing = 再生するものがありません。V マーカーで時間区間を選択してください。
# $got and $total: seconds; $name: the signal, e.g. "n3".
playback-short-read = { $name } の { $total } s の区間のうち { $got } s しか読み込めませんでした。ディスク上のファイルが変更された可能性があります。録音を開き直してください。
# $error: the audio system's error text.
playback-no-output = オーディオ出力を開けません: { $error }
playback-output-unavailable = オーディオ出力を使用できません。
# $seconds: the length played.
playback-playing = { $seconds } s を再生中
# $seconds: the length played; $limit: the playback limit at this rate, in seconds.
playback-playing-first-markers = 最初の { $seconds } s のみ再生中です (このレートでは再生は { $limit } s まで)。続きを聞くには V1 をこの部分の終わりに移動してください。
playback-playing-first-view = 最初の { $seconds } s のみ再生中です (このレートでは再生は { $limit } s まで)。続きを聞くには残りの部分に V マーカーを設定してください。
playback-bad-samples = 数値でない { $count } 個のサンプルは無音として再生されます。
