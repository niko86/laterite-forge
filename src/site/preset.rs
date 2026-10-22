//! Preset format v1: the site's story, loaded and checked before anything
//! generates from it.
//!
//! The loader walks the parsed TOML by hand instead of deriving `Deserialize`,
//! so one pass reports every problem and each names its field. Unknown keys are
//! refused so a typo cannot silently drop a field.

use std::fmt;
use std::path::Path;

use laterite_ags4_validator::{DictVersion, Dictionary};
use toml::Table;
use toml::Value;

/// The only schema this forge reads.
const SCHEMA: i64 = 1;

/// Presets shipped inside the binary as `(name, toml)`.
pub(crate) const SHIPPED: &[(&str, &str)] = &[(
    "london-basin",
    include_str!("../../data/presets/london-basin.toml"),
)];

#[derive(Debug, Clone, PartialEq)]
pub struct Preset {
    pub name: String,
    pub version: u32,
    pub summary: String,
    pub site: Site,
    pub hole_types: Vec<HoleType>,
    /// Top-down order.
    pub units: Vec<Unit>,
    /// Empty when the preset has none.
    pub lenses: Vec<Lens>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Site {
    pub extent_m: Extent,
    pub ground_level_m: Range,
}

/// Size of the local grid, whose origin is (0, 0).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Extent {
    pub x: f64,
    pub y: f64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct HoleType {
    /// A `LOCA_TYPE` code.
    pub code: String,
    /// Relative frequency.
    pub weight: u32,
    pub final_depth_m: Range,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Unit {
    pub id: String,
    pub name: String,
    /// `None` only on the last unit, which runs to final depth.
    pub thickness_m: Option<Range>,
    pub description: DescriptionSpec,
    pub sampling: Sampling,
}

/// A unit with a limited, lenticular footprint inside one host unit. In plan
/// it is a disc of radius R; its thickness is `t_max` at the centre and tapers
/// to nothing at the edge. Each site draws the centre, R, `t_max` and depth
/// fraction once from the ranges here.
#[derive(Debug, Clone, PartialEq)]
pub struct Lens {
    /// Unique across units and lenses.
    pub id: String,
    pub name: String,
    /// The id of a unit (never a lens).
    pub host: String,
    /// Plan radius R, metres; min > 0.
    pub radius_m: Range,
    /// Thickness at the centre, metres; min > 0.
    pub thickness_m: Range,
    /// Mid-depth of the lens as a fraction of the host's thickness at the
    /// hole, within `[0, 1]`.
    pub depth_in_host: Range,
    pub description: DescriptionSpec,
    pub sampling: Sampling,
}

/// Constraints for the unit's description. The site layer checks the terms
/// against the BS 5930 vocabulary, not the loader.
#[derive(Debug, Clone, PartialEq)]
pub struct DescriptionSpec {
    pub principals: Vec<String>,
    pub secondaries: Vec<String>,
    pub colours: Vec<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Sampling {
    /// `SAMP_TYPE` codes.
    pub types: Vec<String>,
    pub interval_m: Range,
}

/// A cited range: `basis` is the source of `min` and `max`.
#[derive(Debug, Clone, PartialEq)]
pub struct Range {
    pub min: f64,
    pub max: f64,
    pub basis: String,
}

/// Every problem found in a preset, one line each, naming where it is.
#[derive(Debug, Clone, PartialEq)]
pub struct PresetErrors(pub Vec<String>);

impl fmt::Display for PresetErrors {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for (i, e) in self.0.iter().enumerate() {
            if i > 0 {
                writeln!(f)?;
            }
            write!(f, "{e}")?;
        }
        Ok(())
    }
}

impl std::error::Error for PresetErrors {}

impl PresetErrors {
    fn one(msg: String) -> Self {
        Self(vec![msg])
    }
}

/// Resolve `arg` as a shipped preset name, else as a path to a TOML file.
pub fn resolve(arg: &str) -> Result<Preset, PresetErrors> {
    if let Some((_, text)) = SHIPPED.iter().find(|(name, _)| *name == arg) {
        return Preset::from_toml(text);
    }
    let path = Path::new(arg);
    if path.is_file() {
        let text = std::fs::read_to_string(path)
            .map_err(|e| PresetErrors::one(format!("{arg}: cannot read preset: {e}")))?;
        return Preset::from_toml(&text);
    }
    let names: Vec<&str> = SHIPPED.iter().map(|(n, _)| *n).collect();
    let shipped = if names.is_empty() {
        "none".to_string()
    } else {
        names.join(", ")
    };
    Err(PresetErrors::one(format!(
        "{arg}: not a shipped preset or a file (shipped presets: {shipped})"
    )))
}

impl Preset {
    pub fn from_toml(text: &str) -> Result<Preset, PresetErrors> {
        let root: Table = text
            .parse()
            .map_err(|e: toml::de::Error| PresetErrors::one(format!("toml: {e}")))?;
        let mut errs = Vec::new();
        let preset = read_preset(&root, &mut errs);
        match preset {
            Some(p) if errs.is_empty() => Ok(p),
            _ => Err(PresetErrors(errs)),
        }
    }
}

/// A table being read: tracks which keys were consumed so the rest can be
/// reported as unknown.
struct Node<'a> {
    table: &'a Table,
    path: String,
    seen: Vec<&'static str>,
}

impl<'a> Node<'a> {
    fn new(table: &'a Table, path: impl Into<String>) -> Self {
        Self {
            table,
            path: path.into(),
            seen: Vec::new(),
        }
    }

    fn at(&self, key: &str) -> String {
        if self.path.is_empty() {
            key.to_string()
        } else {
            format!("{}.{key}", self.path)
        }
    }

    fn get(&mut self, key: &'static str) -> Option<&'a Value> {
        self.seen.push(key);
        self.table.get(key)
    }

    fn finish(self, errs: &mut Vec<String>) {
        for k in self.table.keys() {
            if !self.seen.contains(&k.as_str()) {
                let at = if self.path.is_empty() {
                    "preset"
                } else {
                    &self.path
                };
                errs.push(format!("{at}: unknown key \"{k}\""));
            }
        }
    }
}

fn req_str(n: &mut Node, key: &'static str, errs: &mut Vec<String>) -> Option<String> {
    match n.get(key) {
        None => {
            errs.push(format!("{}: missing", n.at(key)));
            None
        }
        Some(Value::String(s)) => Some(s.clone()),
        Some(_) => {
            errs.push(format!("{}: must be a string", n.at(key)));
            None
        }
    }
}

fn req_int(n: &mut Node, key: &'static str, errs: &mut Vec<String>) -> Option<i64> {
    match n.get(key) {
        None => {
            errs.push(format!("{}: missing", n.at(key)));
            None
        }
        Some(Value::Integer(i)) => Some(*i),
        Some(_) => {
            errs.push(format!("{}: must be an integer", n.at(key)));
            None
        }
    }
}

/// A finite number; TOML integers are accepted where a float is wanted.
fn num(v: Option<&Value>, at: &str, errs: &mut Vec<String>) -> Option<f64> {
    let x = match v {
        None => {
            errs.push(format!("{at}: missing"));
            return None;
        }
        Some(Value::Float(f)) => *f,
        Some(Value::Integer(i)) => *i as f64,
        Some(_) => {
            errs.push(format!("{at}: must be a number"));
            return None;
        }
    };
    if x.is_finite() {
        Some(x)
    } else {
        errs.push(format!("{at}: must be finite"));
        None
    }
}

fn req_table<'a>(n: &mut Node<'a>, key: &'static str, errs: &mut Vec<String>) -> Option<Node<'a>> {
    match n.get(key) {
        None => {
            errs.push(format!("{}: missing", n.at(key)));
            None
        }
        Some(Value::Table(t)) => Some(Node::new(t, n.at(key))),
        Some(_) => {
            errs.push(format!("{}: must be a table", n.at(key)));
            None
        }
    }
}

fn str_list(n: &mut Node, key: &'static str, errs: &mut Vec<String>) -> Option<Vec<String>> {
    let at = n.at(key);
    match n.get(key) {
        None => {
            errs.push(format!("{at}: missing"));
            None
        }
        Some(Value::Array(a)) => {
            let mut out = Vec::new();
            for (i, v) in a.iter().enumerate() {
                if let Value::String(s) = v {
                    out.push(s.clone());
                } else {
                    errs.push(format!("{at}[{i}]: must be a string"));
                }
            }
            Some(out)
        }
        Some(_) => {
            errs.push(format!("{at}: must be an array of strings"));
            None
        }
    }
}

/// Reads `{ min, max, basis }` under `key`.
fn read_range(n: &mut Node, key: &'static str, errs: &mut Vec<String>) -> Option<Range> {
    let mut r = req_table(n, key, errs)?;
    let min = num(r.get("min"), &r.at("min"), errs);
    let max = num(r.get("max"), &r.at("max"), errs);
    let basis = match r.get("basis") {
        None => {
            errs.push(format!("{}: no basis", r.path));
            None
        }
        Some(Value::String(s)) if s.trim().is_empty() => {
            errs.push(format!("{}: empty basis", r.path));
            None
        }
        Some(Value::String(s)) => Some(s.clone()),
        Some(_) => {
            errs.push(format!("{}.basis: must be a string", r.path));
            None
        }
    };
    if let (Some(lo), Some(hi)) = (min, max)
        && lo > hi
    {
        errs.push(format!("{}: min {lo} is greater than max {hi}", r.path));
    }
    r.finish(errs);
    Some(Range {
        min: min?,
        max: max?,
        basis: basis?,
    })
}

fn read_preset(root: &Table, errs: &mut Vec<String>) -> Option<Preset> {
    let mut n = Node::new(root, "");

    match n.get("schema") {
        None => errs.push("schema: missing".to_string()),
        Some(Value::Integer(SCHEMA)) => {}
        Some(Value::Integer(i)) => errs.push(format!(
            "schema: {i} is not supported (this forge reads {SCHEMA})"
        )),
        Some(_) => errs.push("schema: must be an integer".to_string()),
    }

    let name = req_str(&mut n, "name", errs);
    if let Some(s) = &name
        && (s.is_empty()
            || !s
                .chars()
                .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-'))
    {
        errs.push(format!("name: \"{s}\" is not [a-z0-9-]+"));
    }
    let version = req_int(&mut n, "version", errs).and_then(|v| {
        if let Ok(v) = u32::try_from(v)
            && v >= 1
        {
            Some(v)
        } else {
            errs.push(format!("version: {v} must be an integer >= 1"));
            None
        }
    });
    let summary = req_str(&mut n, "summary", errs);

    let site = read_site(&mut n, errs);
    let hole_types = read_hole_types(&mut n, errs);
    let units = read_units(&mut n, errs);
    let lenses = read_lenses(&mut n, errs);
    n.finish(errs);

    Some(Preset {
        name: name?,
        version: version?,
        summary: summary?,
        site: site?,
        hole_types: hole_types?,
        units: units?,
        lenses: lenses?,
    })
}

fn read_site(n: &mut Node, errs: &mut Vec<String>) -> Option<Site> {
    let mut s = req_table(n, "site", errs)?;
    let extent = read_extent(&mut s, errs);
    let ground = read_range(&mut s, "ground_level_m", errs);
    s.finish(errs);
    let (x, y) = extent?;
    Some(Site {
        extent_m: Extent { x, y },
        ground_level_m: ground?,
    })
}

fn read_extent(s: &mut Node, errs: &mut Vec<String>) -> Option<(f64, f64)> {
    let mut e = req_table(s, "extent_m", errs)?;
    let x = num(e.get("x"), &e.at("x"), errs);
    let y = num(e.get("y"), &e.at("y"), errs);
    for (v, k) in [(x, "x"), (y, "y")] {
        if let Some(v) = v
            && v <= 0.0
        {
            errs.push(format!("{}: {v} must be > 0", e.at(k)));
        }
    }
    e.finish(errs);
    Some((x?, y?))
}

/// The tables of an array-of-tables key, each with its index and a node path
/// like `units[1]`.
fn read_array<'a>(
    n: &mut Node<'a>,
    key: &'static str,
    errs: &mut Vec<String>,
) -> Option<Vec<(Node<'a>, usize)>> {
    match n.get(key) {
        None => {
            errs.push(format!("{key}: missing"));
            None
        }
        Some(Value::Array(a)) => {
            let mut out = Vec::new();
            for (i, v) in a.iter().enumerate() {
                if let Value::Table(t) = v {
                    out.push((Node::new(t, format!("{key}[{i}]")), i));
                } else {
                    errs.push(format!("{key}[{i}]: must be a table"));
                }
            }
            Some(out)
        }
        Some(_) => {
            errs.push(format!("{key}: must be an array of tables"));
            None
        }
    }
}

fn read_hole_types(n: &mut Node, errs: &mut Vec<String>) -> Option<Vec<HoleType>> {
    let items = read_array(n, "hole_types", errs)?;
    if items.is_empty() {
        errs.push("hole_types: at least one is required".to_string());
    }
    let dict = Dictionary::bundled(DictVersion::V4_2);
    let loca = dict.abbr_codes("LOCA_TYPE");
    let mut out = Vec::new();
    for (mut h, _) in items {
        let code = req_str(&mut h, "code", errs);
        if let Some(c) = &code
            && !loca.contains(&c.as_str())
        {
            errs.push(format!("{}: \"{c}\" is not a LOCA_TYPE code", h.at("code")));
        }
        let weight = req_int(&mut h, "weight", errs).and_then(|w| {
            if let Ok(w) = u32::try_from(w)
                && w >= 1
            {
                Some(w)
            } else {
                errs.push(format!("{}: {w} must be an integer >= 1", h.at("weight")));
                None
            }
        });
        let depth = read_range(&mut h, "final_depth_m", errs);
        if let Some(d) = &depth
            && d.min <= 0.0
        {
            errs.push(format!(
                "{}: min {} must be > 0",
                h.at("final_depth_m"),
                d.min
            ));
        }
        h.finish(errs);
        if let (Some(code), Some(weight), Some(final_depth_m)) = (code, weight, depth) {
            out.push(HoleType {
                code,
                weight,
                final_depth_m,
            });
        }
    }
    Some(out)
}

fn read_units(n: &mut Node, errs: &mut Vec<String>) -> Option<Vec<Unit>> {
    let items = read_array(n, "units", errs)?;
    if items.is_empty() {
        errs.push("units: at least one is required".to_string());
    }
    let last = items.len().saturating_sub(1);
    let dict = Dictionary::bundled(DictVersion::V4_2);
    let samp = dict.abbr_codes("SAMP_TYPE");
    let mut ids: Vec<(String, usize)> = Vec::new();
    let mut out = Vec::new();
    for (mut u, i) in items {
        let id = req_str(&mut u, "id", errs);
        if let Some(id) = &id {
            if id.is_empty()
                || !id
                    .chars()
                    .all(|c| c.is_ascii_uppercase() || c.is_ascii_digit() || c == '_')
            {
                errs.push(format!("{}: \"{id}\" is not [A-Z0-9_]+", u.at("id")));
            }
            if let Some((_, first)) = ids.iter().find(|(k, _)| k == id) {
                errs.push(format!(
                    "{}: \"{id}\" is already used by units[{first}]",
                    u.at("id")
                ));
            } else {
                ids.push((id.clone(), i));
            }
            // Name the unit in later messages, as in `units[1] (LC)`.
            u.path = format!("{} ({id})", u.path);
        }
        let name = req_str(&mut u, "name", errs);

        let thickness = if u.table.contains_key("thickness_m") {
            if i == last {
                u.get("thickness_m");
                errs.push(format!(
                    "{}: forbidden on the last unit (it runs to final depth)",
                    u.at("thickness_m")
                ));
                None
            } else {
                read_range(&mut u, "thickness_m", errs)
            }
        } else {
            if i != last {
                errs.push(format!(
                    "{}: required on every unit but the last",
                    u.at("thickness_m")
                ));
            }
            u.get("thickness_m");
            None
        };

        let description = read_description(&mut u, errs);
        let sampling = read_sampling(&mut u, &samp, errs);
        u.finish(errs);
        if let (Some(id), Some(name), Some(description), Some(sampling)) =
            (id, name, description, sampling)
            && (i == last || thickness.is_some())
        {
            out.push(Unit {
                id,
                name,
                thickness_m: thickness,
                description,
                sampling,
            });
        }
    }
    Some(out)
}

/// The optional `[[lenses]]`: empty when absent. Ids share one namespace with
/// the units'.
fn read_lenses(n: &mut Node, errs: &mut Vec<String>) -> Option<Vec<Lens>> {
    if !n.table.contains_key("lenses") {
        n.get("lenses");
        return Some(Vec::new());
    }
    let items = read_array(n, "lenses", errs)?;
    // Ids as written, whether or not the entry itself loaded.
    let unit_ids: Vec<&str> = match n.table.get("units") {
        Some(Value::Array(a)) => a
            .iter()
            .filter_map(|u| u.as_table()?.get("id")?.as_str())
            .collect(),
        _ => Vec::new(),
    };
    let lens_ids: Vec<Option<&str>> = match n.table.get("lenses") {
        Some(Value::Array(a)) => a
            .iter()
            .map(|l| l.as_table().and_then(|t| t.get("id")?.as_str()))
            .collect(),
        _ => Vec::new(),
    };
    let dict = Dictionary::bundled(DictVersion::V4_2);
    let samp = dict.abbr_codes("SAMP_TYPE");
    let mut out = Vec::new();
    for (mut l, i) in items {
        let id = req_str(&mut l, "id", errs);
        if let Some(id) = &id {
            if id.is_empty()
                || !id
                    .chars()
                    .all(|c| c.is_ascii_uppercase() || c.is_ascii_digit() || c == '_')
            {
                errs.push(format!("{}: \"{id}\" is not [A-Z0-9_]+", l.at("id")));
            }
            let first_unit = unit_ids.iter().position(|u| u == id);
            let first_lens = lens_ids.iter().position(|x| *x == Some(id.as_str()));
            if let Some(u) = first_unit {
                errs.push(format!(
                    "{}: \"{id}\" is already used by units[{u}]",
                    l.at("id")
                ));
            } else if let Some(f) = first_lens
                && f < i
            {
                errs.push(format!(
                    "{}: \"{id}\" is already used by lenses[{f}]",
                    l.at("id")
                ));
            }
            l.path = format!("{} ({id})", l.path);
        }
        let name = req_str(&mut l, "name", errs);
        let host = req_str(&mut l, "host", errs);
        if let Some(h) = &host
            && !unit_ids.contains(&h.as_str())
        {
            if lens_ids.contains(&Some(h.as_str())) {
                errs.push(format!("{}: \"{h}\" is a lens, not a unit", l.at("host")));
            } else {
                errs.push(format!("{}: \"{h}\" is not a unit", l.at("host")));
            }
        }
        let radius = read_range(&mut l, "radius_m", errs);
        let thickness = read_range(&mut l, "thickness_m", errs);
        for (r, key) in [(&radius, "radius_m"), (&thickness, "thickness_m")] {
            if let Some(r) = r
                && r.min <= 0.0
            {
                errs.push(format!("{}: min {} must be > 0", l.at(key), r.min));
            }
        }
        let depth = read_range(&mut l, "depth_in_host", errs);
        if let Some(d) = &depth {
            if d.min < 0.0 {
                errs.push(format!(
                    "{}: min {} must be >= 0",
                    l.at("depth_in_host"),
                    d.min
                ));
            }
            if d.max > 1.0 {
                errs.push(format!(
                    "{}: max {} must be <= 1",
                    l.at("depth_in_host"),
                    d.max
                ));
            }
        }
        let description = read_description(&mut l, errs);
        let sampling = read_sampling(&mut l, &samp, errs);
        l.finish(errs);
        if let (
            Some(id),
            Some(name),
            Some(host),
            Some(radius_m),
            Some(thickness_m),
            Some(depth_in_host),
            Some(description),
            Some(sampling),
        ) = (
            id,
            name,
            host,
            radius,
            thickness,
            depth,
            description,
            sampling,
        ) {
            out.push(Lens {
                id,
                name,
                host,
                radius_m,
                thickness_m,
                depth_in_host,
                description,
                sampling,
            });
        }
    }
    Some(out)
}

fn read_description(u: &mut Node, errs: &mut Vec<String>) -> Option<DescriptionSpec> {
    let mut d = req_table(u, "description", errs)?;
    let principals = str_list(&mut d, "principals", errs);
    let secondaries = str_list(&mut d, "secondaries", errs);
    let colours = str_list(&mut d, "colours", errs);
    for (list, key) in [(&principals, "principals"), (&colours, "colours")] {
        if let Some(l) = list
            && l.is_empty()
        {
            errs.push(format!("{}: must not be empty", d.at(key)));
        }
    }
    d.finish(errs);
    Some(DescriptionSpec {
        principals: principals?,
        secondaries: secondaries?,
        colours: colours?,
    })
}

fn read_sampling(u: &mut Node, samp: &[&str], errs: &mut Vec<String>) -> Option<Sampling> {
    let mut s = req_table(u, "sampling", errs)?;
    let types = str_list(&mut s, "types", errs);
    if let Some(t) = &types {
        if t.is_empty() {
            errs.push(format!("{}: must not be empty", s.at("types")));
        }
        for (i, c) in t.iter().enumerate() {
            if !samp.contains(&c.as_str()) {
                errs.push(format!(
                    "{}[{i}]: \"{c}\" is not a SAMP_TYPE code",
                    s.at("types")
                ));
            }
        }
    }
    let interval = read_range(&mut s, "interval_m", errs);
    if let Some(r) = &interval
        && r.min <= 0.0
    {
        errs.push(format!("{}: min {} must be > 0", s.at("interval_m"), r.min));
    }
    s.finish(errs);
    Some(Sampling {
        types: types?,
        interval_m: interval?,
    })
}

#[cfg(test)]
#[allow(clippy::float_cmp)] // fixture floats are exact literals
mod tests {
    use super::*;

    /// Obviously fake: no real citation belongs in a test fixture.
    const VALID: &str = r#"
schema = 1
name = "test-basin"
version = 2
summary = "A fixture."

[site]
extent_m = { x = 400.0, y = 300 }
ground_level_m = { min = 8.0, max = 22.0, basis = "test fixture" }

[[hole_types]]
code = "CP"
weight = 3
final_depth_m = { min = 20.0, max = 40.0, basis = "test fixture" }

[[units]]
id = "RTD"
name = "River Terrace Deposits"
thickness_m = { min = -2.0, max = 6.0, basis = "test fixture" }
description = { principals = ["SAND", "GRAVEL"], secondaries = ["gravel"], colours = ["brown"] }
sampling = { types = ["B", "D"], interval_m = { min = 1.0, max = 1.5, basis = "test fixture" } }

[[units]]
id = "LC"
name = "London Clay"
description = { principals = ["CLAY"], secondaries = [], colours = ["grey"] }
sampling = { types = ["U"], interval_m = { min = 1.5, max = 3.0, basis = "test fixture" } }
"#;

    fn errors_of(text: &str) -> Vec<String> {
        Preset::from_toml(text).expect_err("should be refused").0
    }

    /// The fixture with one exact snippet replaced, so each test changes one rule.
    fn with(from: &str, to: &str) -> String {
        assert!(VALID.contains(from), "fixture lacks {from:?}");
        VALID.replacen(from, to, 1)
    }

    fn assert_refused(text: &str, expected: &str) {
        assert_eq!(errors_of(text), vec![expected.to_string()]);
    }

    #[test]
    fn a_valid_preset_loads_into_typed_values() {
        let p = Preset::from_toml(VALID).expect("valid");
        assert_eq!(p.name, "test-basin");
        assert_eq!(p.version, 2);
        assert_eq!(p.summary, "A fixture.");
        assert_eq!(p.site.extent_m.x, 400.0);
        assert_eq!(p.site.extent_m.y, 300.0);
        assert_eq!(
            p.site.ground_level_m,
            Range {
                min: 8.0,
                max: 22.0,
                basis: "test fixture".into()
            }
        );
        assert_eq!(p.hole_types.len(), 1);
        assert_eq!(p.hole_types[0].code, "CP");
        assert_eq!(p.hole_types[0].weight, 3);
        assert_eq!(p.hole_types[0].final_depth_m.max, 40.0);
        assert_eq!(p.units.len(), 2);
        assert_eq!(p.units[0].id, "RTD");
        assert_eq!(p.units[0].thickness_m.as_ref().unwrap().min, -2.0);
        assert_eq!(p.units[0].description.principals, ["SAND", "GRAVEL"]);
        assert_eq!(p.units[0].sampling.types, ["B", "D"]);
        assert_eq!(p.units[0].sampling.interval_m.max, 1.5);
        assert_eq!(p.units[1].id, "LC");
        assert_eq!(p.units[1].thickness_m, None);
        assert!(p.units[1].description.secondaries.is_empty());
    }

    const LENS: &str = r#"
[[lenses]]
id = "SB"
name = "Sand band"
host = "LC"
radius_m = { min = 30.0, max = 80.0, basis = "test fixture" }
thickness_m = { min = 0.3, max = 1.5, basis = "test fixture" }
depth_in_host = { min = 0.2, max = 0.8, basis = "test fixture" }
description = { principals = ["SAND"], secondaries = ["silt"], colours = ["brown"] }
sampling = { types = ["D"], interval_m = { min = 0.5, max = 1.0, basis = "test fixture" } }
"#;

    fn with_lens() -> String {
        format!("{VALID}{LENS}")
    }

    /// The lens fixture with one exact snippet replaced.
    fn lens_with(from: &str, to: &str) -> String {
        let text = with_lens();
        assert!(LENS.contains(from), "lens fixture lacks {from:?}");
        text.replacen(from, to, 1)
    }

    #[test]
    fn a_preset_without_lenses_has_none() {
        assert!(Preset::from_toml(VALID).unwrap().lenses.is_empty());
    }

    #[test]
    fn a_valid_lens_loads_into_typed_values() {
        let p = Preset::from_toml(&with_lens()).expect("valid");
        assert_eq!(p.lenses.len(), 1);
        let l = &p.lenses[0];
        assert_eq!(
            (l.id.as_str(), l.name.as_str(), l.host.as_str()),
            ("SB", "Sand band", "LC")
        );
        assert_eq!((l.radius_m.min, l.radius_m.max), (30.0, 80.0));
        assert_eq!((l.thickness_m.min, l.thickness_m.max), (0.3, 1.5));
        assert_eq!((l.depth_in_host.min, l.depth_in_host.max), (0.2, 0.8));
        assert_eq!(l.description.principals, ["SAND"]);
        assert_eq!(l.sampling.types, ["D"]);
    }

    #[test]
    fn a_lens_with_an_unknown_host_is_refused() {
        assert_refused(
            &lens_with("host = \"LC\"", "host = \"XX\""),
            "lenses[0] (SB).host: \"XX\" is not a unit",
        );
    }

    #[test]
    fn a_lens_cannot_host_a_lens() {
        let text = format!(
            "{}{}",
            with_lens(),
            LENS.replace("\"SB\"", "\"SB2\"")
                .replace("host = \"LC\"", "host = \"SB\"")
        );
        assert_refused(&text, "lenses[1] (SB2).host: \"SB\" is a lens, not a unit");
    }

    #[test]
    fn a_lens_id_must_be_unique_across_units_and_lenses() {
        assert_refused(
            &lens_with("id = \"SB\"", "id = \"RTD\""),
            "lenses[0].id: \"RTD\" is already used by units[0]",
        );
        let text = format!("{}{LENS}", with_lens());
        assert_refused(&text, "lenses[1].id: \"SB\" is already used by lenses[0]");
    }

    #[test]
    fn lens_ranges_outside_their_domain_are_refused() {
        assert_refused(
            &lens_with("radius_m = { min = 30.0", "radius_m = { min = 0.0"),
            "lenses[0] (SB).radius_m: min 0 must be > 0",
        );
        assert_refused(
            &lens_with("thickness_m = { min = 0.3", "thickness_m = { min = 0.0"),
            "lenses[0] (SB).thickness_m: min 0 must be > 0",
        );
        assert_refused(
            &lens_with(
                "depth_in_host = { min = 0.2",
                "depth_in_host = { min = -0.1",
            ),
            "lenses[0] (SB).depth_in_host: min -0.1 must be >= 0",
        );
        assert_refused(
            &lens_with("min = 0.2, max = 0.8", "min = 0.2, max = 1.2"),
            "lenses[0] (SB).depth_in_host: max 1.2 must be <= 1",
        );
    }

    #[test]
    fn a_lens_without_a_basis_or_with_an_unknown_key_is_refused() {
        assert_refused(
            &lens_with(
                ", basis = \"test fixture\" }\nthickness_m",
                " }\nthickness_m",
            ),
            "lenses[0] (SB).radius_m: no basis",
        );
        assert_refused(
            &lens_with("host = \"LC\"", "host = \"LC\"\nhosts = 1"),
            "lenses[0] (SB): unknown key \"hosts\"",
        );
        assert_refused(
            &lens_with("host = \"LC\"\n", ""),
            "lenses[0] (SB).host: missing",
        );
    }

    #[test]
    fn unknown_schema_is_refused() {
        assert_refused(
            &with("schema = 1", "schema = 2"),
            "schema: 2 is not supported (this forge reads 1)",
        );
    }

    #[test]
    fn missing_version_is_refused() {
        assert_refused(&with("version = 2\n", ""), "version: missing");
    }

    #[test]
    fn a_range_with_no_basis_is_refused() {
        assert_refused(
            &with(
                "thickness_m = { min = -2.0, max = 6.0, basis = \"test fixture\" }",
                "thickness_m = { min = -2.0, max = 6.0 }",
            ),
            "units[0] (RTD).thickness_m: no basis",
        );
    }

    #[test]
    fn an_empty_basis_is_refused() {
        assert_refused(
            &with(
                "ground_level_m = { min = 8.0, max = 22.0, basis = \"test fixture\" }",
                "ground_level_m = { min = 8.0, max = 22.0, basis = \"\" }",
            ),
            "site.ground_level_m: empty basis",
        );
    }

    #[test]
    fn a_range_with_min_above_max_is_refused() {
        assert_refused(
            &with(
                "final_depth_m = { min = 20.0, max = 40.0,",
                "final_depth_m = { min = 50.0, max = 40.0,",
            ),
            "hole_types[0].final_depth_m: min 50 is greater than max 40",
        );
    }

    #[test]
    fn a_duplicate_unit_id_is_refused() {
        assert_refused(
            &with("id = \"LC\"", "id = \"RTD\""),
            "units[1].id: \"RTD\" is already used by units[0]",
        );
    }

    #[test]
    fn thickness_on_the_last_unit_is_refused() {
        let text = VALID.replace(
            "description = { principals = [\"CLAY\"]",
            "thickness_m = { min = 1.0, max = 2.0, basis = \"test fixture\" }\ndescription = { principals = [\"CLAY\"]",
        );
        assert_refused(
            &text,
            "units[1] (LC).thickness_m: forbidden on the last unit (it runs to final depth)",
        );
    }

    #[test]
    fn missing_thickness_on_a_non_last_unit_is_refused() {
        assert_refused(
            &with(
                "thickness_m = { min = -2.0, max = 6.0, basis = \"test fixture\" }\n",
                "",
            ),
            "units[0] (RTD).thickness_m: required on every unit but the last",
        );
    }

    #[test]
    fn an_unknown_loca_type_is_refused() {
        assert_refused(
            &with("code = \"CP\"", "code = \"XX\""),
            "hole_types[0].code: \"XX\" is not a LOCA_TYPE code",
        );
    }

    #[test]
    fn an_unknown_samp_type_is_refused() {
        assert_refused(
            &with("types = [\"U\"]", "types = [\"U\", \"ZZZ\"]"),
            "units[1] (LC).sampling.types[1]: \"ZZZ\" is not a SAMP_TYPE code",
        );
    }

    #[test]
    fn an_unknown_key_is_refused() {
        assert_refused(
            &with("weight = 3", "weight = 3\nwieght = 4"),
            "hole_types[0]: unknown key \"wieght\"",
        );
        assert_refused(
            &with(
                "summary = \"A fixture.\"",
                "summary = \"A fixture.\"\nextra = 1",
            ),
            "preset: unknown key \"extra\"",
        );
    }

    #[test]
    fn empty_principals_and_colours_are_refused() {
        let text = with(
            "principals = [\"CLAY\"], secondaries = [], colours = [\"grey\"]",
            "principals = [], secondaries = [], colours = []",
        );
        assert_eq!(
            errors_of(&text),
            vec![
                "units[1] (LC).description.principals: must not be empty".to_string(),
                "units[1] (LC).description.colours: must not be empty".to_string(),
            ]
        );
    }

    #[test]
    fn non_positive_numbers_are_refused() {
        assert_refused(
            &with("x = 400.0", "x = 0.0"),
            "site.extent_m.x: 0 must be > 0",
        );
        assert_refused(
            &with("weight = 3", "weight = 0"),
            "hole_types[0].weight: 0 must be an integer >= 1",
        );
        assert_refused(
            &with("min = 20.0", "min = 0.0"),
            "hole_types[0].final_depth_m: min 0 must be > 0",
        );
        assert_refused(
            &with("interval_m = { min = 1.0", "interval_m = { min = 0.0"),
            "units[0] (RTD).sampling.interval_m: min 0 must be > 0",
        );
    }

    #[test]
    fn a_bad_name_or_unit_id_is_refused() {
        assert_refused(
            &with("name = \"test-basin\"", "name = \"Test Basin\""),
            "name: \"Test Basin\" is not [a-z0-9-]+",
        );
        assert_refused(
            &with("id = \"LC\"", "id = \"lc\""),
            "units[1].id: \"lc\" is not [A-Z0-9_]+",
        );
    }

    #[test]
    fn all_problems_are_reported_not_just_the_first() {
        let text = with("schema = 1", "schema = 3")
            .replace("code = \"CP\"", "code = \"XX\"")
            .replace("types = [\"U\"]", "types = [\"ZZZ\"]");
        assert_eq!(
            errors_of(&text),
            vec![
                "schema: 3 is not supported (this forge reads 1)".to_string(),
                "hole_types[0].code: \"XX\" is not a LOCA_TYPE code".to_string(),
                "units[1] (LC).sampling.types[0]: \"ZZZ\" is not a SAMP_TYPE code".to_string(),
            ]
        );
    }

    #[test]
    fn display_puts_each_problem_on_its_own_line() {
        let text = with("schema = 1", "schema = 3").replace("code = \"CP\"", "code = \"XX\"");
        let shown = Preset::from_toml(&text).unwrap_err().to_string();
        assert_eq!(
            shown,
            "schema: 3 is not supported (this forge reads 1)\n\
             hole_types[0].code: \"XX\" is not a LOCA_TYPE code"
        );
    }

    #[test]
    fn invalid_toml_is_refused() {
        let errs = errors_of("schema = ");
        assert_eq!(errs.len(), 1);
        assert!(errs[0].starts_with("toml: "));
    }

    #[test]
    fn resolve_reads_a_file_path() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("mine.toml");
        std::fs::write(&path, VALID).unwrap();
        let p = resolve(path.to_str().unwrap()).expect("loads");
        assert_eq!(p.name, "test-basin");
    }

    #[test]
    fn resolve_reports_file_problems() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("bad.toml");
        std::fs::write(&path, with("schema = 1", "schema = 9")).unwrap();
        let e = resolve(path.to_str().unwrap()).unwrap_err();
        assert_eq!(e.0, vec!["schema: 9 is not supported (this forge reads 1)"]);
    }

    #[test]
    fn resolve_of_an_unknown_name_lists_the_shipped_names() {
        let e = resolve("no-such-preset").unwrap_err();
        assert_eq!(e.0.len(), 1);
        assert!(
            e.0[0].starts_with("no-such-preset: not a shipped preset or a file (shipped presets: "),
            "{}",
            e.0[0]
        );
    }

    #[test]
    fn every_shipped_preset_loads() {
        for (name, text) in SHIPPED {
            if let Err(e) = Preset::from_toml(text) {
                panic!("shipped preset {name} is refused:\n{e}");
            }
            assert!(resolve(name).is_ok(), "{name} does not resolve by name");
        }
    }
}
