use anyhow::Result;
use std::path::PathBuf;

use crate::index::reader;
use crate::search;

/// List skills filtered by category, risk, or tags.
pub fn run(base_dir: &PathBuf, category: Option<&str>, risk: Option<&str>, tags: Vec<String>, limit: usize) -> Result<()> {
    let index_path = base_dir.join("index.json");
    if !index_path.exists() {
        eprintln!("Index not found. Run 'aas init' first.");
        std::process::exit(1);
    }

    let index = reader::load_index(&index_path)?;
    let tag_refs: Vec<&str> = tags.iter().map(|t| t.as_str()).collect();
    let results = search::scoring::filter(&index, category, risk, &tag_refs, limit);

    if results.is_empty() {
        println!("No skills matched the filters.");
        return Ok(());
    }

    println!("{:<30} {:<15} {:<6} {}", "Name", "Category", "Risk", "ID");
    println!("{}", "-".repeat(70));

    for r in &results {
        println!("{:<30} {:<15} {:<6} {}", r.name, r.category, r.risk, r.skill_id);
    }

    println!("\n{} result(s).", results.len());
    Ok(())
}
