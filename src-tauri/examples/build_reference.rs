//! Builds the bundled reference pack from the sheets in `Global/`.
//!
//! Run by the maintainer, not by CI, because `Global/` holds the real data and
//! is never committed:
//!
//! ```text
//! cargo run --example build_reference
//! ```
//!
//! The expensive, careful part of identity resolution happens here rather than
//! on every launch: names are normalised, matched through the ambiguity gate,
//! and written out as resolved links. The application then ships a table it can
//! use immediately, and a student never has to supply a reference sheet to get
//! an analysis.
//!
//! Only four fields survive: registration number, name, CGPA and branch. The
//! source sheet also carries personal email addresses, phone numbers, gender,
//! dates of birth and resume links; none of them are read, and the pack has
//! nowhere to put them.

use nankiv_core::engine;
use nankiv_core::model::{Confidence, Identifier};
use nankiv_core::parse::{self, academic::parse_academic_sheet};
use nankiv_core::store::Store;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("repository root")
        .to_path_buf();
    let global = root.join("Global");
    if !global.is_dir() {
        eprintln!("No Global/ directory — nothing to build from.");
        std::process::exit(1);
    }

    let store = Store::open_in_memory()?;
    let mut sources: Vec<String> = Vec::new();

    // Academic sheets first: they define which registration numbers matter.
    let mut academic_files: Vec<PathBuf> = Vec::new();
    let mut roster_files: Vec<PathBuf> = Vec::new();
    for entry in std::fs::read_dir(&global)? {
        let p = entry?.path();
        let ext = p.extension().and_then(|e| e.to_str()).unwrap_or("");
        if !matches!(ext, "xlsx" | "xls" | "csv") {
            continue;
        }
        let rows = parse_academic_sheet(&p).unwrap_or_default();
        if rows.iter().any(|r| r.cgpa.is_some()) {
            academic_files.push(p);
        } else {
            roster_files.push(p);
        }
    }

    for p in &academic_files {
        let name = p.file_name().unwrap().to_string_lossy().to_string();
        let rows = parse_academic_sheet(p)?;
        let n = engine::ingest_academics(&store, &rows, &name)?;
        println!("  academics  {name:44} {n:>5} records");
        sources.push(name);
    }

    for p in &roster_files {
        let name = p.file_name().unwrap().to_string_lossy().to_string();
        match parse::parse_file(p) {
            Ok(parsed) if parsed.shape.is_usable() => {
                let h = engine::harvest_identity(&store, &parsed, &name)?;
                println!(
                    "  roster     {name:44} {:>5} ids, {} named",
                    parsed.neo_ids.len(),
                    h.named_links
                );
                sources.push(name);
            }
            _ => println!("  skipped    {name}"),
        }
    }

    // The bridge: resolve Neo IDs to registration numbers through names.
    let report = engine::link_by_name(&store)?;
    println!(
        "\n  name bridge: {} exact, {} approximate, {} ambiguous (left unresolved), {} unmatched",
        report.exact, report.approximate, report.ambiguous, report.unmatched
    );

    // --- serialise -------------------------------------------------------
    let identity = engine::build_graph(&store)?;

    let mut edges: Vec<[String; 3]> = Vec::new();
    for (neo, (reg, conf)) in &identity.neo_to_reg {
        edges.push([neo.clone(), reg.clone(), conf.label().to_string()]);
    }

    let mut names: BTreeMap<String, String> = BTreeMap::new();
    for (key, display) in store.spellings()? {
        names.insert(key, display);
    }

    // Which name keys are actually reachable? Anything else is dead weight.
    let mut name_edges: Vec<[String; 3]> = Vec::new();
    for (a, b, conf, _) in store.edges()? {
        if let (Identifier::NeoId(n), Identifier::NameKey(k))
        | (Identifier::NameKey(k), Identifier::NeoId(n)) = (&a, &b)
        {
            name_edges.push([n.as_str().to_string(), k.clone(), conf.label().into()]);
        }
        if let (Identifier::RegNo(r), Identifier::NameKey(k))
        | (Identifier::NameKey(k), Identifier::RegNo(r)) = (&a, &b)
        {
            name_edges.push([r.as_str().to_string(), k.clone(), conf.label().into()]);
        }
    }

    let mut academics: Vec<(String, Option<f64>, Option<String>)> = Vec::new();
    let (base_cgpa, base_branch) = store.baseline(None)?;
    {
        let conn = store.connection();
        let mut stmt = conn.prepare("SELECT reg_no, cgpa, branch FROM academics")?;
        let rows = stmt.query_map([], |r| {
            Ok((
                r.get::<_, String>(0)?,
                r.get::<_, Option<f64>>(1)?,
                r.get::<_, Option<String>>(2)?,
            ))
        })?;
        for row in rows {
            academics.push(row?);
        }
    }

    let pack = serde_json::json!({
        "version": 1,
        "sources": sources,
        "edges": edges,
        "name_edges": name_edges,
        "names": names,
        "academics": academics,
        "baseline_cgpa": base_cgpa,
        "baseline_branch": base_branch,
    });

    let out_dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("reference");
    std::fs::create_dir_all(&out_dir)?;
    let out = out_dir.join("pack.json");
    let text = serde_json::to_string(&pack)?;
    std::fs::write(&out, &text)?;

    let verified = edges
        .iter()
        .filter(|e| e[2] == Confidence::Verified.label())
        .count();
    let high = edges.iter().filter(|e| e[2] == "high").count();
    println!(
        "\n  wrote {} ({:.0} KB)\n    {} students resolvable to academic data ({verified} verified, {high} exact-name, {} approximate)\n    {} academic records, {} baseline values",
        out.display(),
        text.len() as f64 / 1024.0,
        edges.len(),
        edges.len() - verified - high,
        academics.len(),
        base_cgpa.len(),
    );
    Ok(())
}
