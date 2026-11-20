# SA's file loader, the "SA Open file" window, and its "SA - select channel"
# question. Keep the window titles, the SA control captions (Use options, Data
# start, Channels, Bits, Freq rate, Wave tag, Bits per sample, Data size, Ext.
# Info) and the list entries (All - I/Q, Ch - 1, 16, 24-A, ...) untranslated.

loader-title = SA Open file
loader-ok = Aceptar
loader-cancel = Cancelar

## Toolbar buttons.

loader-back = Atrás
    .hover = Volver a la carpeta anterior
loader-up = Subir
    .hover = Subir una carpeta (Retroceso)
loader-view = Modo de vista
    .hover = Cambiar el modo de vista: lista o detalles
loader-refresh = Actualizar
    .hover = Actualizar la lista

## The list.

loader-col-name = Nombre
loader-col-type = Tipo/Ext
loader-col-size = Tamaño
loader-col-date = Fecha
loader-col-time = Hora
loader-type-folder = Carpeta
loader-type-link = F-Link
loader-popup-update = Actualizar
loader-popup-auto-preview = Vista previa automática
loader-popup-copy = Copiar
loader-popup-paste = Pegar
loader-popup-cancel = Cancelar
loader-search = Búsqueda rápida
    .hover = Escriba parte de un nombre para seleccionar el primer archivo que lo contenga
loader-filter-wav-mp3 = wav & mp3
loader-filter-wav = wav
loader-filter-mp3 = mp3
loader-filter-supported = todos los admitidos
loader-filter-all = *.*
# $path: what was typed or pasted.
loader-path-not-found = Ruta no encontrada: { $path }

## The info panel and the options.

loader-folder-link = Enlace de carpeta
# $value: a number from the file header.
loader-info-tag = Wave tag = { $value }
loader-info-channels = Channels = { $value }
loader-info-rate = Freq rate = { $value }
loader-info-bits = Bits per sample = { $value }
loader-info-size = Data size = { $value }
loader-info-no-header = Sin cabecera
loader-info-unreadable = Formato desconocido
# $codec: the codec name of the file, e.g. "Intel DVI ADPCM (IMA ADPCM)".
loader-ext-info = Ext. Info: { $codec }
loader-use-options = Use options
    .hover = Leer el archivo con estos ajustes en lugar de su cabecera (necesario sin cabecera)
loader-use-options-compressed = Los archivos comprimidos se decodifican según su cabecera; las opciones en bruto leerían ruido.
loader-use-options-headerless = Los datos sin cabecera solo se pueden leer con estas opciones.
loader-data-start = Data start
    .hover = Bytes que se saltan antes de las muestras
loader-channels = Channels
    .hover = El canal que se carga, o All para los dos primeros como I/Q
loader-bits = Bits
    .hover = Formato de muestra y orden de bytes
loader-freq-rate = Freq rate
    .hover = Frecuencia de muestreo, Hz
loader-in-file = Canales del archivo
    .hover = Canales intercalados en este archivo sin cabecera
# $error: why the file cannot be read.
loader-cannot-read = No se puede leer este archivo: { $error }
loader-check-settings = Compruebe los ajustes.

## The preview.

loader-preview-building = Creando la vista previa...
loader-prescaler = Preescalador
    .hover = Preescalador: { $gain } dB. Las muestras por encima de la escala completa (líneas rojas) se recortan.

## SA - select channel.

loader-select-channel-title = SA - select channel
loader-select-channel-group = Este archivo es estéreo, elija
loader-channel-1 = Canal - 1
loader-channel-2 = Canal - 2
loader-all-samples = Todas las muestras
loader-break = Interrumpir
loader-needs-options = Este archivo no tiene una cabecera legible: marque Use options y defina su formato.
loader-no-file = Seleccione antes un archivo.
loader-rate-invalid = Freq rate "{ $text }" no es válido: escriba una frecuencia mayor que 0 y hasta 1000000000 Hz, por ejemplo 48000.
loader-data-start-invalid = Data start "{ $text }" no es válido: escriba un número entero de bytes, por ejemplo 0 o 44.
loader-list-error = No se puede leer la carpeta { $path }: { $error }. Revise sus permisos o elija otra carpeta.
