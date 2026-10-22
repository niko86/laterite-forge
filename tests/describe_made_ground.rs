//! `forge describe --made-ground`, through the real binary.

use std::process::{Command, Output};

fn describe(extra: &[&str]) -> serde_json::Value {
    let out: Output = Command::new(env!("CARGO_BIN_EXE_laterite-ags4-forge"))
        .args(["describe", "--count", "20", "--output", "json"])
        .args(extra)
        .output()
        .expect("the binary runs");
    assert!(out.status.success());
    serde_json::from_slice(&out.stdout).expect("stdout is JSON")
}

#[test]
fn the_flag_tags_every_row_and_exposes_the_constituents() {
    let v = describe(&["--made-ground"]);
    let rows = v["descriptions"].as_array().unwrap();
    assert_eq!(rows.len(), 20);
    for r in rows {
        assert!(r["text"].as_str().unwrap().starts_with("MADE GROUND: "));
        let k = r["constituents"].as_array().unwrap();
        assert!((1..=2).contains(&k.len()));
    }
}

#[test]
fn without_the_flag_there_are_no_made_ground_fields() {
    let v = describe(&[]);
    for r in v["descriptions"].as_array().unwrap() {
        assert!(!r["text"].as_str().unwrap().contains("MADE GROUND"));
        assert!(r.get("constituents").is_none());
    }
}

#[test]
fn the_matrix_is_the_unflagged_draw() {
    let plain = describe(&[]);
    let mg = describe(&["--made-ground"]);
    for (p, m) in plain["descriptions"]
        .as_array()
        .unwrap()
        .iter()
        .zip(mg["descriptions"].as_array().unwrap())
    {
        assert_eq!(p["seed"], m["seed"]);
        assert_eq!(p["principal"], m["principal"]);
        assert_eq!(p["fractions"], m["fractions"]);
    }
}
