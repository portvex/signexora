//! OCG's calculators (`sa-app`'s `Calculators` section of the generator):
//! OFDM with a cyclic prefix from Fd, LU and LG, the nearest whole LU and LG
//! for measured rates, the whole LU and LG for Br and K, and the List of K.
//! Each result has `Use in OFDM`, which copies it into the generator's OFDM
//! editor. Here they open in their own tool window (`Calculators...` in the
//! generator), with list boxes for the two tables.

use std::collections::BTreeMap;

use sn_synth::calc::{self, CpCandidate, KCandidate, Ranked};
use sn_ui_core::generator::{UseFraction, UseLengths, check_lengths, k_candidates, trim};
use sn_ui_core::t;

use super::fields::{CalcNum, Num};
use super::form::{Builder, Tone};
use super::{Btn, Pick, TableId};

/// Most rows of the candidates and K tables.
pub(crate) const TABLE_ROWS: usize = 200;
/// Rows of a table shown at once.
pub(crate) const TABLE_SHOWN: i32 = 6;

/// Inputs of the candidates calculator: Br, K, LU range and tolerance.
type CandidatesKey = (f64, f64, u64, u64, f64);
/// Inputs of the K list.
type KListKey = (bool, f64, f64, f64, f64, u32, usize);

/// Inputs and cached results of the calculators.
#[derive(Debug, Clone)]
pub(crate) struct CalcInputs {
    pub(crate) fd: f64,
    pub(crate) lu: f64,
    pub(crate) lg: f64,
    pub(crate) inv_br: f64,
    pub(crate) inv_spacing: f64,
    pub(crate) inv_fd: f64,
    pub(crate) cand_br: f64,
    pub(crate) cand_k: f64,
    pub(crate) cand_lu_min: u64,
    pub(crate) cand_lu_max: u64,
    pub(crate) cand_tol: f64,
    cand_cache: Option<(CandidatesKey, Result<Ranked<CpCandidate>, String>)>,
    pub(crate) k_from_rates: bool,
    pub(crate) k_br: f64,
    pub(crate) k_spacing: f64,
    pub(crate) k_value: f64,
    pub(crate) k_pct: f64,
    pub(crate) k_max_den: u32,
    pub(crate) k_count: usize,
    k_cache: Option<(KListKey, Result<Vec<KCandidate>, String>)>,
}

impl Default for CalcInputs {
    fn default() -> Self {
        // OCG's test signal: LU 64, LG 20 at Fd 7000 Hz.
        Self {
            fd: 7000.0,
            lu: 64.0,
            lg: 20.0,
            inv_br: 83.333,
            inv_spacing: 109.375,
            inv_fd: 7000.0,
            cand_br: 83.333,
            cand_k: 0.3125,
            cand_lu_min: 16,
            cand_lu_max: 256,
            cand_tol: 0.05,
            cand_cache: None,
            k_from_rates: true,
            k_br: 83.333,
            k_spacing: 109.375,
            k_value: 0.3125,
            k_pct: 0.1,
            k_max_den: 64,
            k_count: 12,
            k_cache: None,
        }
    }
}

impl CalcInputs {
    pub(crate) fn get(&self, n: CalcNum) -> f64 {
        match n {
            CalcNum::Fd => self.fd,
            CalcNum::Lu => self.lu,
            CalcNum::Lg => self.lg,
            CalcNum::InvBr => self.inv_br,
            CalcNum::InvSpacing => self.inv_spacing,
            CalcNum::InvFd => self.inv_fd,
            CalcNum::CandBr => self.cand_br,
            CalcNum::CandK => self.cand_k,
            CalcNum::CandLuMin => self.cand_lu_min as f64,
            CalcNum::CandLuMax => self.cand_lu_max as f64,
            CalcNum::CandTol => self.cand_tol,
            CalcNum::KBr => self.k_br,
            CalcNum::KSpacing => self.k_spacing,
            CalcNum::KValue => self.k_value,
            CalcNum::KPct => self.k_pct,
            CalcNum::KMaxDen => f64::from(self.k_max_den),
            CalcNum::KCount => self.k_count as f64,
        }
    }

    pub(crate) fn set(&mut self, n: CalcNum, v: f64) {
        match n {
            CalcNum::Fd => self.fd = v,
            CalcNum::Lu => self.lu = v,
            CalcNum::Lg => self.lg = v,
            CalcNum::InvBr => self.inv_br = v,
            CalcNum::InvSpacing => self.inv_spacing = v,
            CalcNum::InvFd => self.inv_fd = v,
            CalcNum::CandBr => self.cand_br = v,
            CalcNum::CandK => self.cand_k = v,
            CalcNum::CandLuMin => self.cand_lu_min = v as u64,
            CalcNum::CandLuMax => self.cand_lu_max = v as u64,
            CalcNum::CandTol => self.cand_tol = v,
            CalcNum::KBr => self.k_br = v,
            CalcNum::KSpacing => self.k_spacing = v,
            CalcNum::KValue => self.k_value = v,
            CalcNum::KPct => self.k_pct = v,
            CalcNum::KMaxDen => self.k_max_den = v as u32,
            CalcNum::KCount => self.k_count = v as usize,
        }
    }

    /// The candidates for the current inputs (computed when they change).
    pub(crate) fn candidates(&mut self) -> &Result<Ranked<CpCandidate>, String> {
        let key = (
            self.cand_br,
            self.cand_k,
            self.cand_lu_min,
            self.cand_lu_max,
            self.cand_tol,
        );
        if self.cand_cache.as_ref().is_none_or(|(k, _)| *k != key) {
            // Only the rows the table shows are kept, so a wide LU range stays cheap.
            let r = calc::cp_candidates(key.0, key.1, key.2, key.3, key.4, TABLE_ROWS)
                .map_err(|e| e.to_string());
            self.cand_cache = Some((key, r));
        }
        match &self.cand_cache {
            Some((_, r)) => r,
            None => &EMPTY_CANDIDATES,
        }
    }

    /// The List of K for the current inputs (computed when they change).
    pub(crate) fn k_list(&mut self) -> &Result<Vec<KCandidate>, String> {
        let key = (
            self.k_from_rates,
            self.k_br,
            self.k_spacing,
            self.k_value,
            self.k_pct,
            self.k_max_den,
            self.k_count,
        );
        if self.k_cache.as_ref().is_none_or(|(k, _)| *k != key) {
            let (from_rates, br, spacing, k, pct, max_den, count) = key;
            self.k_cache = Some((
                key,
                k_candidates(from_rates, br, spacing, k, pct, max_den, count),
            ));
        }
        match &self.k_cache {
            Some((_, r)) => r,
            None => &EMPTY_K,
        }
    }

    /// The table's header and rows (columns separated by tabs).
    pub(crate) fn table_rows(&mut self, t: TableId) -> (String, Vec<String>) {
        match t {
            TableId::Candidates => {
                let head = format!("LU\tLG\tFd\tK\t{}", t!("gen-calc-error"));
                let rows = match self.candidates() {
                    Ok(r) => r
                        .best
                        .iter()
                        .take(TABLE_ROWS)
                        .map(|c| {
                            format!(
                                "{}\t{}\t{} Hz\t{}\t{}",
                                c.lu,
                                c.lg,
                                trim(c.fd, 3),
                                trim(c.k, 6),
                                trim(c.lg_error, 4)
                            )
                        })
                        .collect(),
                    Err(_) => Vec::new(),
                };
                (head, rows)
            }
            TableId::KList => {
                let head = format!("K\t{}\t{}", t!("gen-calc-value"), t!("gen-calc-k-error"));
                let rows = match self.k_list() {
                    Ok(list) => list
                        .iter()
                        .take(TABLE_ROWS)
                        .map(|k| {
                            let f = k.fraction;
                            format!(
                                "{}/{}\t{}\t{}",
                                f.p,
                                f.q,
                                trim(k.value, 6),
                                trim(k.error, 6)
                            )
                        })
                        .collect(),
                    Err(_) => Vec::new(),
                };
                (head, rows)
            }
        }
    }

    /// `Use in OFDM` of the CP calculator.
    pub(crate) fn use_cp(&self) -> Option<UseLengths> {
        self.use_cp_checked().ok()
    }

    /// [`Self::use_cp`], or why it is disabled: `Some` reason when the lengths
    /// do not fit the generator, `None` when the calculator shows its error.
    pub(crate) fn use_cp_checked(&self) -> Result<UseLengths, Option<String>> {
        calc::cp_from_lengths(self.fd, self.lu, self.lg).map_err(|_| None)?;
        let (lu, lg) = check_lengths(self.lu, self.lg).map_err(Some)?;
        Ok(UseLengths {
            fd: Some(self.fd),
            lu,
            lg,
        })
    }

    /// `Use in OFDM` of the rates calculator.
    pub(crate) fn use_inverse(&self) -> Option<UseLengths> {
        self.use_inverse_checked().ok()
    }

    /// [`Self::use_inverse`], or why it is disabled (as [`Self::use_cp_checked`]).
    pub(crate) fn use_inverse_checked(&self) -> Result<UseLengths, Option<String>> {
        let r =
            calc::cp_from_rates(self.inv_br, self.inv_spacing, self.inv_fd).map_err(|_| None)?;
        let (lu, lg) = check_lengths(r.lu as f64, r.lg as f64).map_err(Some)?;
        Ok(UseLengths {
            fd: Some(self.inv_fd),
            lu,
            lg,
        })
    }

    /// `Use in OFDM` of candidate row `i`.
    pub(crate) fn use_candidate(&mut self, i: usize) -> Option<UseLengths> {
        self.use_candidate_checked(i).ok()
    }

    /// [`Self::use_candidate`], or why it is disabled (as [`Self::use_cp_checked`]).
    pub(crate) fn use_candidate_checked(&mut self, i: usize) -> Result<UseLengths, Option<String>> {
        let c = self
            .candidates()
            .as_ref()
            .ok()
            .and_then(|found| found.best.get(i).copied())
            .ok_or(None)?;
        let (lu, lg) = check_lengths(c.lu as f64, c.lg as f64).map_err(Some)?;
        Ok(UseLengths {
            fd: Some(c.fd),
            lu,
            lg,
        })
    }

    /// `Use in OFDM` of K row `i`.
    pub(crate) fn use_k(&mut self, i: usize) -> Option<UseFraction> {
        let f = self.k_list().as_ref().ok()?.get(i)?.fraction;
        (f.q > 0).then_some(UseFraction { p: f.p, q: f.q })
    }
}

static EMPTY_CANDIDATES: Result<Ranked<CpCandidate>, String> = Err(String::new());
static EMPTY_K: Result<Vec<KCandidate>, String> = Err(String::new());

/// The calculators' form. `fraction_error` is the generator's refusal of the
/// last K picked; `selected` the selected row of each table.
pub(crate) fn rows(
    b: &mut Builder<'_>,
    c: &mut CalcInputs,
    errors: &BTreeMap<Num, String>,
    fraction_error: Option<&str>,
    selected: (Option<usize>, Option<usize>),
) {
    cp_rows(b, c, errors);
    inverse_rows(b, c, errors);
    candidate_rows(b, c, errors, selected.0);
    k_rows(b, c, errors, selected.1);
    if let Some(e) = fraction_error {
        b.end_group();
        b.wrapped(e, Tone::Error);
    }
}

/// The refusals of the calculator fields `fields` (bad text), in red under
/// their group's inputs: the results below still show the last valid value.
fn field_errors(b: &mut Builder<'_>, errors: &BTreeMap<Num, String>, fields: &[CalcNum]) {
    for f in fields {
        if let Some(e) = errors.get(&Num::Calc(*f)) {
            b.wrapped(e, Tone::Error);
        }
    }
}

fn value_row(b: &mut Builder<'_>, pairs: &[(&str, String)]) {
    let r = b.aligned();
    for (k, v) in pairs {
        r.text(k, Tone::Plain).text(v, Tone::Bold);
    }
}

fn cp_rows(b: &mut Builder<'_>, c: &CalcInputs, errors: &BTreeMap<Num, String>) {
    b.group(&t!("gen-calc-cp"));
    b.line().text(&t!("gen-calc-cp-lengths"), Tone::Dim);
    b.row("Fd").num(Num::Calc(CalcNum::Fd));
    b.row("LU")
        .num(Num::Calc(CalcNum::Lu))
        .text("LG", Tone::Plain)
        .num(Num::Calc(CalcNum::Lg));
    field_errors(b, errors, &[CalcNum::Fd, CalcNum::Lu, CalcNum::Lg]);
    match calc::cp_from_lengths(c.fd, c.lu, c.lg) {
        Ok(r) => {
            value_row(
                b,
                &[
                    ("LS", trim(r.ls, 4)),
                    ("K", trim(r.k, 6)),
                    ("Br", br(r.symbol_rate)),
                ],
            );
            value_row(
                b,
                &[
                    (&t!("gen-calc-spacing"), hz(r.spacing)),
                    (&t!("gen-calc-symbol"), ms(r.symbol_duration)),
                ],
            );
            value_row(
                b,
                &[
                    (&t!("gen-calc-useful"), ms(r.useful_duration)),
                    (&t!("gen-calc-guard"), ms(r.guard_duration)),
                ],
            );
        }
        Err(e) => b.wrapped(&e.to_string(), Tone::Error),
    }
    let usable = c.use_cp_checked();
    b.aligned()
        .when(usable.is_ok())
        .button(Btn::UseCp, &t!("gen-calc-use"))
        .hint(&t!("gen-calc-use.hover"));
    if let Err(Some(why)) = usable {
        b.wrapped(&why, Tone::Dim);
    }
}

fn inverse_rows(b: &mut Builder<'_>, c: &CalcInputs, errors: &BTreeMap<Num, String>) {
    b.group(&t!("gen-calc-inverse"));
    b.row("Br").num(Num::Calc(CalcNum::InvBr));
    b.row(&t!("gen-calc-spacing"))
        .num(Num::Calc(CalcNum::InvSpacing));
    b.row("Fd").num(Num::Calc(CalcNum::InvFd));
    field_errors(
        b,
        errors,
        &[CalcNum::InvBr, CalcNum::InvSpacing, CalcNum::InvFd],
    );
    match calc::cp_from_rates(c.inv_br, c.inv_spacing, c.inv_fd) {
        Ok(r) => {
            let exact = |n: u64, v: f64, err: f64| {
                format!(
                    "{n} ({} {}, {} {})",
                    t!("gen-calc-exact"),
                    trim(v, 4),
                    t!("gen-calc-error"),
                    trim(err, 4)
                )
            };
            value_row(b, &[("K", trim(r.k, 6))]);
            let lu = exact(r.lu, r.lu_exact, r.lu_error);
            let lg = exact(r.lg, r.lg_exact, r.lg_error);
            value_row(b, &[("LU", lu)]);
            value_row(b, &[("LG", lg)]);
            value_row(
                b,
                &[
                    ("Br", br(r.rounded.symbol_rate)),
                    (&t!("gen-calc-spacing"), hz(r.rounded.spacing)),
                ],
            );
        }
        Err(e) => b.wrapped(&e.to_string(), Tone::Error),
    }
    let usable = c.use_inverse_checked();
    b.aligned()
        .when(usable.is_ok())
        .button(Btn::UseInverse, &t!("gen-calc-use"))
        .hint(&t!("gen-calc-use.hover"));
    if let Err(Some(why)) = usable {
        b.wrapped(&why, Tone::Dim);
    }
}

fn candidate_rows(
    b: &mut Builder<'_>,
    c: &mut CalcInputs,
    errors: &BTreeMap<Num, String>,
    selected: Option<usize>,
) {
    b.group(&t!("gen-calc-candidates"));
    b.row("Br").num(Num::Calc(CalcNum::CandBr));
    b.row("K").num(Num::Calc(CalcNum::CandK));
    b.row(&t!("gen-calc-lu-range"))
        .num_w(Num::Calc(CalcNum::CandLuMin), 70)
        .text("-", Tone::Plain)
        .num_w(Num::Calc(CalcNum::CandLuMax), 70);
    b.row(&t!("gen-calc-tolerance"))
        .num(Num::Calc(CalcNum::CandTol));
    field_errors(
        b,
        errors,
        &[
            CalcNum::CandBr,
            CalcNum::CandK,
            CalcNum::CandLuMin,
            CalcNum::CandLuMax,
            CalcNum::CandTol,
        ],
    );
    match c.candidates() {
        Ok(found) => {
            let shown = found.best.len().min(TABLE_ROWS);
            let msg = t!("gen-calc-found", count = found.total, shown = shown);
            b.line().text(&msg, Tone::Plain);
            b.line().table(TableId::Candidates, TABLE_SHOWN);
        }
        Err(e) => {
            let e = e.clone();
            b.wrapped(&e, Tone::Error);
        }
    }
    let usable = selected.map_or(Err(None), |i| c.use_candidate_checked(i));
    b.line()
        .when(usable.is_ok())
        .button(Btn::UseCandidate, &t!("gen-calc-use"))
        .hint(&t!("gen-calc-use.hover"));
    if let Err(Some(why)) = usable {
        b.wrapped(&why, Tone::Dim);
    }
}

fn k_rows(
    b: &mut Builder<'_>,
    c: &mut CalcInputs,
    errors: &BTreeMap<Num, String>,
    selected: Option<usize>,
) {
    b.group(&t!("gen-calc-klist"));
    b.line()
        .radio(Pick::KFrom, 0, &t!("gen-calc-k-from-rates"))
        .radio(Pick::KFrom, 1, &t!("gen-calc-k-direct"));
    if c.k_from_rates {
        b.row("Br").num(Num::Calc(CalcNum::KBr));
        b.row(&t!("gen-calc-spacing"))
            .num(Num::Calc(CalcNum::KSpacing));
    } else {
        b.row("K").num(Num::Calc(CalcNum::KValue));
    }
    b.row(&t!("gen-calc-uncertainty"))
        .num(Num::Calc(CalcNum::KPct))
        .hint(&t!("gen-calc-uncertainty.hover"));
    b.row(&t!("gen-calc-max-den"))
        .num(Num::Calc(CalcNum::KMaxDen));
    b.row(&t!("gen-calc-count")).num(Num::Calc(CalcNum::KCount));
    field_errors(
        b,
        errors,
        &[
            CalcNum::KBr,
            CalcNum::KSpacing,
            CalcNum::KValue,
            CalcNum::KPct,
            CalcNum::KMaxDen,
            CalcNum::KCount,
        ],
    );
    match c.k_list() {
        Ok(list) if list.is_empty() => b.wrapped(&t!("gen-calc-k-none"), Tone::Plain),
        Ok(_) => {
            b.line().table(TableId::KList, TABLE_SHOWN);
        }
        Err(e) => {
            let e = e.clone();
            b.wrapped(&e, Tone::Error);
        }
    }
    let ok = selected.and_then(|i| c.use_k(i)).is_some();
    b.line()
        .when(ok)
        .button(Btn::UseK, &t!("gen-calc-use-k"))
        .hint(&t!("gen-calc-use-k.hover"));
}

fn br(v: f64) -> String {
    format!("{} Bd", trim(v, 4))
}

fn hz(v: f64) -> String {
    format!("{} Hz", trim(v, 4))
}

fn ms(seconds: f64) -> String {
    format!("{} ms", trim(seconds * 1e3, 4))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_defaults_give_ocg_test_signal_and_rows() {
        let mut c = CalcInputs::default();
        let u = c.use_cp().unwrap();
        assert_eq!((u.lu, u.lg, u.fd), (64, 20, Some(7000.0)));
        assert!(c.use_inverse().is_some());
        let (_, rows) = c.table_rows(TableId::Candidates);
        assert_ne!(rows.len(), 0);
        assert!(c.use_candidate(0).is_some());
        let (head, rows) = c.table_rows(TableId::KList);
        assert!(head.starts_with("K\t"));
        assert_ne!(rows.len(), 0);
        assert!(c.use_k(0).is_some());
        assert!(c.use_k(10_000).is_none());
        c.lu = 64.5;
        assert!(c.use_cp().is_none(), "LU must be whole");
        assert_eq!(
            c.use_cp_checked().unwrap_err(),
            Some(t!("gen-calc-lu-not-whole"))
        );
    }
}
