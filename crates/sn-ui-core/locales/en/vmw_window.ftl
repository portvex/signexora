# The VMW mode of the WF window. SA's control names (Y scale, As time, Bright,
# WaveForm, TS, Info) and the units S, ms, us, ns are names: keep them.

vmw-strobe = Strobe
    .hover = Strobe: show every Nth period (1 to 1000); magnifies small period errors into slopes
vmw-y-scale = Y scale
    .hover = Height of one raster line in pixels
vmw-as-time = As time
    .hover = The line period follows the span between the WF V markers
    .disabled = Set the V markers in WF first
vmw-waveform = WaveForm
    .hover = Back to WF
vmw-info = Info
    .hover = Show dT, S and the line rate TSs
vmw-ts = TS
    .hover = Line period S:ms.us.ns. Left click a digit: +1, right click: -1
vmw-period-slider = TS
    .hover = Line period (coarse, logarithmic)
vmw-status = dT
    .hover = Span between the V markers and its rate

## Popup menu (SigNexora tools).

vmw-popup-start-v1 = Start the raster at the first WF V marker
vmw-popup-from-dt = Line period = the last measured dT
vmw-popup-colours = Colour map
vmw-popup-sa-colours = Signals Analyzer
