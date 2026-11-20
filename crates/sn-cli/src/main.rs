//! `sn-cli`: headless entry point for SigNexora.

use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use clap::{Args, Parser, Subcommand, ValueEnum};
use sn_core::Signal;
use sn_io::{Endian, LoadOptions, RawFormat, SampleFormat, WavFormat, probe, write_wav};
use sn_synth::{
    GeneratorSpec, MfskParams, PRESET_NAMES, PskParams, SignalKind, add_awgn_esn0, add_awgn_snr,
    mfsk, psk, random_symbols, tone,
};

#[derive(Debug, Parser)]
#[command(name = "sn-cli", version, about = "SigNexora command-line tool")]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Debug, Subcommand)]
enum Command {
    /// Generate a test signal with exactly known parameters.
    Synth {
        #[command(subcommand)]
        kind: SynthKind,
    },
    /// Generate an OCG-style test signal (tones, noise, FSK, PSK/QAM, C4FM, OFDM with
    /// channel impairments) from a JSON spec or a named preset. Start from
    /// `--print-spec` (or `--preset NAME --print-spec`), edit the JSON, then
    /// `generate --spec spec.json -o out.wav`.
    Generate(GenerateArgs),
    /// Describe a recording: format, channels, rate, length and quality notes.
    Info {
        /// File to inspect.
        file: PathBuf,
    },
    /// Apply a chain of edits (JSON, as recorded in the session graph) and save the
    /// result. Example: `--edit '{"op":"band_select","lo":1500,"hi":2100}'`
    Process {
        /// Input recording.
        input: PathBuf,
        /// Output file (.wav, or .sigmf-meta to keep fractional rates).
        #[arg(short, long)]
        output: PathBuf,
        /// One edit as JSON; repeat for a chain, applied in order.
        #[arg(long = "edit", required = true)]
        edits: Vec<String>,
    },
    /// Measure the precise frequency of the strongest line in a band, after optional
    /// edits (e.g. a detector or power-of-N). Example for a clock line:
    /// `--edit '{"op":"am_detector"}' --lo 30 --hi 200`
    Measure {
        /// Recording to analyse.
        file: PathBuf,
        /// Lower band edge in Hz.
        #[arg(long)]
        lo: f64,
        /// Upper band edge in Hz.
        #[arg(long)]
        hi: f64,
        /// Edits applied first, as JSON.
        #[arg(long = "edit")]
        edits: Vec<String>,
        #[command(flatten)]
        span: SpanArgs,
    },
    /// Find FSK/MFSK tones (histogram peaks) and the tone-grid size (SA: Count freqs),
    /// the same way `fsk-demod` finds its levels when none are given.
    /// Usually after `--edit '{"op":"phase_detector"}' --edit '{"op":"median","window":N}'`.
    Tones {
        /// Recording to analyse.
        file: PathBuf,
        /// Edits applied first, as JSON.
        #[arg(long = "edit")]
        edits: Vec<String>,
        /// Gaussian smoothing of the histogram, in value units (Hz).
        #[arg(long, default_value_t = 4.0)]
        smooth: f64,
    },
    /// PhasePlane: carrier and clock recovery; prints angle clusters, rings and codes.
    Phase {
        /// Recording to analyse.
        file: PathBuf,
        /// Carrier in Hz.
        #[arg(long)]
        carrier: f64,
        /// Keying rate in Bd.
        #[arg(long)]
        baud: f64,
        /// Modulation order (n-Ary).
        #[arg(long, default_value_t = 4)]
        order: u32,
        /// Phase-difference display order (Diff 0-3).
        #[arg(long, default_value_t = 0, value_parser = clap::value_parser!(u8).range(0..=3))]
        diff: u8,
        /// Edits applied first, as JSON.
        #[arg(long = "edit")]
        edits: Vec<String>,
        #[command(flatten)]
        span: SpanArgs,
    },
    /// Estimate the keying rate automatically (SA's ADP), after optional edits.
    Adp {
        /// Recording to analyse.
        file: PathBuf,
        /// Edits applied first, as JSON.
        #[arg(long = "edit")]
        edits: Vec<String>,
        #[command(flatten)]
        span: SpanArgs,
    },
    /// Demodulate FSK/MFSK from a frequency track (SA's universal MFSK demodulator).
    /// Usually after `--edit '{"op":"phase_detector"}'`. Example for APCO-25:
    /// `--edit '{"op":"phase_detector"}' --baud 4800 --rrc 0.2 --lo 10400 --hi 14000
    /// --count 4 --format dibits --map c4fm`
    FskDemod(FskDemodArgs),
    /// Analyse OFDM with a cyclic prefix: Find CT (LS, LU, LG), Get Br (precise symbol
    /// rate, K, frequency offset), the occupied channels and the K calculator (List of
    /// K plus the orthogonality check). Example:
    /// `sn-cli ofdm rec.wav --min-br 40 --max-br 120`
    Ofdm {
        /// Recording to analyse.
        file: PathBuf,
        /// Lowest symbol rate to search, in Bd (SA: MinBr).
        #[arg(long)]
        min_br: f64,
        /// Highest symbol rate to search, in Bd (SA: MaxBr).
        #[arg(long)]
        max_br: f64,
        /// Symbols folded into the correlation triangle.
        #[arg(long, default_value_t = 40)]
        symbols: usize,
        /// First sample of the search.
        #[arg(long, default_value_t = 0)]
        start: usize,
        /// How many K candidates get the orthogonality check (0 skips it).
        #[arg(long, default_value_t = 8)]
        check: usize,
        /// Edits applied first, as JSON.
        #[arg(long = "edit")]
        edits: Vec<String>,
    },
    /// List the strongest spectral components of a file. The spectrum is the mean of
    /// up to 2000 frames spread evenly over the whole file.
    Peaks {
        /// Recording to analyse.
        file: PathBuf,
        /// FFT size.
        #[arg(long, default_value_t = 8192)]
        fft: usize,
        /// How many peaks to list.
        #[arg(long, default_value_t = 5)]
        count: usize,
    },
    /// Run a Rhai script (see the sn-script crate docs for the functions available).
    /// Relative paths in the script are resolved against the script's directory.
    Script {
        /// Script file (.rhai).
        file: PathBuf,
        /// Also save what the script did as a session file, for replay and reports.
        #[arg(long)]
        session: Option<PathBuf>,
        /// Limit on script operations (loops, calls, expressions).
        #[arg(long, default_value_t = sn_script::DEFAULT_MAX_OPERATIONS)]
        max_operations: u64,
    },
    /// Write an analysis report for a saved session. Without --html or --md the
    /// Markdown report is printed.
    SessionReport {
        /// Session file (.json).
        session: PathBuf,
        /// Write a standalone HTML report here.
        #[arg(long)]
        html: Option<PathBuf>,
        /// Write a Markdown report here.
        #[arg(long)]
        md: Option<PathBuf>,
    },
    /// Rebuild every signal of a saved session from its sources and print a summary.
    /// Relative source paths are resolved against the session file's directory.
    Replay {
        /// Session file (.json).
        session: PathBuf,
    },
    /// Time the work behind the viewer on a file: open, waveform overview, and one
    /// screen of sonogram columns zoomed out and zoomed in.
    Bench {
        /// File to open (raw files need --raw-format).
        file: PathBuf,
        /// Screen width in columns.
        #[arg(long, default_value_t = 1600)]
        width: usize,
        /// FFT size.
        #[arg(long, default_value_t = 1024)]
        fft: usize,
        /// Describe a headerless file: sample format, e.g. i16, i8, u8, f32.
        #[arg(long)]
        raw_format: Option<String>,
        /// Channels of a raw file.
        #[arg(long, default_value_t = 2)]
        raw_channels: u16,
        /// Sample rate of a raw file in Hz.
        #[arg(long, default_value_t = 10_000_000.0)]
        raw_rate: f64,
    },
}

#[derive(Debug, Args)]
struct GenerateArgs {
    /// Generator spec as JSON (the format `--print-spec` prints).
    #[arg(long, conflicts_with_all = ["preset", "kind"])]
    spec: Option<PathBuf>,
    /// A named preset (see `--list-presets`).
    #[arg(long, conflicts_with = "kind")]
    preset: Option<String>,
    /// The default spec of one kind of signal.
    #[arg(long, value_enum)]
    kind: Option<KindArg>,
    /// List the preset names and stop.
    #[arg(long, exclusive = true)]
    list_presets: bool,
    /// Print the spec (default, preset or file) as JSON instead of generating it.
    #[arg(long, conflicts_with = "output")]
    print_spec: bool,
    /// Replace the seed of the data and the noise.
    #[arg(long)]
    seed: Option<u64>,
    /// Output file: .wav (a complex signal becomes stereo I/Q), or .sigmf-meta to
    /// keep the centre frequency and fractional sample rates.
    #[arg(short, long, required_unless_present_any = ["print_spec", "list_presets"])]
    output: Option<PathBuf>,
    /// WAV sample format.
    #[arg(long, value_enum, default_value_t = Format::Float32)]
    format: Format,
}

/// Kinds of generated signal.
#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
enum KindArg {
    Tones,
    Noise,
    Fsk,
    Linear,
    C4fm,
    Ofdm,
}

impl From<KindArg> for SignalKind {
    fn from(k: KindArg) -> Self {
        match k {
            KindArg::Tones => Self::Tones,
            KindArg::Noise => Self::Noise,
            KindArg::Fsk => Self::Fsk,
            KindArg::Linear => Self::Linear,
            KindArg::C4fm => Self::C4fm,
            KindArg::Ofdm => Self::Ofdm,
        }
    }
}

#[derive(Debug, Args)]
struct FskDemodArgs {
    /// Recording to demodulate.
    file: PathBuf,
    /// Edits applied first, as JSON (usually the phase detector).
    #[arg(long = "edit")]
    edits: Vec<String>,
    /// Keying rate in baud.
    #[arg(long)]
    baud: f64,
    /// Level centres as a comma list, e.g. `1200,1400,1600`.
    #[arg(long, conflicts_with_all = ["lo", "hi", "count"])]
    levels: Option<String>,
    /// Lowest level of an even grid (with --hi and --count).
    #[arg(long, requires_all = ["hi", "count"])]
    lo: Option<f64>,
    /// Highest level of an even grid.
    #[arg(long, requires_all = ["lo", "count"])]
    hi: Option<f64>,
    /// Number of levels of an even grid (2 to 256).
    #[arg(long, requires_all = ["lo", "hi"])]
    count: Option<usize>,
    /// RRC filter roll-off applied before sampling (APCO-25: 0.2).
    #[arg(long, conflicts_with = "average")]
    rrc: Option<f64>,
    /// Moving average over this fraction of a symbol before sampling (rectangular FSK).
    #[arg(long)]
    average: Option<f64>,
    /// Manual sync: position of the first sampling instant in samples. Automatic
    /// sync when omitted.
    #[arg(long)]
    phase: Option<f64>,
    /// Let the decision levels follow the data.
    #[arg(long)]
    adaptive: bool,
    /// Output form.
    #[arg(long, value_enum, default_value_t = SymbolFormat::Chars)]
    format: SymbolFormat,
    /// Symbol-to-bits map for `--format dibits`.
    #[arg(long, value_enum, default_value_t = MapKind::Natural)]
    map: MapKind,
    /// Symbols per output line (1 to 65536).
    #[arg(
        long,
        default_value_t = 64,
        value_parser = clap::builder::RangedU64ValueParser::<usize>::new().range(1..=65536)
    )]
    width: usize,
    /// Histogram smoothing (value units) when the levels are found automatically.
    #[arg(long, default_value_t = 4.0)]
    smooth: f64,
}

/// The part of the (edited) recording a command analyses, in seconds.
#[derive(Debug, Clone, Copy, Default, Args)]
struct SpanArgs {
    /// Start of the analysed span in seconds (default: start of the file).
    #[arg(long)]
    start: Option<f64>,
    /// End of the analysed span in seconds (default: end of the file).
    #[arg(long)]
    end: Option<f64>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
enum SymbolFormat {
    /// One character per symbol (0-9, A-V), up to 32 levels.
    Chars,
    /// Two hex digits per symbol, separated by "x".
    Hex,
    /// Bit groups per symbol (powers of two only), through `--map`.
    Dibits,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
enum MapKind {
    /// Level k carries the binary value k.
    Natural,
    /// Gray code.
    Gray,
    /// APCO-25 C4FM on frequency levels; inversion detected from long runs.
    C4fm,
    /// APCO-25 C4FM, upright.
    C4fmNormal,
    /// APCO-25 C4FM, spectrum inverted.
    C4fmInverted,
}

#[derive(Debug, Subcommand)]
enum SynthKind {
    /// A sine tone.
    Tone {
        /// Tone frequency in Hz.
        #[arg(long, default_value_t = 1000.0)]
        freq: f64,
        /// Duration in seconds.
        #[arg(long, default_value_t = 1.0)]
        duration: f64,
        #[command(flatten)]
        common: Common,
    },
    /// Continuous-phase M-ary FSK (order 2 = FSK-2).
    Fsk {
        /// Number of tones.
        #[arg(long, default_value_t = 2)]
        order: usize,
        /// Keying rate in baud.
        #[arg(long, default_value_t = 100.0)]
        baud: f64,
        /// Tone spacing (shift) in Hz.
        #[arg(long, default_value_t = 200.0)]
        spacing: f64,
        /// Centre of the tone grid in Hz.
        #[arg(long, default_value_t = 1500.0)]
        center: f64,
        /// Number of symbols.
        #[arg(long, default_value_t = 1000)]
        symbols: usize,
        #[command(flatten)]
        common: Common,
    },
    /// M-ary PSK on a carrier with rectangular pulses.
    Psk {
        /// Modulation order (2, 4, 8, ... 32).
        #[arg(long, default_value_t = 4)]
        order: usize,
        /// Keying rate in baud.
        #[arg(long, default_value_t = 2400.0)]
        baud: f64,
        /// Carrier frequency in Hz.
        #[arg(long, default_value_t = 1800.0)]
        carrier: f64,
        /// Number of symbols.
        #[arg(long, default_value_t = 4800)]
        symbols: usize,
        #[command(flatten)]
        common: Common,
    },
}

#[derive(Debug, Args)]
struct Common {
    /// Output WAV file.
    #[arg(short, long)]
    output: PathBuf,
    /// Sample rate in Hz.
    #[arg(long, default_value_t = 48_000.0)]
    rate: f64,
    /// Peak amplitude (full scale is 1.0).
    #[arg(long, default_value_t = 0.5)]
    amplitude: f64,
    /// Add white Gaussian noise at this SNR (dB, over the whole Nyquist band). The
    /// in-band Es/N0 then depends on the sample rate, and a real signal gets 3 dB less
    /// Es/N0 than a complex one; use --esn0-db to set Es/N0 directly.
    #[arg(long, conflicts_with = "esn0_db")]
    snr_db: Option<f64>,
    /// Add white Gaussian noise at this Es/N0 (dB, energy per symbol over noise
    /// density), independent of the sample rate. FSK and PSK only.
    #[arg(long)]
    esn0_db: Option<f64>,
    /// Seed for symbols and noise.
    #[arg(long, default_value_t = 1)]
    seed: u64,
    /// Output sample format.
    #[arg(long, value_enum, default_value_t = Format::Pcm16)]
    format: Format,
}

#[derive(Debug, Clone, Copy, ValueEnum)]
enum Format {
    Pcm16,
    Float32,
}

impl From<Format> for WavFormat {
    fn from(f: Format) -> Self {
        match f {
            Format::Pcm16 => WavFormat::Pcm16,
            Format::Float32 => WavFormat::Float32,
        }
    }
}

fn main() -> Result<()> {
    match Cli::parse().command {
        Command::Synth { kind } => synth(kind),
        Command::Generate(args) => generate(&args, &mut std::io::stdout().lock()),
        Command::Info { file } => info(&file),
        Command::Process {
            input,
            output,
            edits,
        } => process(&input, &output, &edits),
        Command::Peaks { file, fft, count } => peaks(&file, fft, count).map(drop),
        Command::FskDemod(args) => fsk_demod(&args).map(drop),
        Command::Measure {
            file,
            lo,
            hi,
            edits,
            span,
        } => measure(&file, lo, hi, &edits, &span).map(drop),
        Command::Adp { file, edits, span } => adp(&file, &edits, &span),
        Command::Phase {
            file,
            carrier,
            baud,
            order,
            diff,
            edits,
            span,
        } => phase(
            &file,
            &PhaseArgs {
                carrier,
                baud,
                order,
                diff,
            },
            &edits,
            &span,
        ),
        Command::Ofdm {
            file,
            min_br,
            max_br,
            symbols,
            start,
            check,
            edits,
        } => ofdm(
            &file,
            &OfdmArgs {
                min_br,
                max_br,
                symbols,
                start,
                check,
            },
            &edits,
        )
        .map(drop),
        Command::Tones {
            file,
            edits,
            smooth,
        } => tones(&file, &edits, smooth).map(drop),
        Command::Script {
            file,
            session,
            max_operations,
        } => script(&file, session.as_deref(), max_operations),
        Command::SessionReport { session, html, md } => {
            session_report(&session, html.as_deref(), md.as_deref())
        }
        Command::Replay { session } => replay(&session),
        Command::Bench {
            file,
            width,
            fft,
            raw_format,
            raw_channels,
            raw_rate,
        } => bench(
            &file,
            width,
            fft,
            raw_format.as_deref(),
            raw_channels,
            raw_rate,
        ),
    }
}

fn synth(kind: SynthKind) -> Result<()> {
    // The symbol rate is needed for --esn0-db; a plain tone has none.
    let (signal, common, summary, baud) = match kind {
        SynthKind::Tone {
            freq,
            duration,
            common,
        } => {
            let s = tone(freq, common.amplitude, common.rate, duration)?;
            (s, common, format!("tone {freq} Hz, {duration} s"), None)
        }
        SynthKind::Fsk {
            order,
            baud,
            spacing,
            center,
            symbols,
            common,
        } => {
            let p = MfskParams {
                order,
                baud,
                spacing,
                center,
                sample_rate: common.rate,
                amplitude: common.amplitude,
            };
            let s = mfsk(&p, &random_symbols(symbols, order, common.seed))?;
            (
                s,
                common,
                format!("FSK-{order}, {baud} Bd, spacing {spacing} Hz, centre {center} Hz"),
                Some(baud),
            )
        }
        SynthKind::Psk {
            order,
            baud,
            carrier,
            symbols,
            common,
        } => {
            let p = PskParams {
                order,
                baud,
                carrier,
                sample_rate: common.rate,
                amplitude: common.amplitude,
            };
            let s = psk(&p, &random_symbols(symbols, order, common.seed))?;
            (
                s,
                common,
                format!("PSK-{order}, {baud} Bd, carrier {carrier} Hz"),
                Some(baud),
            )
        }
    };
    let noise_seed = common.seed.wrapping_add(1);
    let (signal, noise): (Signal, String) = match (common.snr_db, common.esn0_db) {
        (Some(snr), _) => (
            add_awgn_snr(&signal, snr, noise_seed)?,
            format!(", SNR {snr} dB"),
        ),
        (None, Some(esn0)) => {
            let Some(rate) = baud else {
                anyhow::bail!(
                    "--esn0-db needs a symbol rate, which a tone does not have; use --snr-db"
                );
            };
            (
                add_awgn_esn0(&signal, esn0, rate, noise_seed)?,
                format!(", Es/N0 {esn0} dB"),
            )
        }
        (None, None) => (signal, String::new()),
    };
    write_wav(&common.output, &signal, common.format.into())
        .with_context(|| format!("writing {}", common.output.display()))?;
    println!(
        "Wrote {}: {summary}, {} Hz, {:.3} s{noise}",
        common.output.display(),
        common.rate,
        signal.duration(),
    );
    Ok(())
}

fn info(file: &Path) -> Result<()> {
    let i = probe(file).with_context(|| format!("reading {}", file.display()))?;
    println!("File:        {}", i.path.display());
    println!("Container:   {:?}", i.container);
    println!("Codec:       {}", i.codec);
    if let Some(f) = i.sample_format {
        println!("Samples:     {}", f.label());
    }
    println!("Channels:    {}", i.channels);
    println!("Sample rate: {} Hz", i.sample_rate);
    if i.center_frequency != 0.0 {
        println!("Centre freq: {} Hz", i.center_frequency);
    }
    match (i.frames, i.duration()) {
        (Some(f), Some(d)) => println!("Frames:      {f} ({d:.3} s)"),
        _ => println!("Frames:      unknown until decoded"),
    }
    println!(
        "Access:      {}",
        if i.streamable {
            "memory-mapped, read on demand"
        } else {
            "decoded into memory"
        }
    );
    for note in &i.notes {
        println!("Note:        {note}");
    }
    Ok(())
}

fn process(input: &Path, output: &Path, edits: &[String]) -> Result<()> {
    let signal = apply_edits(open_signal(input)?, edits, &mut |i, edit, signal, took| {
        eprintln!(
            "{:>2}. {:<40} {:>8.3} s  -> {} samples at {} Hz",
            i + 1,
            edit.label(),
            took.as_secs_f64(),
            signal.len(),
            signal.sample_rate()
        );
    })?;
    let written = write_output(output, &signal, "sn-cli process", WavFormat::Float32)?;
    println!("Wrote {}", written.display());
    Ok(())
}

/// True when `path` names SigMF output: its extension starts with "sigmf".
fn is_sigmf(path: &Path) -> bool {
    path.extension()
        .and_then(|e| e.to_str())
        .is_some_and(|e| e.starts_with("sigmf"))
}

/// The error for a sample rate WAV cannot store, naming the SigMF way out.
fn wav_rate_error(output: &Path, rate: f64) -> anyhow::Error {
    anyhow::anyhow!(
        "the sample rate {rate} Hz cannot be stored in WAV, which needs a whole number of Hz; \
         write {} instead",
        output.with_extension("sigmf-meta").display()
    )
}

/// The warning to print when writing `signal` to `output` loses its centre frequency:
/// WAV has nowhere to keep it, SigMF does.
fn lost_center_frequency_warning(output: &Path, signal: &Signal) -> Option<String> {
    let fc = signal.center_frequency();
    (!is_sigmf(output) && fc != 0.0).then(|| {
        format!(
            "warning: WAV does not store the centre frequency ({fc} Hz); write {} to keep it",
            output.with_extension("sigmf-meta").display()
        )
    })
}

/// Write `signal` to `output`: SigMF when the extension starts with "sigmf" (the
/// metadata carries `description`), otherwise WAV in `format`. Returns the file
/// written (the .sigmf-meta for SigMF).
fn write_output(
    output: &Path,
    signal: &Signal,
    description: &str,
    format: WavFormat,
) -> Result<PathBuf> {
    if is_sigmf(output) {
        return sn_io::write_sigmf(output, signal, description)
            .with_context(|| format!("writing {}", output.display()));
    }
    if let Some(warning) = lost_center_frequency_warning(output, signal) {
        eprintln!("{warning}");
    }
    match write_wav(output, signal, format) {
        Ok(()) => Ok(output.to_path_buf()),
        Err(sn_io::Error::FractionalSampleRate(rate)) => Err(wav_rate_error(output, rate)),
        Err(e @ sn_io::Error::Unsupported(_))
            if !sn_io::wav_can_store_rate(signal.sample_rate()) =>
        {
            Err(anyhow::Error::new(e).context(format!(
                "writing {}; write {} instead",
                output.display(),
                output.with_extension("sigmf-meta").display()
            )))
        }
        Err(e) => Err(anyhow::Error::new(e).context(format!("writing {}", output.display()))),
    }
}

/// The spec `args` asks for: a file, a preset, a kind's default, or the default
/// linear (PSK/QAM) signal; with the seed replaced when `--seed` is given.
fn generator_spec(args: &GenerateArgs) -> Result<GeneratorSpec> {
    let mut spec = if let Some(path) = &args.spec {
        let text =
            std::fs::read_to_string(path).with_context(|| format!("reading {}", path.display()))?;
        serde_json::from_str(&text).with_context(|| {
            format!(
                "{} is not a valid generator spec; `sn-cli generate --print-spec` prints \
                 one to start from",
                path.display()
            )
        })?
    } else if let Some(name) = &args.preset {
        GeneratorSpec::preset(name).with_context(|| {
            format!(
                "no preset named '{name}'; the presets are: {}",
                PRESET_NAMES.join(", ")
            )
        })?
    } else {
        GeneratorSpec::default_for(args.kind.map_or(SignalKind::Linear, SignalKind::from))
    };
    if let Some(seed) = args.seed {
        spec.common_mut().seed = seed;
    }
    Ok(spec)
}

/// `sn-cli generate`: list presets, print a spec, or generate and write it.
fn generate(args: &GenerateArgs, out: &mut dyn std::io::Write) -> Result<()> {
    if args.list_presets {
        for name in PRESET_NAMES {
            writeln!(out, "{name}")?;
        }
        return Ok(());
    }
    let spec = generator_spec(args)?;
    if args.print_spec {
        writeln!(out, "{}", serde_json::to_string_pretty(&spec)?)?;
        return Ok(());
    }
    let Some(output) = &args.output else {
        anyhow::bail!("give the output file with -o");
    };
    let rate = spec.common().sample_rate;
    if !is_sigmf(output) && !sn_io::wav_can_store_rate(rate) {
        return Err(wav_rate_error(output, rate));
    }
    let made = sn_synth::generate(&spec)?;
    let description = if spec.common().label.is_empty() {
        made.description.clone()
    } else {
        format!("{}: {}", spec.common().label, made.description)
    };
    let written = write_output(output, &made.signal, &description, args.format.into())?;
    writeln!(
        out,
        "Wrote {}: {}, {} Hz, {} samples ({:.3} s)",
        written.display(),
        made.description,
        made.signal.sample_rate(),
        made.signal.len(),
        made.signal.duration()
    )?;
    Ok(())
}

/// Open a recording with the default load options.
fn open_signal(file: &Path) -> Result<Signal> {
    sn_io::open(file, &LoadOptions::default())
        .with_context(|| format!("opening {}", file.display()))
}

/// Apply a chain of JSON edits in order. `report` is called after each edit with its
/// position in the chain, the edit, its result and the time it took.
fn apply_edits(
    mut signal: Signal,
    edits: &[String],
    report: &mut dyn FnMut(usize, &sn_dsp::ops::Edit, &Signal, std::time::Duration),
) -> Result<Signal> {
    for (i, text) in edits.iter().enumerate() {
        let edit: sn_dsp::ops::Edit = serde_json::from_str(text)
            .with_context(|| format!("edit {} is not a valid operation: {text}", i + 1))?;
        let t = std::time::Instant::now();
        signal = edit
            .run(&signal, &|| false)
            .with_context(|| format!("edit {} ({}) failed", i + 1, edit.label()))?;
        report(i, &edit, &signal, t.elapsed());
    }
    Ok(signal)
}

/// Open a file and apply a chain of JSON edits, reporting each one on stderr.
fn open_with_edits(file: &Path, edits: &[String]) -> Result<Signal> {
    apply_edits(open_signal(file)?, edits, &mut |_, edit, signal, _| {
        eprintln!("applied: {} ({} Hz)", edit.label(), signal.sample_rate());
    })
}

/// Sample range `start..end` selected by `--start`/`--end` (seconds) on `signal`.
fn span_of(signal: &Signal, span: &SpanArgs) -> Result<(usize, usize)> {
    let fs = signal.sample_rate();
    let len = signal.len();
    let to_index = |t: f64, flag: &str| -> Result<usize> {
        if !t.is_finite() || t < 0.0 {
            anyhow::bail!("--{flag} must be a time in seconds, 0 or more (got {t})");
        }
        Ok(((t * fs).round() as usize).min(len))
    };
    let start = span.start.map(|t| to_index(t, "start")).transpose()?;
    let end = span.end.map(|t| to_index(t, "end")).transpose()?;
    let (start, end) = (start.unwrap_or(0), end.unwrap_or(len));
    if end <= start {
        anyhow::bail!(
            "the span from --start {:.6} s to --end {:.6} s is empty; the recording is {:.6} s long",
            start as f64 / fs,
            end as f64 / fs,
            len as f64 / fs
        );
    }
    Ok((start, end))
}

fn measure(
    file: &Path,
    lo: f64,
    hi: f64,
    edits: &[String],
    span: &SpanArgs,
) -> Result<sn_analysis::line::LineEstimate> {
    use sn_analysis::line::{MAX_SAMPLES, measure_line};
    let signal = open_with_edits(file, edits)?;
    let (start, end) = span_of(&signal, span)?;
    let n = end - start;
    if n < 64 {
        anyhow::bail!(
            "the span holds {n} samples; a measurement needs at least 64 (widen it with --start/--end)"
        );
    }
    if n > MAX_SAMPLES {
        anyhow::bail!(
            "the span holds {n} samples; a measurement takes at most {MAX_SAMPLES} (narrow it with --start/--end)"
        );
    }
    if lo.is_nan() || hi.is_nan() || hi <= lo {
        anyhow::bail!("--hi ({hi} Hz) must be above --lo ({lo} Hz)");
    }
    let e = measure_line(&signal, start, end, lo, hi, &|| false)?;
    println!(
        "line: {:.4} Hz  level {:.1} dBFS  SNR {:.1} dB  (pure-tone bound +/- {:.1e} Hz)",
        e.frequency, e.level_db, e.snr_db, e.uncertainty
    );
    Ok(e)
}

/// Histogram level suggestion over the signal without its first and last 5%
/// (filter edges). Shared by `tones` and `fsk-demod` so both pick the same levels.
fn suggest_levels_trimmed(signal: &Signal, smooth: f64) -> Result<sn_demod::mfsk::LevelSuggestion> {
    let edge = signal.len() / 20;
    Ok(sn_demod::mfsk::suggest_levels(
        signal,
        edge,
        signal.len().saturating_sub(edge),
        smooth,
    )?)
}

fn tones(file: &Path, edits: &[String], smooth: f64) -> Result<sn_demod::mfsk::LevelSuggestion> {
    let signal = open_with_edits(file, edits)?;
    let s = suggest_levels_trimmed(&signal, smooth)
        .context("no tone grid found in the histogram; try another --smooth or check the edits")?;
    for t in &s.tones {
        println!("tone {t:>10.2}");
    }
    if let sn_demod::mfsk::Levels::Grid { lo, hi, count } = &s.grid {
        let spacing = (hi - lo) / (*count as f64 - 1.0).max(1.0);
        println!(
            "grid: {count} positions, spacing {spacing:.2}, {lo:.2} to {hi:.2}, {} tones found",
            s.tones.len()
        );
    } else {
        println!("grid: {:?}", s.grid);
    }
    Ok(s)
}

/// PhasePlane settings of the `phase` command.
#[derive(Debug, Clone, Copy)]
struct PhaseArgs {
    carrier: f64,
    baud: f64,
    order: u32,
    diff: u8,
}

fn phase(file: &Path, a: &PhaseArgs, edits: &[String], span: &SpanArgs) -> Result<()> {
    use sn_demod::phase::{PhaseConfig, offset_timing, rings, run};
    let PhaseArgs {
        carrier,
        baud,
        order,
        diff,
    } = *a;
    let signal = open_with_edits(file, edits)?;
    let (start, end) = span_of(&signal, span)?;
    let cfg = PhaseConfig {
        carrier,
        baud,
        order,
        diff,
        ..PhaseConfig::default()
    };
    let r = run(&signal, start, end, &cfg, &|| false)?;
    // Angle clusters: peaks of the angle histogram (circular, so analyse 0..360 twice).
    let skip = r.angles.len() / 10;
    let doubled: Vec<f32> = r
        .angles
        .iter()
        .skip(skip)
        .flat_map(|a| [*a, *a + 360.0])
        .collect();
    let hs = Signal::real(doubled, 1.0)?;
    let h = sn_analysis::histogram::Histogram::build(&hs, 0, hs.len(), 1440, Some((0.0, 720.0)))?;
    let mut peaks: Vec<f64> = h
        .peaks(0.2, 2.0)
        .into_iter()
        .filter(|a| (180.0..540.0).contains(a))
        .map(|a| a.rem_euclid(360.0))
        .collect();
    peaks.sort_by(f64::total_cmp);
    let spread = |a: &f32| {
        let step = 360.0 / order.max(1) as f32;
        let e = a - (a / step).round() * step;
        e * e
    };
    let rms = (r.angles.iter().skip(skip).map(spread).sum::<f32>()
        / (r.angles.len() - skip).max(1) as f32)
        .sqrt();
    println!("symbols:        {}", r.points.len());
    println!(
        "carrier offset: {:+.3} Hz (tracked by the loop)",
        r.carrier_offset
    );
    println!(
        "angle clusters: {} at {:?} deg",
        peaks.len(),
        peaks
            .iter()
            .map(|a| (a * 10.0).round() / 10.0)
            .collect::<Vec<_>>()
    );
    println!("angle error:    {rms:.2} deg RMS from the nearest {order}-PSK angle");
    println!("rings:          {:?}", rings(&r.points));
    println!(
        "I/Q offset:     {:.2} symbol (about 0.5 means offset PSK)",
        offset_timing(&r.trajectory)
    );
    let codes: String = r
        .codes
        .iter()
        .take(64)
        .map(|c| char::from_digit(*c % 36, 36).unwrap_or('?'))
        .collect();
    let more = if r.codes.len() > 64 { "..." } else { "" };
    println!("codes:          {codes}{more}");
    Ok(())
}

fn parse_levels(text: &str) -> Result<Vec<f64>> {
    text.split(',')
        .map(|t| {
            t.trim().parse::<f64>().with_context(|| {
                format!("'{t}' in --levels is not a number; give the levels as e.g. 1200,1400,1600")
            })
        })
        .collect()
}

/// Demodulator settings from the command line; finds the levels from the histogram
/// when none are given.
fn fsk_config(a: &FskDemodArgs, signal: &Signal) -> Result<sn_demod::mfsk::MfskConfig> {
    use sn_demod::mfsk::{Levels, MfskConfig, Prefilter, SyncMode};
    let levels = match (&a.levels, a.lo, a.hi, a.count) {
        (Some(text), ..) => Levels::Explicit(parse_levels(text)?),
        (None, Some(lo), Some(hi), Some(count)) => Levels::Grid { lo, hi, count },
        _ => {
            let s = suggest_levels_trimmed(signal, a.smooth)
                .context("could not find the levels from the histogram; give them with --levels or --lo/--hi/--count")?;
            let tones: Vec<String> = s.tones.iter().map(|t| format!("{t:.1}")).collect();
            eprintln!("levels from histogram peaks: {}", tones.join(", "));
            s.grid
        }
    };
    let mut cfg = MfskConfig::new(levels, a.baud);
    cfg.filter = match (a.rrc, a.average) {
        (Some(alpha), _) => Prefilter::Rrc { alpha },
        (None, Some(fraction)) => Prefilter::Average { fraction },
        (None, None) => Prefilter::None,
    };
    cfg.adaptive = a.adaptive;
    if let Some(phase) = a.phase {
        cfg.sync = SyncMode::Manual { phase };
    }
    Ok(cfg)
}

fn fsk_demod(a: &FskDemodArgs) -> Result<sn_demod::mfsk::Demodulated> {
    use sn_demod::mfsk::{
        BitMap, Polarity, bits_to_string, demodulate, detect_c4fm_inversion, symbols_to_bits,
        symbols_to_chars, symbols_to_hex,
    };
    let signal = open_with_edits(&a.file, a.edits.as_slice())?;
    if signal.unit() != sn_core::SampleUnit::Hertz {
        eprintln!(
            "note: the signal is not a phase-detector output; the levels are compared with its raw values (add --edit '{{\"op\":\"phase_detector\"}}' for an IF recording)"
        );
    }
    let cfg = fsk_config(a, &signal)?;
    let d = demodulate(&signal, &cfg, &|| false)?;
    let centres: Vec<String> = d
        .layout
        .centres()
        .iter()
        .map(|c| format!("{c:.1}"))
        .collect();
    println!("levels:       {}", centres.join(", "));
    println!("symbols:      {}", d.symbols.len());
    println!(
        "timing:       phase {:.2} samples, clock error {:+.1} ppm (measured {:.4} Bd)",
        d.timing_phase,
        d.clock_error * 1e6,
        d.measured_baud
    );
    println!(
        "quality:      {:.3} (1 = every value on its level)  eye opening {:.3}",
        d.quality, d.eye_opening
    );
    let counts: Vec<String> = d.level_counts.iter().map(ToString::to_string).collect();
    println!("level counts: {}", counts.join(" "));
    if a.adaptive {
        let f: Vec<String> = d.final_levels.iter().map(|c| format!("{c:.1}")).collect();
        println!("final levels: {}", f.join(", "));
    }
    let width = a.width.max(1);
    let lines: Vec<String> = match a.format {
        SymbolFormat::Chars => symbols_to_chars(&d.symbols)?
            .as_bytes()
            .chunks(width)
            .map(|c| String::from_utf8_lossy(c).into_owned())
            .collect(),
        SymbolFormat::Hex => d.symbols.chunks(width).map(symbols_to_hex).collect(),
        SymbolFormat::Dibits => {
            let order = d.layout.order();
            let c4fm = matches!(
                a.map,
                MapKind::C4fm | MapKind::C4fmNormal | MapKind::C4fmInverted
            );
            if c4fm && order != 4 {
                anyhow::bail!(
                    "the C4FM bit map needs 4 levels but the demodulator has {order}; use --map natural or gray, or set 4 levels"
                );
            }
            let map = match a.map {
                MapKind::Natural => BitMap::natural(order)?,
                MapKind::Gray => BitMap::gray(order)?,
                MapKind::C4fmNormal => BitMap::c4fm(),
                MapKind::C4fmInverted => BitMap::c4fm_inverted(),
                MapKind::C4fm => {
                    let r = detect_c4fm_inversion(&d.symbols);
                    eprintln!(
                        "polarity:     {:?} (dibit-00 runs: {} upright, {} inverted symbols)",
                        r.polarity, r.normal_evidence, r.inverted_evidence
                    );
                    if r.polarity == Polarity::Inverted {
                        BitMap::c4fm_inverted()
                    } else {
                        BitMap::c4fm()
                    }
                }
            };
            if map.order() != order {
                anyhow::bail!(
                    "the bit map has {} levels but the demodulator has {order}; use --map natural or gray, or set 4 levels for C4FM",
                    map.order()
                );
            }
            let group = map.bits_per_symbol() as usize;
            let bits = symbols_to_bits(&d.symbols, &map)?;
            bits.chunks(width.saturating_mul(group).max(1))
                .map(|c| bits_to_string(c, group))
                .collect()
        }
    };
    for line in lines {
        println!("{line}");
    }
    Ok(d)
}

fn adp(file: &Path, edits: &[String], span: &SpanArgs) -> Result<()> {
    let signal = open_with_edits(file, edits)?;
    let (start, end) = span_of(&signal, span)?;
    for (i, c) in sn_analysis::adp::estimate_baud(&signal, start, end, &|| false)?
        .iter()
        .enumerate()
    {
        println!(
            "{}. {:>10.3} Bd  ({:?} detector, score {:.1}){}",
            i + 1,
            c.baud,
            c.detector,
            c.score,
            if c.refined { "" } else { " (coarse)" }
        );
    }
    Ok(())
}

/// Settings of the `ofdm` command.
#[derive(Debug, Clone, Copy)]
struct OfdmArgs {
    min_br: f64,
    max_br: f64,
    symbols: usize,
    start: usize,
    check: usize,
}

fn ofdm(file: &Path, a: &OfdmArgs, edits: &[String]) -> Result<sn_ofdm::CtResult> {
    use sn_ofdm::{
        Baseband, BrParams, CtParams, Grid, KParams, OrthoParams, check_orthogonality, find_ct,
        get_br, list_of_k, occupancy,
    };
    let signal = open_with_edits(file, edits)?;
    let bb = Baseband::from_signal(&signal, 0, signal.len(), &|| false)?;
    let fs = bb.sample_rate();
    let t = std::time::Instant::now();
    let ct = find_ct(
        &bb,
        &CtParams {
            min_br: a.min_br,
            max_br: a.max_br,
            symbols: a.symbols,
            start: a.start,
        },
        &|| false,
    )?;
    println!(
        "Find CT ({:.2} s): LS {}  LU {}  LG {}  K {:.4}  start {}  coherence {:.3}  margin {:.2}  \
         confidence {:.2}",
        t.elapsed().as_secs_f64(),
        ct.ls,
        ct.lu,
        ct.lg,
        ct.k(),
        ct.start,
        ct.triangle.coherence,
        ct.margin,
        ct.confidence
    );
    for c in ct.candidates.iter().skip(1).take(4) {
        println!(
            "   also: LS {}  LU {}  LG {}  coherence {:.3}  score {:.1}",
            c.ls, c.lu, c.lg, c.coherence, c.score
        );
    }
    let br = get_br(&bb, &ct, &BrParams::default(), &|| false)?;
    println!(
        "Get Br: {:.6} Bd  LS {:.5}  LU {:.4}  LG {:.4}  K {:.5}  spacing {:.4} Hz  \
         offset {:+.3} Hz  ({} symbols, coherence {:.3})",
        br.br, br.ls, br.lu, br.lg, br.k, br.spacing, br.freq_offset, br.symbols, br.coherence
    );
    let grid = Grid::from_br(&br);
    match occupancy(&bb, &grid, 0, 200) {
        Ok(occ) => match (occ.occupied.first(), occ.occupied.last()) {
            (Some(first), Some(last)) => println!(
                "Channels: {} occupied over {} grid positions, {:.1} to {:.1} Hz (LU {} at {} Hz)",
                occ.count(),
                occ.span(),
                grid.bin_frequency(*first),
                grid.bin_frequency(*last),
                grid.lu,
                fs
            ),
            _ => println!("Channels: none above the noise floor"),
        },
        Err(e) => println!("Channels: not measured: {e}"),
    }
    let list = list_of_k(&KParams::from_br(&br))?;
    let ranked = if a.check > 0 {
        check_orthogonality(
            &bb,
            &br,
            &list,
            &OrthoParams {
                top: a.check,
                ..OrthoParams::default()
            },
            &|| false,
        )?
    } else {
        list
    };
    println!("List of K (most plausible first):");
    for (i, c) in ranked.iter().take(10).enumerate() {
        let ici = c
            .ici_db
            .map_or(String::new(), |v| format!("  ICI {v:.1} dB"));
        println!(
            "{:>3}. K {}/{} = {:.5}  LS {}  LU {}{}  LG {}  native rate {:.3} Hz  \
             LU error {:.4}{}",
            i + 1,
            c.k_num,
            c.k_den,
            c.k,
            c.ls,
            c.lu,
            if c.even_lu { "" } else { " (odd)" },
            c.lg,
            c.sample_rate,
            c.lu_error,
            ici
        );
    }
    Ok(ct)
}

/// Most frames averaged by `peaks`.
const PEAK_FRAMES: usize = 2000;

/// Start of each spectrum frame: half-overlapped frames when they fit within
/// `max_frames`, otherwise `max_frames` frames spread evenly over the whole file. The
/// last full frame (at `len - fft`) is always included.
fn frame_starts(len: usize, fft: usize, max_frames: usize) -> Vec<usize> {
    let last = len.saturating_sub(fft);
    let hop = (fft / 2).max(1);
    let n = (last / hop + 1).min(max_frames.max(1));
    if n <= 1 {
        return vec![0];
    }
    (0..n)
        .map(|k| (k as u128 * last as u128 / (n as u128 - 1)) as usize)
        .collect()
}

/// Spectral peak: frequency in Hz and level in dBFS.
#[derive(Debug, Clone, Copy, PartialEq)]
struct Peak {
    frequency: f64,
    level_db: f32,
}

fn peaks(file: &Path, fft: usize, count: usize) -> Result<Vec<Peak>> {
    let signal = open_signal(file)?;
    let (floor, found) = spectrum_peaks(&signal, fft, count)?;
    println!("Noise floor (median bin): {floor:.1} dBFS");
    for p in &found {
        println!(
            "{:>12.2} Hz  {:>7.1} dBFS  ({:+.1} dB above floor)",
            p.frequency,
            p.level_db,
            p.level_db - floor
        );
    }
    Ok(found)
}

/// The `count` strongest local maxima of the mean spectrum, strongest first, and the
/// noise floor (median bin) in dBFS.
fn spectrum_peaks(signal: &Signal, fft: usize, count: usize) -> Result<(f32, Vec<Peak>)> {
    use sn_dsp::spectrum::{Combine, SpectrumEngine};
    use sn_dsp::window::WindowKind;
    let engine = SpectrumEngine::new(fft, WindowKind::default(), signal.kind())?;
    let frames = frame_starts(signal.len(), fft, PEAK_FRAMES);
    let db = engine.column_db(signal, &frames, Combine::Mean, &|| false)?;
    let mut idx: Vec<usize> = (0..db.len()).collect();
    let level = |i: usize| db.get(i).copied().unwrap_or(f32::MIN);
    // Local maxima only, strongest first.
    idx.retain(|&i| i > 0 && level(i) >= level(i - 1) && level(i) >= level(i + 1));
    idx.sort_by(|a, b| level(*b).total_cmp(&level(*a)));
    let fs = signal.sample_rate();
    let mut sorted = db.clone();
    sorted.sort_by(f32::total_cmp);
    let floor = sorted.get(sorted.len() / 2).copied().unwrap_or(f32::MIN);
    let found = idx
        .into_iter()
        .take(count)
        .map(|i| Peak {
            frequency: engine.bin_frequency(i, fs),
            level_db: level(i),
        })
        .collect();
    Ok((floor, found))
}

fn script(file: &Path, session: Option<&Path>, max_operations: u64) -> Result<()> {
    let mut engine = sn_script::ScriptEngine::new(sn_script::ScriptOptions {
        max_operations,
        ..Default::default()
    });
    let run = engine.run_file(file);
    // Save what was done even when the script failed part-way, so it can be inspected.
    if let Some(out) = session {
        let s = engine.session();
        match sn_session::save(out, &s)
            .with_context(|| format!("saving the session to {}", out.display()))
        {
            Ok(()) => println!(
                "Saved session with {} signals to {}",
                s.nodes.len(),
                out.display()
            ),
            // The script's own error matters more; report the save failure beside it.
            Err(e) if run.is_err() => eprintln!("error: {e:#}"),
            Err(e) => return Err(e),
        }
    }
    run.with_context(|| format!("running {}", file.display()))
}

fn session_report(session: &Path, html: Option<&Path>, md: Option<&Path>) -> Result<()> {
    let s = sn_session::load(session)?;
    if let Some(path) = html {
        sn_session::report::save_report(path, &sn_session::report::html(&s))?;
        println!("Wrote {}", path.display());
    }
    if let Some(path) = md {
        sn_session::report::save_report(path, &sn_session::report::markdown(&s))?;
        println!("Wrote {}", path.display());
    }
    if html.is_none() && md.is_none() {
        print!("{}", sn_session::report::markdown(&s));
    }
    Ok(())
}

fn replay(session: &Path) -> Result<()> {
    let s = sn_session::load(session)?;
    let options = sn_session::ReplayOptions {
        base_dir: session.parent().map(Path::to_path_buf),
    };
    let t = std::time::Instant::now();
    let out = sn_session::replay(
        &s,
        &options,
        &mut |p| eprintln!("[{}/{}] {}", p.done + 1, p.total, p.label),
        &|| false,
    )?;
    for node in &s.nodes {
        match out.new_id(node.id).and_then(|id| out.graph.get(id)) {
            Some(n) => println!(
                "ok      {:<5} {:<40} {:>10} samples at {} Hz",
                node.id.to_string(),
                node.label,
                n.signal.len(),
                n.signal.sample_rate()
            ),
            None => println!("FAILED  {:<5} {}", node.id.to_string(), node.label),
        }
    }
    for issue in &out.issues {
        println!(
            "{} {} ({}): {}",
            if issue.failed { "error:  " } else { "warning:" },
            issue.node,
            issue.label,
            issue.message
        );
    }
    println!(
        "Rebuilt {} of {} signals in {:.3} s; {} measurements restored.",
        out.id_map.len(),
        s.nodes.len(),
        t.elapsed().as_secs_f64(),
        out.measurements.len()
    );
    if !out.is_complete() {
        anyhow::bail!(
            "{} signals could not be rebuilt; see the errors above",
            out.failed_count()
        );
    }
    Ok(())
}

fn parse_sample_format(s: &str) -> Result<SampleFormat> {
    Ok(match s {
        "u8" => SampleFormat::U8,
        "i8" => SampleFormat::I8,
        "i16" => SampleFormat::I16,
        "i24" => SampleFormat::I24,
        "i32" => SampleFormat::I32,
        "f32" => SampleFormat::F32,
        "f64" => SampleFormat::F64,
        other => {
            anyhow::bail!("unknown sample format '{other}' (use u8, i8, i16, i24, i32, f32, f64)")
        }
    })
}

fn bench(
    file: &Path,
    width: usize,
    fft: usize,
    raw_format: Option<&str>,
    raw_channels: u16,
    raw_rate: f64,
) -> Result<()> {
    use std::time::Instant;

    use sn_dsp::overview::Overview;
    use sn_dsp::spectrum::{ColumnRequest, Combine, SpectrumEngine, sonogram_columns};
    use sn_dsp::window::WindowKind;

    let raw = raw_format
        .map(|f| -> Result<RawFormat> {
            Ok(RawFormat {
                format: parse_sample_format(f)?,
                endian: Endian::Little,
                channels: raw_channels,
                sample_rate: raw_rate,
                data_offset: 0,
            })
        })
        .transpose()?;
    let t = Instant::now();
    let signal = sn_io::open(
        file,
        &LoadOptions {
            raw,
            ..LoadOptions::default()
        },
    )
    .with_context(|| format!("opening {}", file.display()))?;
    println!(
        "open:              {:>8.3} s  ({} samples, {:?}, {})",
        t.elapsed().as_secs_f64(),
        signal.len(),
        signal.kind(),
        if signal.is_lazy() {
            "memory-mapped"
        } else {
            "in memory"
        }
    );

    let t = Instant::now();
    let ov = Overview::build(&signal, Overview::DEFAULT_BLOCK, &|| false)?;
    println!(
        "overview build:    {:>8.3} s  (once per file, in the background)",
        t.elapsed().as_secs_f64()
    );

    let t = Instant::now();
    let w = width.max(1);
    let spp = signal.len() as f64 / w as f64;
    for x in 0..w {
        let a = (x as f64 * spp) as usize;
        // black_box keeps the optimiser from dropping the work being timed.
        std::hint::black_box(ov.span(&signal, a, (a as f64 + spp) as usize));
    }
    println!(
        "waveform redraw:   {:>8.3} s  (full file, {w} columns)",
        t.elapsed().as_secs_f64()
    );

    let engine = SpectrumEngine::new(fft, WindowKind::default(), signal.kind())?;
    let screen = |spp: f64, label: &str| -> Result<()> {
        let req = ColumnRequest {
            start: 0.0,
            samples_per_column: spp,
            hop: fft / 2,
            max_frames_per_column: 24,
            combine: Combine::PeakHold,
        };
        let t = Instant::now();
        let cols = std::hint::black_box(sonogram_columns(
            &engine,
            &signal,
            &req,
            0..w as i64,
            &|| false,
        )?);
        println!(
            "sonogram {label:<9} {:>8.3} s  ({} columns, {:.0} samples/column)",
            t.elapsed().as_secs_f64(),
            cols.len(),
            spp
        );
        Ok(())
    };
    screen(spp, "full:")?;
    screen(fft as f64 / 2.0, "zoomed:")?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use clap::CommandFactory;

    use super::*;

    #[test]
    fn process_applies_a_chain_and_rejects_bad_json() {
        let dir = tempfile::tempdir().unwrap();
        let input = dir.path().join("in.wav");
        let out = dir.path().join("out.wav");
        let s = tone(1000.0, 0.5, 8000.0, 1.0).unwrap();
        write_wav(&input, &s, WavFormat::Pcm16).unwrap();
        let chain = [
            r#"{"op":"band_select","lo":900,"hi":1100}"#.to_owned(),
            r#"{"op":"resample","rate":4000}"#.to_owned(),
        ];
        process(&input, &out, &chain).unwrap();
        let info = probe(&out).unwrap();
        assert_eq!(info.sample_rate, 4000.0);
        assert!(process(&input, &out, &["{not json".to_owned()]).is_err());
        assert!(process(&input, &out, &[r#"{"op":"no_such_op"}"#.to_owned()]).is_err());
    }

    #[test]
    fn process_writes_fractional_rates_to_sigmf_only() {
        let dir = tempfile::tempdir().unwrap();
        let input = dir.path().join("in.wav");
        let s = tone(1000.0, 0.5, 8000.0, 0.5).unwrap();
        write_wav(&input, &s, WavFormat::Pcm16).unwrap();
        let fractional = [r#"{"op":"resample","rate":4000.5}"#.to_owned()];
        // WAV cannot hold 4000.5 Hz: the error names the SigMF file to write instead.
        let wav = dir.path().join("out.wav");
        let err = format!("{:#}", process(&input, &wav, &fractional).unwrap_err());
        assert!(err.contains("out.sigmf-meta"), "{err}");
        assert!(!wav.exists());
        // SigMF keeps it.
        let meta = dir.path().join("out.sigmf-meta");
        process(&input, &meta, &fractional).unwrap();
        assert_eq!(probe(&meta).unwrap().sample_rate, 4000.5);
        // A rate within the WAV writer's tolerance of a whole number is rounded.
        let near = [r#"{"op":"resample","rate":4000.0000000001}"#.to_owned()];
        process(&input, &wav, &near).unwrap();
        assert_eq!(probe(&wav).unwrap().sample_rate, 4000.0);
    }

    #[test]
    fn script_session_replay_and_report_commands() {
        let dir = tempfile::tempdir().unwrap();
        let wav = dir.path().join("in.wav");
        write_wav(
            &wav,
            &tone(1000.0, 0.5, 8000.0, 1.0).unwrap(),
            WavFormat::Pcm16,
        )
        .unwrap();
        let script_file = dir.path().join("s.rhai");
        std::fs::write(
            &script_file,
            r#"let s = open("in.wav"); let b = edit(s, #{op: "lowpass", cutoff: 1500}); print(info(b).rate);"#,
        )
        .unwrap();
        let session = dir.path().join("session.json");
        script(&script_file, Some(&session), 1_000_000).unwrap();
        replay(&session).unwrap();
        let html = dir.path().join("r.html");
        let md = dir.path().join("r.md");
        session_report(&session, Some(&html), Some(&md)).unwrap();
        assert!(
            std::fs::read_to_string(&html)
                .unwrap()
                .contains("lowpass 1500 Hz")
        );
        assert!(
            std::fs::read_to_string(&md)
                .unwrap()
                .contains("## Derivation tree")
        );

        // A missing source makes replay fail with a clear message.
        std::fs::remove_file(&wav).unwrap();
        let e = replay(&session).unwrap_err();
        assert!(e.to_string().contains("could not be rebuilt"));
        // A failing script is an error, and the session is still saved.
        std::fs::write(&script_file, r#"open("in.wav");"#).unwrap();
        assert!(script(&script_file, Some(&session), 1_000_000).is_err());
        assert!(session.exists());
    }

    #[test]
    fn fsk_demod_decodes_a_synthetic_file() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("fsk.wav");
        let p = MfskParams {
            order: 4,
            baud: 100.0,
            spacing: 200.0,
            center: 1500.0,
            sample_rate: 8000.0,
            amplitude: 0.5,
        };
        let s = mfsk(&p, &random_symbols(300, 4, 3)).unwrap();
        write_wav(&path, &s, WavFormat::Float32).unwrap();
        let cli = Cli::try_parse_from([
            "sn-cli",
            "fsk-demod",
            path.to_str().unwrap(),
            "--edit",
            r#"{"op":"phase_detector"}"#,
            "--baud",
            "100",
            "--lo",
            "1200",
            "--hi",
            "1800",
            "--count",
            "4",
            "--rrc",
            "0.5",
            "--format",
            "dibits",
            "--map",
            "gray",
        ])
        .unwrap();
        let Command::FskDemod(args) = cli.command else {
            panic!("expected fsk-demod")
        };
        let d = fsk_demod(&args).unwrap();
        // Match each decision to the transmitted symbol by its sampling instant.
        let truth = random_symbols(300, 4, 3);
        let (mut n, mut errors) = (0, 0);
        for (s, t) in d.symbols.iter().zip(&d.instants) {
            let k = (t * p.baud / p.sample_rate).floor() as usize;
            if k < 5 || k + 5 >= truth.len() {
                continue;
            }
            n += 1;
            if usize::from(*s) != truth[k] {
                errors += 1;
            }
        }
        assert!(n > 250, "only {n} symbols decoded");
        assert_eq!(errors, 0, "{errors} of {n} symbols wrong");

        // C4FM needs 4 levels: a 2-level grid is rejected before any polarity check.
        let mut two = Cli::try_parse_from([
            "sn-cli",
            "fsk-demod",
            path.to_str().unwrap(),
            "--edit",
            r#"{"op":"phase_detector"}"#,
            "--baud",
            "100",
            "--lo",
            "1200",
            "--hi",
            "1800",
            "--count",
            "2",
            "--format",
            "dibits",
            "--map",
            "c4fm",
        ])
        .unwrap();
        let Command::FskDemod(args) = &mut two.command else {
            panic!("expected fsk-demod")
        };
        let e = fsk_demod(args).unwrap_err();
        assert!(e.to_string().contains("needs 4 levels"), "{e}");
        // --width is bounded.
        assert!(
            Cli::try_parse_from([
                "sn-cli",
                "fsk-demod",
                "x.wav",
                "--baud",
                "100",
                "--width",
                "0"
            ])
            .is_err()
        );

        // `tones` finds the four tones and a 4-position grid.
        let s = tones(&path, &[r#"{"op":"phase_detector"}"#.to_owned()], 4.0).unwrap();
        assert_eq!(s.tones.len(), 4, "tones {:?}", s.tones);
        let sn_demod::mfsk::Levels::Grid { lo, hi, count } = s.grid else {
            panic!("expected a grid")
        };
        assert_eq!(count, 4);
        assert!((lo - 1200.0).abs() < 20.0 && (hi - 1800.0).abs() < 20.0);

        assert!(parse_levels("1, 2,x").is_err());
        assert_eq!(parse_levels("1, 2.5").unwrap(), vec![1.0, 2.5]);
    }

    #[test]
    fn ofdm_command_runs_on_a_synthetic_file() {
        let dir = tempfile::tempdir().unwrap();
        let input = dir.path().join("ofdm.wav");
        let p = sn_synth::OfdmParams {
            sample_rate: 7200.0,
            lu: 36,
            lg: 24,
            channels: 12,
            first_bin: -6,
            unused: vec![],
            modulation: sn_synth::OfdmModulation::Qpsk,
            overrides: vec![],
            symbols: 300,
            frequency_offset: 0.0,
            amplitude: 0.3,
            seed: 3,
        };
        let s = sn_synth::ofdm_real(&p, 1800.0).unwrap().signal;
        write_wav(&input, &s, WavFormat::Float32).unwrap();
        let cli = Cli::try_parse_from([
            "sn-cli",
            "ofdm",
            input.to_str().unwrap(),
            "--min-br",
            "60",
            "--max-br",
            "200",
        ])
        .unwrap();
        let Command::Ofdm {
            file,
            min_br,
            max_br,
            symbols,
            start,
            check,
            edits,
        } = cli.command
        else {
            panic!("expected ofdm")
        };
        let a = OfdmArgs {
            min_br,
            max_br,
            symbols,
            start,
            check,
        };
        let ct = ofdm(&file, &a, &edits).unwrap();
        assert_eq!((ct.lu, ct.lg), (36, 24));
        let bad = OfdmArgs {
            min_br: 200.0,
            max_br: 60.0,
            ..a
        };
        assert!(ofdm(&file, &bad, &edits).is_err());
    }

    #[test]
    fn frame_starts_cover_the_whole_file() {
        assert_eq!(frame_starts(100, 256, 2000), vec![0]);
        assert_eq!(frame_starts(256, 256, 2000), vec![0]);
        // Half-overlapped, last full frame included.
        assert_eq!(
            frame_starts(1024, 256, 2000),
            vec![0, 128, 256, 384, 512, 640, 768]
        );
        // Capped: spread evenly, first and last frame kept.
        let f = frame_starts(10_000_256, 256, 2000);
        assert_eq!(f.len(), 2000);
        assert_eq!(f.first(), Some(&0));
        assert_eq!(f.last(), Some(&10_000_000));
        assert_eq!(frame_starts(1000, 1, 2000).len(), 1000);
    }

    #[test]
    fn peaks_find_a_carrier_late_in_a_long_file() {
        // 1.2 M samples of silence, then a 1000 Hz tone: past the first 2000 frames.
        let fs = 8000.0;
        let mut x = vec![0.0_f32; 1_200_000];
        x.extend(
            (0..100_000)
                .map(|n| (0.5 * (std::f64::consts::TAU * 1000.0 * f64::from(n) / fs).sin()) as f32),
        );
        let s = Signal::real(x, fs).unwrap();
        let (_, found) = spectrum_peaks(&s, 256, 3).unwrap();
        let top = found.first().unwrap();
        assert!(
            (top.frequency - 1000.0).abs() <= fs / 256.0,
            "strongest peak at {} Hz",
            top.frequency
        );

        // Through the file command too.
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("t.wav");
        write_wav(
            &path,
            &tone(1500.0, 0.5, 8000.0, 2.0).unwrap(),
            WavFormat::Pcm16,
        )
        .unwrap();
        let found = peaks(&path, 1024, 5).unwrap();
        assert!((found.first().unwrap().frequency - 1500.0).abs() <= 8000.0 / 1024.0);
    }

    #[test]
    fn measure_takes_a_span_and_names_the_cli_flags() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("t.wav");
        write_wav(
            &path,
            &tone(1000.0, 0.5, 8000.0, 1.0).unwrap(),
            WavFormat::Float32,
        )
        .unwrap();
        let all = SpanArgs::default();
        let e = measure(&path, 900.0, 1100.0, &[], &all).unwrap();
        assert!((e.frequency - 1000.0).abs() < 0.1, "{}", e.frequency);
        let half = SpanArgs {
            start: Some(0.25),
            end: Some(0.75),
        };
        let e = measure(&path, 900.0, 1100.0, &[], &half).unwrap();
        assert!((e.frequency - 1000.0).abs() < 0.1, "{}", e.frequency);

        let short = SpanArgs {
            start: Some(0.5),
            end: Some(0.501),
        };
        let err = measure(&path, 900.0, 1100.0, &[], &short).unwrap_err();
        assert!(err.to_string().contains("--start/--end"), "{err}");
        let empty = SpanArgs {
            start: Some(0.8),
            end: Some(0.2),
        };
        let err = measure(&path, 900.0, 1100.0, &[], &empty).unwrap_err();
        assert!(err.to_string().contains("is empty"), "{err}");
        let err = measure(&path, 1100.0, 900.0, &[], &all).unwrap_err();
        assert!(err.to_string().contains("--hi"), "{err}");
        let negative = SpanArgs {
            start: Some(-1.0),
            end: None,
        };
        assert!(measure(&path, 900.0, 1100.0, &[], &negative).is_err());
        // A failing edit names its position in the chain.
        let err = measure(
            &path,
            900.0,
            1100.0,
            &[r#"{"op":"resample","rate":-5}"#.to_owned()],
            &all,
        )
        .unwrap_err();
        assert!(err.to_string().starts_with("edit 1"), "{err}");
    }

    #[test]
    fn phase_diff_is_limited_to_0_to_3() {
        let args = |d: &'static str| {
            [
                "sn-cli",
                "phase",
                "x.wav",
                "--carrier",
                "1800",
                "--baud",
                "2400",
                "--diff",
                d,
            ]
        };
        assert!(Cli::try_parse_from(args("3")).is_ok());
        assert!(Cli::try_parse_from(args("4")).is_err());
    }

    #[test]
    fn cli_definition_is_valid() {
        Cli::command().debug_assert();
    }

    #[test]
    fn phase0_exit_command_writes_a_psk4_file() {
        let dir = tempfile::tempdir().unwrap();
        let out = dir.path().join("x.wav");
        let cli = Cli::try_parse_from([
            "sn-cli",
            "synth",
            "psk",
            "--order",
            "4",
            "--baud",
            "2400",
            "-o",
            out.to_str().unwrap(),
        ])
        .unwrap();
        let Command::Synth { kind } = cli.command else {
            panic!("expected synth")
        };
        synth(kind).unwrap();
        let info = probe(&out).unwrap();
        assert_eq!((info.channels, info.sample_rate), (1, 48_000.0));
        assert!(
            (info.duration().unwrap() - 2.0).abs() < 1e-3,
            "4800 symbols at 2400 Bd = 2 s"
        );
    }

    fn generate_args(extra: &[&str]) -> Result<GenerateArgs> {
        let mut a = vec!["sn-cli", "generate"];
        a.extend_from_slice(extra);
        match Cli::try_parse_from(a)?.command {
            Command::Generate(g) => Ok(g),
            other => anyhow::bail!("expected generate, got {other:?}"),
        }
    }

    fn run_generate(extra: &[&str]) -> Result<String> {
        let mut out = Vec::new();
        generate(&generate_args(extra)?, &mut out)?;
        Ok(String::from_utf8(out)?)
    }

    #[test]
    fn generate_lists_prints_and_writes() {
        let dir = tempfile::tempdir().unwrap();
        // The presets, one per line.
        let list = run_generate(&["--list-presets"]).unwrap();
        assert_eq!(list.lines().collect::<Vec<_>>(), PRESET_NAMES.to_vec());
        // Print a preset, edit it, generate from the file.
        let json = run_generate(&["--preset", "qam16_rrc_1200", "--print-spec"]).unwrap();
        let mut spec: GeneratorSpec = serde_json::from_str(&json).unwrap();
        spec.common_mut().length = sn_synth::Length::Seconds(0.5);
        let spec_path = dir.path().join("spec.json");
        std::fs::write(&spec_path, serde_json::to_string(&spec).unwrap()).unwrap();
        let wav = dir.path().join("q.wav");
        let msg = run_generate(&[
            "--spec",
            spec_path.to_str().unwrap(),
            "-o",
            wav.to_str().unwrap(),
        ])
        .unwrap();
        assert!(msg.contains("QAM-16, 1200 Bd, RRC 0.35"), "{msg}");
        let info = probe(&wav).unwrap();
        assert_eq!((info.channels, info.sample_rate), (1, 8000.0));
        // The file holds exactly what the generator makes.
        let want = sn_synth::generate(&spec).unwrap().signal;
        let got = open_signal(&wav).unwrap();
        assert_eq!(got.len(), want.len());
        let (mut a, mut b) = (vec![0.0_f32; want.len()], vec![0.0_f32; want.len()]);
        want.read_real(0, &mut a);
        got.read_real(0, &mut b);
        assert_eq!(a, b);
        // A kind's default with another seed, as complex SigMF.
        let kind_json = run_generate(&["--kind", "ofdm", "--seed", "9", "--print-spec"]).unwrap();
        let mut ofdm: GeneratorSpec = serde_json::from_str(&kind_json).unwrap();
        assert_eq!(ofdm.common().seed, 9);
        if let GeneratorSpec::Ofdm(o) = &mut ofdm {
            o.complex = true;
            o.center = 0.0;
            o.common.length = sn_synth::Length::Symbols(20);
            o.common.center_frequency = 7_100_000.0;
        }
        std::fs::write(&spec_path, serde_json::to_string(&ofdm).unwrap()).unwrap();
        let sigmf = dir.path().join("o.sigmf-meta");
        run_generate(&[
            "--spec",
            spec_path.to_str().unwrap(),
            "-o",
            sigmf.to_str().unwrap(),
        ])
        .unwrap();
        let s = open_signal(&sigmf).unwrap();
        assert_eq!(s.kind(), sn_core::SampleKind::Complex);
        assert_eq!(s.len(), 20 * 80);
        assert!((s.center_frequency() - 7_100_000.0).abs() < 1e-3);
        // The same signal as WAV is written, but the centre frequency is lost and the
        // user is told so.
        let wav_iq = dir.path().join("o.wav");
        assert!(lost_center_frequency_warning(&sigmf, &s).is_none());
        let warning = lost_center_frequency_warning(&wav_iq, &s).unwrap();
        assert!(warning.contains("7100000 Hz"), "{warning}");
        assert!(warning.contains("o.sigmf-meta"), "{warning}");
        run_generate(&[
            "--spec",
            spec_path.to_str().unwrap(),
            "-o",
            wav_iq.to_str().unwrap(),
        ])
        .unwrap();
        let got = open_signal(&wav_iq).unwrap();
        assert_eq!(got.kind(), sn_core::SampleKind::Complex);
        assert_eq!(got.center_frequency(), 0.0);
        let real = tone(1000.0, 0.5, 8000.0, 0.1).unwrap();
        assert!(lost_center_frequency_warning(&wav_iq, &real).is_none());
    }

    #[test]
    fn generate_refuses_a_fractional_wav_rate_before_synthesis() {
        let dir = tempfile::tempdir().unwrap();
        // Far too long to generate: if the rate check came after synthesis, this
        // would fail on the length instead.
        let mut spec = GeneratorSpec::default_for(SignalKind::Linear);
        spec.common_mut().sample_rate = 8000.5;
        spec.common_mut().length = sn_synth::Length::Seconds(1e9);
        let spec_path = dir.path().join("spec.json");
        std::fs::write(&spec_path, serde_json::to_string(&spec).unwrap()).unwrap();
        let wav = dir.path().join("x.wav");
        let err = run_generate(&[
            "--spec",
            spec_path.to_str().unwrap(),
            "-o",
            wav.to_str().unwrap(),
        ])
        .unwrap_err();
        assert!(err.to_string().contains("x.sigmf-meta"), "{err}");
        assert!(!wav.exists());
    }

    #[test]
    fn generate_explains_its_errors() {
        let dir = tempfile::tempdir().unwrap();
        let o = dir.path().join("x.wav");
        let o = o.to_str().unwrap();
        let err = run_generate(&["--preset", "nope", "-o", o]).unwrap_err();
        assert!(err.to_string().contains("mfsk_11"), "{err}");
        // A fractional rate needs SigMF; this is refused before synthesis.
        let err = run_generate(&["--preset", "mil188_110b_39tone_like", "-o", o]).unwrap_err();
        assert!(err.to_string().contains("sigmf-meta"), "{err}");
        // An invalid spec file names the way out.
        let bad = dir.path().join("bad.json");
        std::fs::write(&bad, r#"{"kind": "warp"}"#).unwrap();
        let err = run_generate(&["--spec", bad.to_str().unwrap(), "-o", o]).unwrap_err();
        assert!(err.to_string().contains("--print-spec"), "{err}");
        // Output is required to generate, and the sources exclude each other.
        assert!(generate_args(&[]).is_err());
        assert!(generate_args(&["--preset", "mfsk_11", "--kind", "fsk", "-o", o]).is_err());
        assert!(generate_args(&["--print-spec", "-o", o]).is_err());
        // `synth` still works next to it.
        assert!(Cli::try_parse_from(["sn-cli", "synth", "tone", "-o", o]).is_ok());
    }

    #[test]
    fn synth_noise_by_esn0() {
        let dir = tempfile::tempdir().unwrap();
        let out = dir.path().join("n.wav");
        let o = out.to_str().unwrap();
        let parse = |extra: &[&str]| {
            let mut a = vec!["sn-cli", "synth"];
            a.extend_from_slice(extra);
            a.extend_from_slice(&["-o", o]);
            Cli::try_parse_from(a)
        };
        // SNR and Es/N0 are alternatives.
        assert!(parse(&["psk", "--snr-db", "10", "--esn0-db", "10"]).is_err());
        let Command::Synth { kind } = parse(&["psk", "--esn0-db", "10"]).unwrap().command else {
            panic!("expected synth")
        };
        synth(kind).unwrap();
        assert!(probe(&out).is_ok());
        // A tone has no symbol rate.
        let Command::Synth { kind } = parse(&["tone", "--esn0-db", "10"]).unwrap().command else {
            panic!("expected synth")
        };
        let err = synth(kind).unwrap_err().to_string();
        assert!(err.contains("symbol rate"), "{err}");
    }
}
