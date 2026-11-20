# Audio playback of a time span: errors and status messages.

playback-rate-too-low = Die Abtastrate ist zu niedrig zum Abspielen.
# $rate: the sample rate in Hz.
playback-rate-too-high = Die Wiedergabe ist für Audioraten gedacht; dieses Signal hat { $rate } Hz Abtastrate. Das gewünschte Band zuerst nach unten verschieben und neu abtasten.
playback-nothing = Nichts abzuspielen: einen Zeitbereich mit den V-Markern auswählen.
# $got and $total: seconds; $name: the signal, e.g. "n3".
playback-short-read = Nur { $got } s des { $total } s langen Bereichs von { $name } lesbar; die Datei wurde evtl. auf dem Datenträger geändert. Aufnahme erneut öffnen.
# $error: the audio system's error text.
playback-no-output = Audioausgabe kann nicht geöffnet werden: { $error }
playback-output-unavailable = Die Audioausgabe ist nicht verfügbar.
# $seconds: the length played.
playback-playing = Spiele { $seconds } s ab
# $seconds: the length played; $limit: the playback limit at this rate, in seconds.
playback-playing-first-markers = Spiele nur die ersten { $seconds } s ab (bei dieser Rate ist die Wiedergabe auf { $limit } s begrenzt); V1 an das Ende dieses Teils setzen, um den Rest zu hören.
playback-playing-first-view = Spiele nur die ersten { $seconds } s ab (bei dieser Rate ist die Wiedergabe auf { $limit } s begrenzt); V-Marker auf den Rest setzen, um ihn zu hören.
playback-bad-samples = { $count } Abtastwerte, die keine Zahlen waren, werden als Stille wiedergegeben.
