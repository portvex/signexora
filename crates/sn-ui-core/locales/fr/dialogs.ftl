# Help > About and the error box.

about-title = À propos de SigNexora
about-text = Une réimplémentation libre de l'interface et des méthodes d'analyse de Signals Analyzer 6.2.6.7. Sans lien avec son auteur.
# $licence: the licence expression, e.g. "MIT OR Apache-2.0".
about-licence = Licence : { $licence }
about-credits = Remerciements
about-credit-sa = Signals Analyzer de SergUA6 : le programme original, son manuel et ses notes de version.
about-credit-egui = egui et eframe (MIT ou Apache-2.0).
about-credit-noto = Polices Noto Sans JP et Noto Sans KR, SIL Open Font License 1.1.
about-credit-crates = rustfft, symphonia, rodio, hound, rhai et les autres crates Rust de Cargo.lock, sous leurs propres licences.
dialog-ok = OK
dialog-cancel = Annuler

## SA's question before an ACF or CCF (6.2.4.2) and its limits.

# $seconds: the expected time; $megabytes: the memory needed (both with decimals).
dialogs-corr-confirm = Durée prévue de ce traitement ~ { $seconds } secondes,
    mémoire libre nécessaire ~ { $megabytes } Mo.
    Réfléchissez bien, puis appuyez sur 'Annuler', ou sur 'OK' pour continuer.
# $megabytes: the size over the limit; $limit: SA's limit, both in mBytes.
dialogs-corr-limit = Désolé, il faut ~ { $megabytes } Mo de mémoire libre.
    La limite est de { $limit } Mo, veuillez changer la taille du fragment !
# $count: SigNexora's largest selection in samples (for the plural); $samples: the
# same number with digit grouping.
dialogs-corr-too-long = { $count ->
    [one] Désolé, SigNexora traite ici au plus { $samples } échantillon, veuillez changer la taille du fragment !
   *[other] Désolé, SigNexora traite ici au plus { $samples } échantillons, veuillez changer la taille du fragment !
}
