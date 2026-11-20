# Help > About and the error box.

about-title = Acerca de SigNexora
about-text = Una reimplementación libre de la interfaz y los métodos de análisis de Signals Analyzer 6.2.6.7. Sin relación con su autor.
# $licence: the licence expression, e.g. "MIT OR Apache-2.0".
about-licence = Licencia: { $licence }
about-credits = Créditos
about-credit-sa = Signals Analyzer de SergUA6: el programa original, su manual y sus notas de versión.
about-credit-egui = egui y eframe (MIT o Apache-2.0).
about-credit-noto = Fuentes Noto Sans JP y Noto Sans KR, SIL Open Font License 1.1.
about-credit-crates = rustfft, symphonia, rodio, hound, rhai y los demás crates de Rust de Cargo.lock, con sus propias licencias.
dialog-ok = Aceptar
dialog-cancel = Cancelar

## SA's question before an ACF or CCF (6.2.4.2) and its limits.

# $seconds: the expected time; $megabytes: the memory needed (both with decimals).
dialogs-corr-confirm = Tiempo previsto de este proceso ~ { $seconds } segundos,
    y necesita ~ { $megabytes } MB de memoria libre.
    Piénselo bien y pulse 'Cancelar', o 'Aceptar' para continuar.
# $megabytes: the size over the limit; $limit: SA's limit, both in mBytes.
dialogs-corr-limit = Lo siento, necesita ~ { $megabytes } MB de memoria libre.
    El límite es { $limit } MB, cambie el tamaño del fragmento.
# $count: SigNexora's largest selection in samples (for the plural); $samples: the
# same number with digit grouping.
dialogs-corr-too-long = { $count ->
    [one] Lo siento, SigNexora trabaja aquí con { $samples } muestra como máximo, cambie el tamaño del fragmento.
   *[other] Lo siento, SigNexora trabaja aquí con { $samples } muestras como máximo, cambie el tamaño del fragmento.
}
