//! Embeds the Fluent translation files.
//!
//! Every `locales/<language>/<module>.ftl` file is listed in
//! `$OUT_DIR/locales.rs` as `(language, file name, contents)` through
//! `include_str!`, so adding a file or a language needs no code change.

use std::fmt::Write as _;
use std::path::Path;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let root = Path::new(&std::env::var("CARGO_MANIFEST_DIR")?).join("locales");
    println!("cargo:rerun-if-changed={}", root.display());

    let mut languages: Vec<_> = std::fs::read_dir(&root)?
        .filter_map(Result::ok)
        .map(|e| e.path())
        .filter(|p| p.is_dir())
        .collect();
    languages.sort();

    let mut out =
        String::from("/// `(language, file name, contents)` of every embedded Fluent file.\n");
    out.push_str("pub(crate) static FILES: &[(&str, &str, &str)] = &[\n");
    for dir in languages {
        println!("cargo:rerun-if-changed={}", dir.display());
        let Some(lang) = dir.file_name().and_then(|n| n.to_str()).map(str::to_owned) else {
            continue;
        };
        let mut files: Vec<_> = std::fs::read_dir(&dir)?
            .filter_map(Result::ok)
            .map(|e| e.path())
            .filter(|p| p.extension().is_some_and(|x| x == "ftl"))
            .collect();
        files.sort();
        for file in files {
            println!("cargo:rerun-if-changed={}", file.display());
            let Some(name) = file.file_name().and_then(|n| n.to_str()) else {
                continue;
            };
            let path = file.to_str().ok_or("a locale path is not valid UTF-8")?;
            writeln!(out, "    ({lang:?}, {name:?}, include_str!({path:?})),")?;
        }
    }
    out.push_str("];\n");

    let dest = Path::new(&std::env::var("OUT_DIR")?).join("locales.rs");
    std::fs::write(dest, out)?;
    Ok(())
}
