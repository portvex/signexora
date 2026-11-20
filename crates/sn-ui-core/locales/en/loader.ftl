# SA's file loader, the "SA Open file" window, and its "SA - select channel"
# question. Keep the window titles, the SA control captions (Use options, Data
# start, Channels, Bits, Freq rate, Wave tag, Bits per sample, Data size, Ext.
# Info) and the list entries (All - I/Q, Ch - 1, 16, 24-A, ...) untranslated.

loader-title = SA Open file
loader-ok = Ok
loader-cancel = Cancel

## Toolbar buttons.

loader-back = Back
    .hover = Back to the previous folder
loader-up = Up
    .hover = Up one folder (Backspace)
loader-view = View mode
    .hover = Change the view mode: list or details
loader-refresh = Update
    .hover = Update the list

## The list.

loader-col-name = Name
loader-col-type = Type/Ext
loader-col-size = Size
loader-col-date = Date
loader-col-time = Time
loader-type-folder = Folder
loader-type-link = F-Link
loader-popup-update = Update
loader-popup-auto-preview = Auto preview
loader-popup-copy = Copy
loader-popup-paste = Paste
loader-popup-cancel = Cancel
loader-search = Quick search
    .hover = Type part of a file name to select the first file that contains it
loader-filter-wav-mp3 = wav & mp3
loader-filter-wav = wav
loader-filter-mp3 = mp3
loader-filter-supported = all supported
loader-filter-all = *.*
# $path: what was typed or pasted.
loader-path-not-found = Path not found: { $path }

## The info panel and the options.

loader-folder-link = Folder's link
# $value: a number from the file header.
loader-info-tag = Wave tag = { $value }
loader-info-channels = Channels = { $value }
loader-info-rate = Freq rate = { $value }
loader-info-bits = Bits per sample = { $value }
loader-info-size = Data size = { $value }
loader-info-no-header = No header
loader-info-unreadable = Unknown format
# $codec: the codec name of the file, e.g. "Intel DVI ADPCM (IMA ADPCM)".
loader-ext-info = Ext. Info: { $codec }
loader-use-options = Use options
    .hover = Read the file with these settings instead of its header (needed for headerless files)
loader-use-options-compressed = Compressed files are decoded from their header; raw options would read noise.
loader-use-options-headerless = Headerless data can only be read with these options.
loader-data-start = Data start
    .hover = Bytes to skip before the samples
loader-channels = Channels
    .hover = The channel to load, or All for the first two as I/Q
loader-bits = Bits
    .hover = Sample format and byte order
loader-freq-rate = Freq rate
    .hover = Sample rate, Hz
loader-in-file = Ch. in file
    .hover = Channels interleaved in this headerless file
# $error: why the file cannot be read.
loader-cannot-read = Cannot read this file: { $error }
loader-check-settings = Please check settings!

## The preview.

loader-preview-building = Building preview...
loader-prescaler = Prescaler
    .hover = Prescaler: { $gain } dB. Samples above full scale (red lines) are clipped.

## SA - select channel.

loader-select-channel-title = SA - select channel
loader-select-channel-group = This file stereo, please select
loader-channel-1 = Channel - 1
loader-channel-2 = Channel - 2
loader-all-samples = All samples
loader-break = Break
loader-needs-options = This file has no header SigNexora can read: tick Use options and set its format.
loader-no-file = Select a file first.
loader-rate-invalid = Freq rate "{ $text }" is not valid: enter a rate above 0 and up to 1000000000 Hz, such as 48000.
loader-data-start-invalid = Data start "{ $text }" is not valid: enter a whole number of bytes, such as 0 or 44.
loader-list-error = Cannot read folder { $path }: { $error }. Check its permissions or choose another folder.
