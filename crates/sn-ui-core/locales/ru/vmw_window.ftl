# The VMW mode of the WF window. SA's control names (Y scale, As time, Bright,
# WaveForm, TS, Info) and the units S, ms, us, ns are names: keep them.

vmw-strobe = Strobe
    .hover = Строб: каждый N-й период (от 1 до 1000); превращает малые ошибки периода в наклоны
vmw-y-scale = Y scale
    .hover = Высота строки растра в пикселях
vmw-as-time = As time
    .hover = Период строки равен интервалу между маркерами V в WF
    .disabled = Сначала установите маркеры V в WF
vmw-waveform = WaveForm
    .hover = Назад в WF
vmw-info = Info
    .hover = Показать dT, S и частоту строк TSs
vmw-ts = TS
    .hover = Период строки S:ms.us.ns. Левый щелчок по цифре: +1, правый: -1
vmw-period-slider = TS
    .hover = Период строки (грубо, логарифмически)
vmw-status = dT
    .hover = Интервал между маркерами V и его частота

## Popup menu (SigNexora tools).

vmw-popup-start-v1 = Начать растр с первого маркера V в WF
vmw-popup-from-dt = Период строки = последний измеренный dT
vmw-popup-colours = Палитра
vmw-popup-sa-colours = Signals Analyzer
