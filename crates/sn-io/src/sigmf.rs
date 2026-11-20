//! SigMF metadata (`.sigmf-meta` + `.sigmf-data`), the open standard for SDR captures.

use std::path::{Path, PathBuf};

use crate::format::{Endian, SampleFormat};
use crate::{Error, Result, io_err};

/// The parts of a SigMF recording we use.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct SigmfMeta {
    pub(crate) data_path: PathBuf,
    pub(crate) datatype: String,
    pub(crate) format: SampleFormat,
    pub(crate) endian: Endian,
    pub(crate) complex: bool,
    /// Recorded channels (`core:num_channels`); complex channels count once.
    pub(crate) num_channels: u16,
    /// Bytes before the first sample in the data file (`core:header_bytes`).
    pub(crate) header_bytes: u64,
    pub(crate) sample_rate: f64,
    pub(crate) center_frequency: f64,
    pub(crate) description: Option<String>,
}

/// The `.sigmf-meta` and `.sigmf-data` paths for any of the two.
pub(crate) fn pair(path: &Path) -> Option<(PathBuf, PathBuf)> {
    let ext = path.extension()?.to_str()?;
    if ext != "sigmf-meta" && ext != "sigmf-data" {
        return None;
    }
    Some((
        path.with_extension("sigmf-meta"),
        path.with_extension("sigmf-data"),
    ))
}

/// Parse a SigMF `core:datatype` such as `cf32_le`, `ci16_le`, `ru8`.
pub(crate) fn parse_datatype(dt: &str) -> Result<(bool, SampleFormat, Endian)> {
    let unsupported = || Error::Unsupported(format!("SigMF datatype '{dt}'"));
    let (body, endian) = match dt.rsplit_once('_') {
        Some((b, "le")) => (b, Endian::Little),
        Some((b, "be")) => (b, Endian::Big),
        Some(_) => return Err(unsupported()),
        None => (dt, Endian::Little),
    };
    let (complex, kind) = if let Some(k) = body.strip_prefix('c') {
        (true, k)
    } else if let Some(k) = body.strip_prefix('r') {
        (false, k)
    } else {
        return Err(unsupported());
    };
    let format = match kind {
        "f64" => SampleFormat::F64,
        "f32" => SampleFormat::F32,
        "i32" => SampleFormat::I32,
        "i16" => SampleFormat::I16,
        "i8" => SampleFormat::I8,
        "u8" => SampleFormat::U8,
        _ => return Err(unsupported()),
    };
    Ok((complex, format, endian))
}

/// Read the metadata file. `data_path` is the data file next to it, used unless the
/// metadata names another one with `core:dataset`.
pub(crate) fn read(meta_path: &Path, data_path: &Path) -> Result<SigmfMeta> {
    let text = std::fs::read_to_string(meta_path).map_err(io_err(meta_path))?;
    parse(&text, meta_path, data_path)
}

fn parse(text: &str, meta_path: &Path, data_path: &Path) -> Result<SigmfMeta> {
    let json: serde_json::Value = serde_json::from_str(text)
        .map_err(|e| Error::Malformed(format!("SigMF metadata is not valid JSON: {e}")))?;
    let global = json
        .get("global")
        .ok_or_else(|| Error::Malformed("SigMF metadata has no global section".into()))?;
    let datatype = global
        .get("core:datatype")
        .and_then(serde_json::Value::as_str)
        .ok_or_else(|| Error::Malformed("SigMF metadata has no core:datatype".into()))?
        .to_owned();
    let (complex, format, endian) = parse_datatype(&datatype)?;
    let sample_rate = global
        .get("core:sample_rate")
        .and_then(serde_json::Value::as_f64)
        .ok_or_else(|| Error::Malformed("SigMF metadata has no core:sample_rate".into()))?;
    let num_channels = match global.get("core:num_channels") {
        None => 1,
        Some(v) => v
            .as_u64()
            .and_then(|n| u16::try_from(n).ok())
            .filter(|&n| n >= 1 && (!complex || n <= u16::MAX / 2))
            .ok_or_else(|| {
                Error::Malformed(format!("SigMF core:num_channels is not a valid count: {v}"))
            })?,
    };
    let captures = json
        .get("captures")
        .and_then(serde_json::Value::as_array)
        .map_or(&[][..], Vec::as_slice);
    let header_of = |c: &serde_json::Value| -> Result<u64> {
        match c.get("core:header_bytes") {
            None => Ok(0),
            Some(v) => v.as_u64().ok_or_else(|| {
                Error::Malformed(format!("SigMF core:header_bytes is not a byte count: {v}"))
            }),
        }
    };
    let header_bytes = captures.first().map_or(Ok(0), header_of)?;
    for c in captures.iter().skip(1) {
        if header_of(c)? != 0 {
            return Err(Error::Unsupported(
                "SigMF recordings with headers between captures are not supported; \
                 convert the data file to plain samples first"
                    .into(),
            ));
        }
    }
    let center_frequency = captures
        .first()
        .and_then(|c| c.get("core:frequency"))
        .and_then(serde_json::Value::as_f64)
        .unwrap_or(0.0);
    let data_path = match global.get("core:dataset") {
        None => data_path.to_owned(),
        Some(v) => {
            let name = v.as_str().map(Path::new).filter(|p| {
                let mut parts = p.components();
                matches!(parts.next(), Some(std::path::Component::Normal(_)))
                    && parts.next().is_none()
            });
            let Some(name) = name else {
                return Err(Error::Unsupported(format!(
                    "SigMF core:dataset must name a file next to the metadata file, not {v}"
                )));
            };
            meta_path.with_file_name(name)
        }
    };
    Ok(SigmfMeta {
        data_path,
        datatype,
        format,
        endian,
        complex,
        num_channels,
        header_bytes,
        sample_rate,
        center_frequency,
        description: global
            .get("core:description")
            .and_then(serde_json::Value::as_str)
            .map(str::to_owned),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn datatypes() {
        assert_eq!(
            parse_datatype("cf32_le").unwrap(),
            (true, SampleFormat::F32, Endian::Little)
        );
        assert_eq!(
            parse_datatype("ri16_be").unwrap(),
            (false, SampleFormat::I16, Endian::Big)
        );
        assert_eq!(
            parse_datatype("cu8").unwrap(),
            (true, SampleFormat::U8, Endian::Little)
        );
        assert!(parse_datatype("cu16_le").is_err());
        assert!(parse_datatype("xf32_le").is_err());
    }

    #[test]
    fn pairs_meta_and_data() {
        let (m, d) = pair(Path::new("/x/rec.sigmf-data")).unwrap();
        assert_eq!(m, Path::new("/x/rec.sigmf-meta"));
        assert_eq!(d, Path::new("/x/rec.sigmf-data"));
        assert!(pair(Path::new("rec.wav")).is_none());
    }

    fn meta(global_extra: &str, capture_extra: &str) -> Result<SigmfMeta> {
        let text = format!(
            r#"{{"global":{{"core:datatype":"ci16_le","core:sample_rate":1000{global_extra}}},
            "captures":[{{"core:sample_start":0{capture_extra}}}]}}"#
        );
        parse(
            &text,
            Path::new("/x/a.sigmf-meta"),
            Path::new("/x/a.sigmf-data"),
        )
    }

    #[test]
    fn channels_header_and_dataset() {
        let m = meta("", "").unwrap();
        assert_eq!((m.num_channels, m.header_bytes), (1, 0));
        assert_eq!(m.data_path, Path::new("/x/a.sigmf-data"));

        let m = meta(
            r#","core:num_channels":3,"core:dataset":"cap.bin""#,
            r#","core:header_bytes":512"#,
        )
        .unwrap();
        assert_eq!((m.num_channels, m.header_bytes), (3, 512));
        assert_eq!(m.data_path, Path::new("/x/cap.bin"));

        assert!(meta(r#","core:num_channels":0"#, "").is_err());
        assert!(meta(r#","core:dataset":"../other.bin""#, "").is_err());
        assert!(meta(r#","core:dataset":"/etc/passwd""#, "").is_err());
        assert!(meta("", r#","core:header_bytes":-4"#).is_err());
    }
}
