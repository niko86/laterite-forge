//! PROTOTYPE for #1101 — throwaway, never merge. Lives on `prototype/forge-results`.
//!
//! Question: what does a `scale --scaffold wide` file look like with result
//! headings filled in, and what does the per-heading range data cost to author?
//!
//! Switch: `FORGE_PROTO_RESULTS=1` (an env var, not a flag, so no CLI plumbing).
//! Off: this module is never called, so the default draw cannot move.
//! On: a post-pass over the finished groups, BEFORE ABBR/UNIT/TYPE are collected
//! (so new units are declared), drawing from a second, salted `Rng` — the
//! flagged file is the unflagged file plus columns.

use std::collections::HashMap;

use laterite_ags4_parity::Rng;
use laterite_ags4_validator::Dictionary;

use super::model::Group;

pub fn enabled() -> bool {
    std::env::var_os("FORGE_PROTO_RESULTS").is_some()
}

/// How one result value is made. `Range` is hand-authored data; the rest are
/// couplings between headings of the same row.
enum Gen {
    Range(f64, f64),
    /// LLPL_PI = LLPL_LL − LLPL_PL.
    Pi,
    /// dry density = bulk / (1 + mc/100), from the named bulk + mc headings.
    Dden(&'static str, &'static str),
    /// Cell pressure rising with specimen depth (a depth coupling, not strata).
    CellFromDepth,
}

/// The hand-authored part. Every line here is a judgement someone has to own.
const RESULTS: &[(&str, &str, Gen)] = &[
    ("LLPL", "LLPL_LL", Gen::Range(25.0, 80.0)),
    ("LLPL", "LLPL_PL", Gen::Range(14.0, 30.0)),
    ("LLPL", "LLPL_PI", Gen::Pi),
    ("LLPL", "LLPL_425", Gen::Range(60.0, 100.0)),
    ("LNMC", "LNMC_MC", Gen::Range(8.0, 45.0)),
    ("ISPT", "ISPT_NVAL", Gen::Range(4.0, 50.0)),
    ("LDEN", "LDEN_MC", Gen::Range(10.0, 35.0)),
    ("LDEN", "LDEN_BDEN", Gen::Range(1.70, 2.20)),
    ("LDEN", "LDEN_DDEN", Gen::Dden("LDEN_BDEN", "LDEN_MC")),
    ("TRET", "TRET_IMC", Gen::Range(12.0, 35.0)),
    ("TRET", "TRET_BDEN", Gen::Range(1.80, 2.20)),
    ("TRET", "TRET_DDEN", Gen::Dden("TRET_BDEN", "TRET_IMC")),
    ("TRET", "TRET_CELL", Gen::CellFromDepth),
    ("TRET", "TRET_DEVF", Gen::Range(40.0, 450.0)),
    ("TRET", "TRET_STRN", Gen::Range(2.0, 15.0)),
];

/// Format `v` the way the heading's AGS type wants it.
fn fmt(ags_type: &str, v: f64) -> String {
    match ags_type.strip_suffix("DP").and_then(|n| n.parse::<usize>().ok()) {
        Some(p) => format!("{v:.p$}"),
        None => format!("{v:.0}"), // 0DP is caught above; X / XN carry a plain number
    }
}

fn draw(rng: &mut Rng, lo: f64, hi: f64) -> f64 {
    lo + (hi - lo) * (rng.below(10_001) as f64 / 10_000.0)
}

pub fn add(groups: &mut [Group], dict: &Dictionary<'static>, seed: u64) {
    let mut rng = Rng::seeded(seed ^ 0x5245_5355_4c54_5321);
    for g in groups.iter_mut() {
        let wanted: Vec<&(&str, &str, Gen)> = RESULTS
            .iter()
            .filter(|(c, h, _)| *c == g.code && dict.heading(c, h).is_some() && g.col(h).is_none())
            .collect();
        if wanted.is_empty() {
            continue;
        }
        let code = g.code.clone();
        let depth_col = g.col("SPEC_DPTH");
        // Per row: heading -> value, computed in table order so couplings see their inputs.
        let mut added: Vec<HashMap<&str, String>> = Vec::with_capacity(g.rows.len());
        for r in &g.rows {
            let mut row: HashMap<&str, String> = HashMap::new();
            let num = |row: &HashMap<&str, String>, h: &str| row[h].parse::<f64>().unwrap();
            for (_, h, how) in &wanted {
                let t = dict.heading(&code, h).unwrap().ags_type;
                let v = match how {
                    Gen::Range(lo, hi) => draw(&mut rng, *lo, *hi),
                    Gen::Pi => num(&row, "LLPL_LL") - num(&row, "LLPL_PL"),
                    Gen::Dden(b, m) => num(&row, b) / (1.0 + num(&row, m) / 100.0),
                    Gen::CellFromDepth => {
                        let d: f64 = depth_col.map_or(5.0, |c| r.values[c].parse().unwrap_or(5.0));
                        (50.0 + 20.0 * d + draw(&mut rng, 0.0, 50.0)).round()
                    }
                };
                row.insert(h, fmt(t, v));
            }
            added.push(row);
        }
        // Re-lay the columns in dictionary order, existing + added.
        let order: Vec<&str> = dict
            .group_headings(&code)
            .iter()
            .copied()
            .filter(|h| g.col(h).is_some() || wanted.iter().any(|(_, w, _)| w == h))
            .collect();
        let old = g.clone();
        g.headings = order.iter().map(|h| (*h).to_string()).collect();
        g.units = order
            .iter()
            .map(|h| old.col(h).map_or_else(|| dict.heading(&code, h).unwrap().unit.to_string(), |c| old.units[c].clone()))
            .collect();
        g.types = order
            .iter()
            .map(|h| old.col(h).map_or_else(|| dict.heading(&code, h).unwrap().ags_type.to_string(), |c| old.types[c].clone()))
            .collect();
        for (r, extra) in g.rows.iter_mut().zip(&added) {
            let prev = r.values.clone();
            r.values = order
                .iter()
                .map(|h| old.col(h).map_or_else(|| extra[h].clone(), |c| prev[c].clone()))
                .collect();
        }
    }
}
