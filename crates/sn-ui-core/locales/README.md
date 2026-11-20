# Translations

The interface text of every SigNexora front end lives in
[Fluent](https://projectfluent.org/) files in this crate (`sn-ui-core`):
`locales/<language>/<module>.ftl`. `build.rs` embeds every file, so adding a
file or a language needs no code change. Languages: `en` (source and fallback),
`fr`, `es`, `de`, `pt`, `uk`, `ru`, `ja`, `ko`.

## In code

```rust
use sn_ui_core::t; // or `use crate::t;` inside sn-ui-core

ui.button(t!("sono-fft-size"));
ui.label(t!("loader-rate", rate = format!("{rate:.1}")));
ui.label(t!("jobs-running", count = jobs.len()));
```

- One `.ftl` file per source module, named after it (`sonogram.rs` uses
  `sonogram.ftl`); the front ends share the files of the modules they both
  have. Keys start with the module prefix (`sono-`, `wave-`, ...)
  so files never collide. A tooltip can be an attribute: `sono-fft-size.hover`.
- Pass counts that choose a plural form as integers. Format every other number
  in Rust exactly as before and pass it as a string.
- Do not translate: units (Hz, dB, Bd, s), measurement abbreviations used by
  Signals Analyzer (Br, dT, V1/V2, H1/H2, ADP, CT, LS, LU, LG, K), file
  names, error text from other crates (pass it as an argument), keyboard key
  names, and colour map names.
- Build a sentence in one message with arguments; do not glue translated
  fragments together, word order differs between languages.

## In translations

- Every language must have the same files, message ids and attributes as
  English, and may use only variables that the English message uses. Use
  plural selectors where the language needs them (`uk`, `ru` have one, few,
  many and other).
- No em or en dashes, arrows, bullets, check marks or stars; use plain hyphens.
- Keep translations about as short as English: they sit in toolbars and menus.

`cargo test -p sn-ui-core i18n` checks all of this, and that every `t!` key
used in the source of this crate and of the front ends (`sa-app`, `sn-gui`; the
list is `SOURCE_DIRS` in `src/i18n.rs`) exists in English.
