# Help > About and the error box.

about-title = Sobre o SigNexora
about-text = Uma reimplementação livre da interface e dos métodos de análise do Signals Analyzer 6.2.6.7. Sem vínculo com o seu autor.
# $licence: the licence expression, e.g. "MIT OR Apache-2.0".
about-licence = Licença: { $licence }
about-credits = Créditos
about-credit-sa = Signals Analyzer de SergUA6: o programa original, o seu manual e as notas de versão.
about-credit-egui = egui e eframe (MIT ou Apache-2.0).
about-credit-noto = Fontes Noto Sans JP e Noto Sans KR, SIL Open Font License 1.1.
about-credit-crates = rustfft, symphonia, rodio, hound, rhai e os demais crates Rust do Cargo.lock, sob as suas próprias licenças.
dialog-ok = OK
dialog-cancel = Cancelar

## SA's question before an ACF or CCF (6.2.4.2) and its limits.

# $seconds: the expected time; $megabytes: the memory needed (both with decimals).
dialogs-corr-confirm = Tempo previsto deste processo ~ { $seconds } segundos,
    e precisa de ~ { $megabytes } MB de memória livre.
    Pense bem e pressione 'Cancelar', ou 'OK' para continuar.
# $megabytes: the size over the limit; $limit: SA's limit, both in mBytes.
dialogs-corr-limit = Desculpe, precisa de ~ { $megabytes } MB de memória livre.
    O limite é { $limit } MB, altere o tamanho do fragmento!
# $count: SigNexora's largest selection in samples (for the plural); $samples: the
# same number with digit grouping.
dialogs-corr-too-long = { $count ->
    [one] Desculpe, o SigNexora trabalha aqui com no máximo { $samples } amostra, altere o tamanho do fragmento!
   *[other] Desculpe, o SigNexora trabalha aqui com no máximo { $samples } amostras, altere o tamanho do fragmento!
}
