use anyhow::Result;
use std::path::Path;

use crate::index::reader;
use crate::search;

/// Search skills by keyword with relevance scoring.
pub fn run(base_dir: &Path, query: &str, limit: usize) -> Result<()> {
    let index_path = base_dir.join("index.json");
    if !index_path.exists() {
        eprintln!("Index not found. Run 'aas init' first.");
        std::process::exit(1);
    }

    let index = reader::load_index(&index_path)?;
    let results = search::scoring::search(&index, query, limit);

    if results.is_empty() {
        println!("No skills matched '{}'.", query);
        return Ok(());
    }

    println!("{:<30} {:<15} {:<6} Score", "Name", "Category", "Risk");
    println!("{}", "-".repeat(72));

    for r in &results {
        println!("{:<30} {:<15} {:<6} {}", r.name, r.category, r.risk, r.score);
    }

    println!("\n{} result(s) for '{}'.", results.len(), query);
    Ok(())
}
