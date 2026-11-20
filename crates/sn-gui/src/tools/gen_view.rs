//! The signal generator window: the kind and its presets, the editor of the
//! kind's own fields, the common fields, the channel impairments, the derived
//! values and the preview in group boxes that scroll, and under them the
//! validation message with `Generate`, `Generate and save as...` and
//! `Calculators...` (`sa-app`'s `Generator::show`).

use gpui::{AnyElement, Context, IntoElement, ParentElement, Styled, Window, div};
use sn_synth::calc;
use sn_synth::{
    C4fmSpec, Constellation, FskShaping, FskSpec, GeneratorSpec, LinearSpec, NoiseLevel, OfdmModel,
    OfdmSpec, PilotPattern, Plan, Shaping, SignalKind, ToneSpec,
};
use sn_ui_core::generator::{fraction, trim};
use sn_ui_core::{format, t};

use super::fields::Num;
use super::form::{Builder, Tone, layout, measurer};
use super::{Btn, Pick, TextField, Toggle, Tool};
use crate::app::Frame;
use crate::win95::geom::Rect;

/// Height of the preview plot.
pub(crate) const PREVIEW_H: i32 = 130;
/// Least height of the fixed strip with the message and the buttons.
const STRIP_H: i32 = 58;

impl Frame {
    /// The generator window's client area (`cw` x `ch`).
    pub(crate) fn generator_body(
        &mut self,
        win: u32,
        cw: i32,
        ch: i32,
        window: &mut Window,
        cx: &mut Context<'_, Self>,
    ) -> AnyElement {
        let problem = self.tools.problem();
        let plan = self
            .tools
            .ocg
            .spec
            .plan()
            .ok()
            .filter(|_| problem.is_none());
        let measure = measurer(window);
        let mut b = Builder::new(&measure, cw - crate::win95::scrollbar::THICKNESS);
        self.generator_rows(&mut b, plan.as_ref());
        let form = b.finish();
        let mut s = Builder::new(&measure, cw);
        self.strip_rows(&mut s, problem.as_deref(), plan.as_ref());
        let strip_form = s.finish();
        // The strip grows with a long (wrapped) message, up to half the window.
        let strip_h = layout(&strip_form, cw)
            .height
            .clamp(STRIP_H, (ch / 2).max(STRIP_H));
        let view = Rect::new(0, 0, cw, (ch - strip_h).max(0));
        let body = self.form_view(Tool::Generator, win, &form, view, cx);
        let strip = self.form_fixed(
            win,
            &strip_form,
            Rect::new(0, ch - strip_h, cw, strip_h),
            cx,
        );
        div()
            .relative()
            .size_full()
            .child(body)
            .child(strip)
            .into_any_element()
    }

    /// The calculators window's client area.
    pub(crate) fn calc_body(
        &mut self,
        win: u32,
        cw: i32,
        ch: i32,
        window: &mut Window,
        cx: &mut Context<'_, Self>,
    ) -> AnyElement {
        let measure = measurer(window);
        let mut b = Builder::new(&measure, cw - crate::win95::scrollbar::THICKNESS);
        let selected = (
            self.table_selected(super::TableId::Candidates),
            self.table_selected(super::TableId::KList),
        );
        let hub = &mut self.tools;
        let why = hub.ocg.fraction_error().map(str::to_owned);
        super::calc::rows(&mut b, &mut hub.calc, &hub.errors, why.as_deref(), selected);
        let form = b.finish();
        self.form_view(Tool::Calculators, win, &form, Rect::new(0, 0, cw, ch), cx)
    }

    /// The fixed strip: the message, then the buttons.
    fn strip_rows(&self, b: &mut Builder<'_>, problem: Option<&str>, plan: Option<&Plan>) {
        let generating = self.generating();
        match (problem, plan) {
            // Wrapped: the field, the bad text and the range are often wider
            // than the window, more so in other languages.
            (Some(e), _) => b.wrapped(e, Tone::Error),
            (None, Some(p)) => {
                let fs = self.tools.ocg.spec.common().sample_rate;
                let ready = t!(
                    "gen-ready",
                    samples = format::count(p.samples as u64),
                    duration = trim(p.duration(fs), 3)
                );
                b.line().text(&ready, Tone::Ok);
            }
            (None, None) => {
                b.line();
            }
        }
        let can = problem.is_none() && plan.is_some() && !generating;
        let why = if generating {
            t!("gen-busy")
        } else {
            t!("gen-fix-first")
        };
        let r = b.line().when(can);
        r.button(Btn::Generate, &t!("gui-gen-generate"))
            .hint(&if can {
                t!("gen-generate.hover")
            } else {
                why.clone()
            });
        r.button(Btn::GenerateSave, &t!("gui-gen-generate-save"))
            .hint(&if can {
                t!("gen-generate-save.hover")
            } else {
                why
            });
        r.when(true)
            .button(Btn::Calculators, &t!("gui-gen-calculators"));
    }

    /// The scrolling part of the generator.
    fn generator_rows(&mut self, b: &mut Builder<'_>, plan: Option<&Plan>) {
        b.row(&t!("gen-kind"))
            .combo(Pick::Kind, 140)
            .button(Btn::Preset, &t!("gui-gen-preset"))
            .hint(&t!("gen-preset.hover"));
        b.group(&t!("gen-section-signal"));
        kind_rows(b, &self.tools.ocg.spec);
        b.group(&t!("gen-section-common"));
        common_rows(b, &self.tools.ocg.spec, plan);
        b.group(&t!("gen-section-channel"));
        channel_rows(b, &self.tools.ocg.spec);
        b.group(&t!("gen-section-derived"));
        derived_rows(b, &self.tools.ocg.spec, plan);
        b.group(&t!("gen-section-preview"));
        let r = b
            .line()
            .radio(Pick::Preview, 0, &t!("gen-preview-spectrum"))
            .radio(Pick::Preview, 1, &t!("gen-preview-wave"));
        if self.tools.ocg.preview_busy() {
            r.text(&t!("gen-preview-building"), Tone::Dim);
        }
        b.line().preview(PREVIEW_H);
        b.end_group();
    }

    /// A form without a scroll bar in `view`.
    fn form_fixed(
        &mut self,
        win: u32,
        form: &super::form::Form,
        view: Rect,
        cx: &mut Context<'_, Self>,
    ) -> AnyElement {
        let lay = layout(form, view.w);
        self.form_static(win, form, &lay, view, cx)
    }
}

/// The editor of the kind's own fields.
fn kind_rows(b: &mut Builder<'_>, spec: &GeneratorSpec) {
    match spec {
        GeneratorSpec::Tones(s) => tones_rows(b, &s.tones),
        GeneratorSpec::Noise(_) => {
            b.row(&t!("gen-amplitude-rms")).num(Num::Amplitude);
            complex_row(b);
        }
        GeneratorSpec::Fsk(s) => fsk_rows(b, s),
        GeneratorSpec::Linear(s) => linear_rows(b, s),
        GeneratorSpec::C4fm(s) => c4fm_rows(b, s),
        GeneratorSpec::Ofdm(o) => ofdm_rows(b, o),
    }
}

fn complex_row(b: &mut Builder<'_>) {
    b.aligned()
        .check(Toggle::Complex, &t!("gen-complex"))
        .hint(&t!("gen-complex.hover"));
}

fn tones_rows(b: &mut Builder<'_>, tones: &[ToneSpec]) {
    let several = tones.len() > 1;
    b.columns(&[
        (t!("gen-tones-frequency"), super::form::NUM_W, "Hz"),
        (t!("gen-amplitude-peak"), 70, ""),
        (t!("gen-phase-rad"), 70, ""),
    ]);
    for i in 0..tones.len() {
        let r = b
            .row(&(i + 1).to_string())
            .num(Num::ToneFrequency(i))
            .num_w(Num::ToneAmplitude(i), 70)
            .num_w(Num::TonePhase(i), 70)
            .when(several)
            .small_button(Btn::RemoveTone(i), &t!("gen-remove"));
        if !several {
            r.hint(&t!("gen-tones-last"));
        }
    }
    b.aligned()
        .when(tones.len() < sn_synth::spec::MAX_TONES)
        .button(Btn::AddTone, &t!("gen-tones-add"))
        .when(true)
        .check(Toggle::Complex, &t!("gen-complex"))
        .hint(&t!("gen-complex.hover"));
}

fn fsk_rows(b: &mut Builder<'_>, s: &FskSpec) {
    b.row(&t!("gen-fsk-order")).num(Num::Order);
    b.row(&t!("gen-baud")).num(Num::Baud);
    b.row(&t!("gen-fsk-spacing")).num(Num::Spacing);
    b.row(&t!("gen-center")).num(Num::Center);
    b.row(&t!("gen-amplitude-peak")).num(Num::Amplitude);
    let r = b
        .row(&t!("gen-fsk-shaping"))
        .radio(Pick::FskShaping, 0, &t!("gen-fsk-shaping-none"))
        .radio(Pick::FskShaping, 1, &t!("gen-fsk-shaping-gaussian"));
    if let FskShaping::Gaussian { .. } = s.shaping {
        r.text(&t!("gen-fsk-bt"), Tone::Plain).num_w(Num::Bt, 60);
    }
    b.aligned()
        .small_button(Btn::Msk, &t!("gen-fsk-msk"))
        .hint(&t!("gen-fsk-msk.hover"))
        .small_button(Btn::Gmsk, &t!("gen-fsk-gmsk"))
        .hint(&t!("gen-fsk-gmsk.hover"))
        .check(Toggle::Complex, &t!("gen-complex"))
        .hint(&t!("gen-complex.hover"));
}

fn linear_rows(b: &mut Builder<'_>, s: &LinearSpec) {
    b.row(&t!("gen-lin-constellation"))
        .combo(Pick::Constellation, 120);
    b.row(&t!("gen-baud")).num(Num::Baud);
    b.row(&t!("gen-carrier")).num(Num::Carrier);
    b.row(&t!("gen-amplitude-rms")).num(Num::Amplitude);
    let r = b.row(&t!("gen-lin-shaping")).combo(Pick::Shaping, 190);
    if let Shaping::RaisedCosine { .. } | Shaping::RootRaisedCosine { .. } = s.shaping {
        r.text(&t!("gen-alpha"), Tone::Plain).num_w(Num::Alpha, 60);
    }
    let psk = s.constellation.is_psk();
    let pi4 = s.constellation == Constellation::Pi4Dqpsk;
    let hint = if !psk {
        t!("gen-lin-differential-qam")
    } else if pi4 {
        t!("gen-lin-differential-pi4")
    } else {
        t!("gen-lin-differential.hover")
    };
    b.aligned()
        .when(psk && !pi4)
        .check(Toggle::Differential, &t!("gen-lin-differential"))
        .hint(&hint)
        .when(true)
        .check(Toggle::Complex, &t!("gen-complex"))
        .hint(&t!("gen-complex.hover"));
}

fn c4fm_rows(b: &mut Builder<'_>, s: &C4fmSpec) {
    b.row(&t!("gen-baud")).num(Num::Baud);
    b.row(&t!("gen-carrier")).num(Num::Carrier);
    b.row(&t!("gen-amplitude-peak")).num(Num::Amplitude);
    b.row(&t!("gen-c4fm-pulse"))
        .check(Toggle::Shaped, &t!("gen-c4fm-shaped"))
        .hint(&t!("gen-c4fm-shaped.hover"))
        .when(s.shaped)
        .radio(Pick::C4fmPulse, 0, "RC 0.2")
        .radio(Pick::C4fmPulse, 1, &t!("gen-c4fm-apco"))
        .hint(&t!("gen-c4fm-apco.hover"));
    b.wrapped(&t!("gen-c4fm-note"), Tone::Dim);
}

fn ofdm_rows(b: &mut Builder<'_>, o: &OfdmSpec) {
    b.row(&t!("gen-ofdm-lu"))
        .num(Num::Lu)
        .hint(&t!("gen-ofdm-lu.hover"));
    b.row(&t!("gen-ofdm-lg"))
        .num(Num::Lg)
        .hint(&t!("gen-ofdm-lg.hover"));
    b.row(&t!("gen-ofdm-channels")).num(Num::Channels);
    b.row(&t!("gen-ofdm-first-bin"))
        .num(Num::FirstBin)
        .hint(&t!("gen-ofdm-first-bin.hover"));
    b.row(&t!("gen-ofdm-modulation"))
        .combo(Pick::Modulation, 120);
    b.row(&t!("gen-ofdm-unused"))
        .field(TextField::Unused, 200)
        .hint(&t!("gen-ofdm-unused.hover"))
        .text("3, 7, 10-12", Tone::Dim);
    b.row(&t!("gen-ofdm-overrides"))
        .field(TextField::Overrides, 200)
        .hint(&t!("gen-ofdm-overrides.hover"))
        .text("0: pilot, 5-6: QAM-16", Tone::Dim);
    let r = b.row(&t!("gen-ofdm-model"));
    for (i, (m, key)) in [
        (OfdmModel::A, "gen-ofdm-model-a"),
        (OfdmModel::B, "gen-ofdm-model-b"),
        (OfdmModel::C, "gen-ofdm-model-c"),
    ]
    .into_iter()
    .enumerate()
    {
        r.radio(Pick::Model, i, m.name()).hint(&t!(key));
    }
    b.row(&t!("gen-ofdm-pilots"))
        .radio(Pick::Pilots, 0, &t!("gen-ofdm-pilots-none"))
        .radio(Pick::Pilots, 1, &t!("gen-ofdm-pilots-list"))
        .radio(Pick::Pilots, 2, &t!("gen-ofdm-pilots-every"));
    match o.pilots {
        PilotPattern::None => {}
        PilotPattern::Positions(_) => {
            b.aligned()
                .field(TextField::Pilots, 140)
                .hint(&t!("gen-ofdm-pilots-list.hover"))
                .text("0, 8, 16", Tone::Dim);
        }
        PilotPattern::Every { .. } => {
            b.aligned()
                .text(&t!("gen-ofdm-step"), Tone::Plain)
                .num_w(Num::PilotStep, 60)
                .text(&t!("gen-ofdm-offset"), Tone::Plain)
                .num_w(Num::PilotOffset, 60);
        }
    }
    let model_c = o.model == OfdmModel::C;
    let hint = if model_c {
        t!("gen-ofdm-reference.hover")
    } else {
        t!("gen-ofdm-reference.disabled")
    };
    b.row(&t!("gen-ofdm-reference"))
        .when(model_c)
        .radio(Pick::Reference, 0, &t!("gen-ofdm-reference-constant"))
        .hint(&hint)
        .radio(Pick::Reference, 1, &t!("gen-ofdm-reference-differential"))
        .hint(&hint);
    // OFDM amplitude is RMS.
    b.row(&t!("gen-amplitude-rms")).num(Num::Amplitude);
    b.row(&t!("gen-ofdm-center"))
        .num(Num::Center)
        .hint(&t!("gen-ofdm-center.hover"));
    complex_row(b);
}

/// Label, sample rate, length, seed and centre frequency.
fn common_rows(b: &mut Builder<'_>, spec: &GeneratorSpec, plan: Option<&Plan>) {
    let c = spec.common();
    let has_symbols = !matches!(spec.kind(), SignalKind::Tones | SignalKind::Noise);
    let complex = plan.map_or_else(
        || sn_ui_core::generator::spec_is_complex(spec),
        |p| p.complex,
    );
    b.row(&t!("gen-label"))
        .field(TextField::Label, 240)
        .hint(&t!("gen-label.hover"));
    b.row(&t!("gen-sample-rate")).num(Num::SampleRate);
    let seconds = matches!(c.length, sn_synth::Length::Seconds(_));
    let r = b
        .row(&t!("gen-length"))
        .radio(Pick::Length, 0, &t!("gen-length-seconds"))
        .when(has_symbols)
        .radio(Pick::Length, 1, &t!("gen-length-symbols"));
    if !has_symbols {
        r.hint(&t!("gen-length-symbols.disabled"));
    }
    b.aligned()
        .num(if seconds { Num::Seconds } else { Num::Symbols });
    b.row(&t!("gen-seed"))
        .num_w(Num::Seed, 110)
        .small_button(Btn::NewSeed, &t!("gen-new-seed"))
        .hint(&t!("gen-new-seed.hover"));
    b.row(&t!("gen-center-frequency"))
        .when(complex)
        .num(Num::CenterFrequency)
        .hint(&if complex {
            t!("gen-center-frequency.hover")
        } else {
            t!("gen-center-frequency.disabled")
        });
}

/// The channel impairments.
fn channel_rows(b: &mut Builder<'_>, spec: &GeneratorSpec) {
    let ch = &spec.common().channel;
    b.row(&t!("gen-ch-offset")).num(Num::Offset);
    b.row(&t!("gen-ch-drift")).num(Num::Drift);
    b.row(&t!("gen-ch-clock"))
        .num(Num::Clock)
        .hint(&t!("gen-ch-clock.hover"));
    let r = b.row(&t!("gen-ch-noise")).combo(Pick::Noise, 170);
    if ch.noise.is_some() {
        r.num_w(Num::NoiseDb, 70);
    }
    if let Some(NoiseLevel::Snr { bandwidth, .. }) = ch.noise {
        let r = b
            .aligned()
            .check(Toggle::Bandwidth, &t!("gen-ch-bandwidth"))
            .hint(&t!("gen-ch-bandwidth.hover"));
        if bandwidth.is_some() {
            r.num(Num::Bandwidth);
        }
    }
    let r = b
        .row(&t!("gen-ch-clip"))
        .check(Toggle::Clip, "")
        .hint(&t!("gen-ch-clip.hover"));
    if ch.clip.is_some() {
        r.num_w(Num::Clip, 70);
    }
    b.line()
        .text(&t!("gen-ch-multipath"), Tone::Plain)
        .hint(&t!("gen-ch-multipath.hover"));
    if !ch.multipath.is_empty() {
        b.columns(&[
            (t!("gen-ch-delay"), 70, "ms"),
            (t!("gen-ch-gain"), 70, ""),
            (t!("gen-phase-rad"), 70, ""),
        ]);
    }
    for i in 0..ch.multipath.len() {
        b.row(&(i + 1).to_string())
            .num_w(Num::TapDelay(i), 70)
            .num_w(Num::TapGain(i), 70)
            .num_w(Num::TapPhase(i), 70)
            .small_button(Btn::RemoveTap(i), &t!("gen-remove"));
    }
    b.aligned()
        .when(ch.multipath.len() < sn_synth::MAX_TAPS)
        .button(Btn::AddTap, &t!("gen-ch-add-tap"));
}

/// Values that follow from the parameters.
fn derived_rows(b: &mut Builder<'_>, spec: &GeneratorSpec, plan: Option<&Plan>) {
    let Some(p) = plan else {
        b.wrapped(&t!("gen-d-none"), Tone::Dim);
        return;
    };
    let fs = spec.common().sample_rate;
    let mut row = |key: &str, value: String| {
        b.row(&t!(key)).text(&value, Tone::Bold);
    };
    if let Some(r) = p.symbol_rate {
        row("gen-d-symbol-rate", format!("{} Bd", trim(r, 4)));
        row("gen-d-sps", trim(fs / r, 4));
    }
    if let Some(n) = p.symbols {
        row("gen-d-symbols", format::count(n as u64));
    }
    row(
        "gen-d-bandwidth",
        format!("{} Hz", trim(p.occupied_bandwidth, 3)),
    );
    row("gen-d-samples", format::count(p.samples as u64));
    row("gen-d-duration", format!("{} s", trim(p.duration(fs), 4)));
    match spec {
        GeneratorSpec::Fsk(s) if s.baud > 0.0 => {
            row("gen-d-index", trim(s.spacing / s.baud, 4));
        }
        GeneratorSpec::Ofdm(o) => {
            if let Ok(c) = calc::cp_from_lengths(fs, o.lu as f64, o.lg as f64) {
                row("gen-d-ls", trim(c.ls, 0));
                row(
                    "gen-d-k",
                    format!("{} = {}", trim(c.k, 6), fraction(o.lg, o.lu)),
                );
                row("gen-d-spacing", format!("{} Hz", trim(c.spacing, 4)));
                row("gen-d-br", format!("{} Bd", trim(c.symbol_rate, 4)));
            }
        }
        _ => {}
    }
}
