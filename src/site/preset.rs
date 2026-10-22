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

use super::properties::{self as props, Domain};

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

// Every field carries its unit as the TOML key does (`_m`).
#[allow(clippy::struct_field_names)]
#[derive(Debug, Clone, PartialEq)]
pub struct Site {
    pub extent_m: Extent,
    pub ground_level_m: Range,
    /// The water table's depth below ground level, metres.
    pub groundwater_depth_m: Range,
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
    /// Index properties (required).
    pub properties: Properties,
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
    /// Index properties (required).
    pub properties: Properties,
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

/// A cited range with an optional depth gradient: the value at depth `z`
/// below the unit's top is the field value in `range` plus `per_m` x `z`.
#[derive(Debug, Clone, PartialEq)]
pub struct Quantity {
    pub range: Range,
    /// Change per metre of depth; its own cited range.
    pub per_m: Option<Range>,
}

/// The index properties a unit or lens may carry (ADR 0001, "Values and
/// descriptions"). `fine` is required when any principal is CLAY or SILT, and
/// `coarse` when any is SAND or GRAVEL.
#[derive(Debug, Clone, PartialEq)]
pub struct Properties {
    pub fine: Option<FineProperties>,
    pub coarse: Option<CoarseProperties>,
    /// Particle density ratio Gs.
    pub particle_density_ratio: Quantity,
    /// Sr above the water table; 1 when absent.
    pub degree_of_saturation: Option<Quantity>,
    /// The chance, per hole, unit and quantity, of an outlier; a range so the
    /// rate may vary smoothly across the site. Default
    /// [`props::OUTLIER_RATE_DEFAULT`].
    pub outlier_rate: Option<Range>,
}

/// Drawn for CLAY and SILT units. Liquid limit, moisture content and the
/// consistency index are derived from these.
#[derive(Debug, Clone, PartialEq)]
pub struct FineProperties {
    pub plastic_limit_percent: Quantity,
    pub plasticity_index_percent: Quantity,
    pub liquidity_index: Quantity,
}

/// Drawn for SAND and GRAVEL units.
#[derive(Debug, Clone, PartialEq)]
pub struct CoarseProperties {
    pub spt_n: Quantity,
    pub moisture_content_percent: Quantity,
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
    let range = range_in(&mut r, errs);
    r.finish(errs);
    range
}

/// The `min`, `max` and `basis` of the table `r`; the caller finishes `r`.
fn range_in(r: &mut Node, errs: &mut Vec<String>) -> Option<Range> {
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
    Some(Range {
        min: min?,
        max: max?,
        basis: basis?,
    })
}

/// A range that must lie inside `domain`, each bound checked and named.
fn check_domain(r: &Range, at: &str, domain: &Domain, errs: &mut Vec<String>) {
    for (which, v) in [("min", r.min), ("max", r.max)] {
        if !domain.contains(v) {
            errs.push(format!(
                "{at}: {which} {v} is outside its domain ({})",
                domain.describe()
            ));
        }
    }
}

/// Reads `{ min, max, basis, per_m? }` under `key`, inside `domain`.
fn read_quantity(
    n: &mut Node,
    key: &'static str,
    domain: &Domain,
    errs: &mut Vec<String>,
) -> Option<Quantity> {
    let mut q = req_table(n, key, errs)?;
    let range = range_in(&mut q, errs);
    if let Some(r) = &range {
        check_domain(r, &q.path, domain, errs);
    }
    let per_m = if q.table.contains_key("per_m") {
        read_range(&mut q, "per_m", errs)
    } else {
        q.get("per_m");
        None
    };
    q.finish(errs);
    Some(Quantity {
        range: range?,
        per_m,
    })
}

/// A quantity of a property set. `unusable` is why the unit cannot use the
/// set, if it cannot: the key is then refused when written, and otherwise it
/// is required.
fn set_quantity(
    p: &mut Node,
    key: &'static str,
    domain: &Domain,
    unusable: Option<&str>,
    errs: &mut Vec<String>,
) -> Option<Quantity> {
    if let Some(reason) = unusable {
        if p.table.contains_key(key) {
            p.get(key);
            errs.push(format!("{}: {reason}, so it cannot use this", p.at(key)));
        }
        None
    } else {
        read_quantity(p, key, domain, errs)
    }
}

/// The `properties` table of a unit or lens whose principals are
/// `principals`. `None` when it has problems (which are then in `errs`).
fn read_properties(
    u: &mut Node,
    principals: &[String],
    errs: &mut Vec<String>,
) -> Option<Properties> {
    let has = |names: [&str; 2]| principals.iter().any(|p| names.contains(&p.as_str()));
    let (fine, coarse) = (has(["CLAY", "SILT"]), has(["SAND", "GRAVEL"]));
    let mut p = req_table(u, "properties", errs)?;
    if !fine && !coarse {
        errs.push(format!(
            "{}: no principal is fine (CLAY, SILT) or coarse (SAND, GRAVEL), so no property set applies",
            p.path
        ));
        return None;
    }
    let no_fine = (!fine).then_some("the unit has no fine principal (CLAY, SILT)");
    let no_coarse = (!coarse).then_some("the unit has no coarse principal (SAND, GRAVEL)");
    let pl = set_quantity(
        &mut p,
        "plastic_limit_percent",
        &props::PLASTIC_LIMIT_PERCENT,
        no_fine,
        errs,
    );
    let pi = set_quantity(
        &mut p,
        "plasticity_index_percent",
        &props::PLASTICITY_INDEX_PERCENT,
        no_fine,
        errs,
    );
    let li = set_quantity(
        &mut p,
        "liquidity_index",
        &props::LIQUIDITY_INDEX,
        no_fine,
        errs,
    );
    let spt = set_quantity(&mut p, "spt_n", &props::SPT_N, no_coarse, errs);
    let w = set_quantity(
        &mut p,
        "moisture_content_percent",
        &props::MOISTURE_CONTENT_PERCENT,
        no_coarse,
        errs,
    );
    let gs = read_quantity(
        &mut p,
        "particle_density_ratio",
        &props::PARTICLE_DENSITY_RATIO,
        errs,
    );
    let sr = if p.table.contains_key("degree_of_saturation") {
        read_quantity(
            &mut p,
            "degree_of_saturation",
            &props::DEGREE_OF_SATURATION,
            errs,
        )
    } else {
        p.get("degree_of_saturation");
        None
    };
    let rate = if p.table.contains_key("outlier_rate") {
        let rate = read_range(&mut p, "outlier_rate", errs);
        if let Some(r) = &rate {
            check_domain(r, &p.at("outlier_rate"), &props::OUTLIER_RATE, errs);
        }
        rate
    } else {
        p.get("outlier_rate");
        None
    };
    p.finish(errs);
    Some(Properties {
        fine: if fine {
            Some(FineProperties {
                plastic_limit_percent: pl?,
                plasticity_index_percent: pi?,
                liquidity_index: li?,
            })
        } else {
            None
        },
        coarse: if coarse {
            Some(CoarseProperties {
                spt_n: spt?,
                moisture_content_percent: w?,
            })
        } else {
            None
        },
        particle_density_ratio: gs?,
        degree_of_saturation: sr,
        outlier_rate: rate,
    })
}

/// The unit or lens's `properties`, which are required. They depend on the
/// principals, so a description that failed to load leaves them unchecked.
fn required_properties(
    n: &mut Node,
    description: Option<&DescriptionSpec>,
    errs: &mut Vec<String>,
) -> Option<Properties> {
    // An empty principal list is already reported; it would only cascade.
    if let Some(d) = description
        && !d.principals.is_empty()
    {
        read_properties(n, &d.principals, errs)
    } else {
        n.get("properties");
        None
    }
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
    let groundwater = read_range(&mut s, "groundwater_depth_m", errs);
    if let Some(r) = &groundwater {
        check_domain(
            r,
            &s.at("groundwater_depth_m"),
            &props::GROUNDWATER_DEPTH_M,
            errs,
        );
    }
    s.finish(errs);
    let (x, y) = extent?;
    Some(Site {
        extent_m: Extent { x, y },
        ground_level_m: ground?,
        groundwater_depth_m: groundwater?,
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
        let properties = required_properties(&mut u, description.as_ref(), errs);
        u.finish(errs);
        if let (Some(id), Some(name), Some(description), Some(sampling), Some(properties)) =
            (id, name, description, sampling, properties)
            && (i == last || thickness.is_some())
        {
            out.push(Unit {
                id,
                name,
                thickness_m: thickness,
                description,
                sampling,
                properties,
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
        let properties = required_properties(&mut l, description.as_ref(), errs);
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
            Some(properties),
        ) = (
            id,
            name,
            host,
            radius,
            thickness,
            depth,
            description,
            sampling,
            properties,
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
                properties,
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

    /// Obviously fake: no real citation belongs in a test fixture. Without
    /// properties or groundwater, which the loader requires; [`valid`] adds
    /// them.
    const BASE: &str = r#"
schema = 1
name = "test-basin"
version = 2
summary = "A fixture."

[site]
extent_m = { x = 400.0, y = 300 }
ground_level_m = { min = 8.0, max = 22.0, basis = "test fixture" }
groundwater_depth_m = { min = 1.0, max = 4.0, basis = "test fixture" }

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

    /// The complete fixture: [`BASE`] with a properties block on each unit.
    fn valid() -> String {
        with_props(RTD_PROPS, LC_PROPS)
    }

    /// The fixture with one exact snippet replaced, so each test changes one rule.
    fn with(from: &str, to: &str) -> String {
        let valid = valid();
        assert!(valid.contains(from), "fixture lacks {from:?}");
        valid.replacen(from, to, 1)
    }

    fn assert_refused(text: &str, expected: &str) {
        assert_eq!(errors_of(text), vec![expected.to_string()]);
    }

    #[test]
    fn a_valid_preset_loads_into_typed_values() {
        let p = Preset::from_toml(&valid()).expect("valid");
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

[lenses.properties]
spt_n = { min = 5, max = 20, basis = "test fixture" }
moisture_content_percent = { min = 10, max = 20, basis = "test fixture" }
particle_density_ratio = { min = 2.6, max = 2.7, basis = "test fixture" }
"#;

    fn with_lens() -> String {
        format!("{}{LENS}", valid())
    }

    /// The lens fixture with one exact snippet replaced.
    fn lens_with(from: &str, to: &str) -> String {
        let text = with_lens();
        assert!(LENS.contains(from), "lens fixture lacks {from:?}");
        text.replacen(from, to, 1)
    }

    #[test]
    fn a_preset_without_lenses_has_none() {
        assert!(Preset::from_toml(&valid()).unwrap().lenses.is_empty());
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
        let text = valid().replace(
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
        std::fs::write(&path, valid()).unwrap();
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

    const RTD_PROPS: &str = r#"
[units.properties]
spt_n = { min = 8, max = 40, basis = "test fixture", per_m = { min = 0.5, max = 1.5, basis = "test fixture" } }
moisture_content_percent = { min = 8, max = 20, basis = "test fixture" }
particle_density_ratio = { min = 2.6, max = 2.7, basis = "test fixture" }
degree_of_saturation = { min = 0.6, max = 0.9, basis = "test fixture" }
"#;

    const LC_PROPS: &str = r#"
[units.properties]
plastic_limit_percent = { min = 20, max = 30, basis = "test fixture" }
plasticity_index_percent = { min = 30, max = 50, basis = "test fixture" }
liquidity_index = { min = -0.1, max = 0.5, basis = "test fixture" }
particle_density_ratio = { min = 2.65, max = 2.75, basis = "test fixture" }
outlier_rate = { min = 0.0, max = 0.05, basis = "test fixture" }
"#;

    /// The fixture with a properties block on each unit.
    fn with_props(rtd: &str, lc: &str) -> String {
        let text = BASE.replacen(
            "\n[[units]]\nid = \"LC\"",
            &format!("{rtd}\n[[units]]\nid = \"LC\""),
            1,
        );
        format!("{text}{lc}")
    }

    /// The valid property blocks with one exact snippet replaced.
    fn props_with(from: &str, to: &str) -> String {
        let both = format!("{RTD_PROPS}{LC_PROPS}");
        assert!(both.contains(from), "property fixtures lack {from:?}");
        let (rtd, lc) = (
            RTD_PROPS.replacen(from, to, 1),
            LC_PROPS.replacen(from, to, 1),
        );
        with_props(&rtd, &lc)
    }

    #[test]
    fn a_valid_properties_block_loads() {
        let p = Preset::from_toml(&valid()).expect("valid");
        let coarse = &p.units[0].properties;
        assert!(coarse.fine.is_none());
        let c = coarse.coarse.as_ref().unwrap();
        assert_eq!((c.spt_n.range.min, c.spt_n.range.max), (8.0, 40.0));
        assert_eq!(c.spt_n.per_m.as_ref().unwrap().max, 1.5);
        assert_eq!(coarse.degree_of_saturation.as_ref().unwrap().range.min, 0.6);
        assert_eq!(coarse.outlier_rate, None);
        let fine = &p.units[1].properties;
        assert!(fine.coarse.is_none());
        let f = fine.fine.as_ref().unwrap();
        assert_eq!(f.liquidity_index.range.min, -0.1);
        assert_eq!(f.plastic_limit_percent.per_m, None);
        assert_eq!(fine.outlier_rate.as_ref().unwrap().max, 0.05);
        assert_eq!(fine.degree_of_saturation, None);
    }

    #[test]
    fn properties_are_required_on_every_unit() {
        let text = BASE.replacen(
            "\n[[units]]\nid = \"LC\"",
            &format!("{RTD_PROPS}\n[[units]]\nid = \"LC\""),
            1,
        );
        assert_refused(&text, "units[1] (LC).properties: missing");
        assert_refused(
            &with_props("", LC_PROPS),
            "units[0] (RTD).properties: missing",
        );
        let both_missing = errors_of(BASE);
        assert!(both_missing.contains(&"units[0] (RTD).properties: missing".to_string()));
        assert!(both_missing.contains(&"units[1] (LC).properties: missing".to_string()));
    }

    #[test]
    fn properties_are_required_on_a_lens() {
        let text = with_lens().replacen("[lenses.properties]", "[lenses.unrelated]", 1);
        let errs = errors_of(&text);
        assert!(
            errs.contains(&"lenses[0] (SB).properties: missing".to_string()),
            "{errs:?}"
        );
    }

    #[test]
    fn groundwater_depth_is_required_on_the_site() {
        let text = with(
            "groundwater_depth_m = { min = 1.0, max = 4.0, basis = \"test fixture\" }\n",
            "",
        );
        assert_refused(&text, "site.groundwater_depth_m: missing");
    }

    #[test]
    fn the_site_carries_a_groundwater_depth() {
        let g = Preset::from_toml(&valid())
            .unwrap()
            .site
            .groundwater_depth_m;
        assert_eq!((g.min, g.max), (1.0, 4.0));
    }

    #[test]
    fn a_lens_carries_properties() {
        let p = Preset::from_toml(&with_lens()).expect("valid");
        assert!(p.lenses[0].properties.coarse.is_some());
    }

    #[test]
    fn the_groundwater_depth_may_not_be_negative() {
        let text = with(
            "groundwater_depth_m = { min = 1.0,",
            "groundwater_depth_m = { min = -1.0,",
        );
        assert_refused(
            &text,
            "site.groundwater_depth_m: min -1 is outside its domain (>= 0)",
        );
    }

    #[test]
    fn a_coarse_unit_must_give_the_coarse_set() {
        let text = with_props(&RTD_PROPS.replace("spt_n = {", "spt_x = {"), LC_PROPS);
        let errs = errors_of(&text);
        assert_eq!(
            errs,
            vec![
                "units[0] (RTD).properties.spt_n: missing".to_string(),
                "units[0] (RTD).properties: unknown key \"spt_x\"".to_string(),
            ]
        );
    }

    #[test]
    fn a_fine_unit_must_give_the_fine_set() {
        let text = with_props(
            RTD_PROPS,
            &LC_PROPS.replace("liquidity_index", "liquidity_ndx"),
        );
        let errs = errors_of(&text);
        assert!(errs.contains(&"units[1] (LC).properties.liquidity_index: missing".to_string()));
    }

    #[test]
    fn particle_density_is_required() {
        let text = with_props(
            RTD_PROPS,
            &LC_PROPS.replace("particle_density_ratio", "particle_density"),
        );
        assert!(
            errors_of(&text)
                .contains(&"units[1] (LC).properties.particle_density_ratio: missing".to_string())
        );
    }

    #[test]
    fn a_property_the_unit_cannot_use_is_refused() {
        let fine_on_coarse = format!(
            "{RTD_PROPS}plastic_limit_percent = {{ min = 20, max = 30, basis = \"test fixture\" }}\n"
        );
        assert_refused(
            &with_props(&fine_on_coarse, LC_PROPS),
            "units[0] (RTD).properties.plastic_limit_percent: the unit has no fine principal (CLAY, SILT), so it cannot use this",
        );
        let coarse_on_fine =
            format!("{LC_PROPS}spt_n = {{ min = 20, max = 30, basis = \"test fixture\" }}\n");
        assert_refused(
            &with_props(RTD_PROPS, &coarse_on_fine),
            "units[1] (LC).properties.spt_n: the unit has no coarse principal (SAND, GRAVEL), so it cannot use this",
        );
    }

    #[test]
    fn a_unit_with_both_kinds_of_principal_needs_both_sets() {
        let text = with(
            "principals = [\"CLAY\"]",
            "principals = [\"CLAY\", \"SAND\"]",
        );
        assert_eq!(
            errors_of(&text),
            vec![
                "units[1] (LC).properties.spt_n: missing".to_string(),
                "units[1] (LC).properties.moisture_content_percent: missing".to_string(),
            ]
        );
    }

    #[test]
    fn properties_on_a_unit_with_no_fine_or_coarse_principal_are_refused() {
        let text = with("principals = [\"CLAY\"]", "principals = [\"PEAT\"]");
        assert_refused(
            &text,
            "units[1] (LC).properties: no principal is fine (CLAY, SILT) or coarse (SAND, GRAVEL), so no property set applies",
        );
    }

    #[test]
    fn an_unknown_property_key_is_refused() {
        let text = with_props(
            RTD_PROPS,
            &format!(
                "{LC_PROPS}undrained_strength = {{ min = 1, max = 2, basis = \"test fixture\" }}\n"
            ),
        );
        assert_refused(
            &text,
            "units[1] (LC).properties: unknown key \"undrained_strength\"",
        );
        let text = props_with(
            "min = 0.5, max = 1.5, basis = \"test fixture\" }",
            "min = 0.5, max = 1.5, basis = \"test fixture\", rate = 1 }",
        );
        assert_refused(
            &text,
            "units[0] (RTD).properties.spt_n.per_m: unknown key \"rate\"",
        );
    }

    #[test]
    fn a_property_without_a_basis_is_refused() {
        let text = with_props(
            &RTD_PROPS.replace(
                "moisture_content_percent = { min = 8, max = 20, basis = \"test fixture\" }",
                "moisture_content_percent = { min = 8, max = 20 }",
            ),
            LC_PROPS,
        );
        assert_refused(
            &text,
            "units[0] (RTD).properties.moisture_content_percent: no basis",
        );
        let text = props_with(
            "per_m = { min = 0.5, max = 1.5, basis = \"test fixture\" }",
            "per_m = { min = 0.5, max = 1.5 }",
        );
        assert_refused(&text, "units[0] (RTD).properties.spt_n.per_m: no basis");
    }

    #[test]
    fn ranges_outside_a_physical_domain_are_refused_by_name() {
        for (from, to, expected) in [
            (
                "plastic_limit_percent = { min = 20,",
                "plastic_limit_percent = { min = 0,",
                "units[1] (LC).properties.plastic_limit_percent: min 0 is outside its domain (> 0)",
            ),
            (
                "plasticity_index_percent = { min = 30, max = 50,",
                "plasticity_index_percent = { min = -5, max = 50,",
                "units[1] (LC).properties.plasticity_index_percent: min -5 is outside its domain (> 0)",
            ),
            (
                "liquidity_index = { min = -0.1, max = 0.5,",
                "liquidity_index = { min = -0.1, max = 3.5,",
                "units[1] (LC).properties.liquidity_index: max 3.5 is outside its domain (>= -1 and <= 3)",
            ),
            (
                "particle_density_ratio = { min = 2.65, max = 2.75,",
                "particle_density_ratio = { min = 0.9, max = 2.75,",
                "units[1] (LC).properties.particle_density_ratio: min 0.9 is outside its domain (> 1 and <= 4)",
            ),
            (
                "outlier_rate = { min = 0.0, max = 0.05,",
                "outlier_rate = { min = 0.0, max = 1.5,",
                "units[1] (LC).properties.outlier_rate: max 1.5 is outside its domain (>= 0 and <= 1)",
            ),
        ] {
            let text = with_props(RTD_PROPS, &LC_PROPS.replacen(from, to, 1));
            assert_refused(&text, expected);
        }
        for (from, to, expected) in [
            (
                "spt_n = { min = 8,",
                "spt_n = { min = -1,",
                "units[0] (RTD).properties.spt_n: min -1 is outside its domain (>= 0)",
            ),
            (
                "moisture_content_percent = { min = 8,",
                "moisture_content_percent = { min = -8,",
                "units[0] (RTD).properties.moisture_content_percent: min -8 is outside its domain (>= 0)",
            ),
            (
                "degree_of_saturation = { min = 0.6, max = 0.9,",
                "degree_of_saturation = { min = 0.6, max = 1.2,",
                "units[0] (RTD).properties.degree_of_saturation: max 1.2 is outside its domain (> 0 and <= 1)",
            ),
        ] {
            let text = with_props(&RTD_PROPS.replacen(from, to, 1), LC_PROPS);
            assert_refused(&text, expected);
        }
    }
}
