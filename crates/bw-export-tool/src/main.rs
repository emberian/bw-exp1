//! CLI tool for inspecting and manipulating Blackwing export files
//!
//! Usage:
//! - `bw-export info <file>` - Show export information
//! - `bw-export list <file> <type>` - List entities of a type
//! - `bw-export extract <file> <type> --format json` - Extract entities to JSON
//! - `bw-export scripts <file>` - List scripts in export
//! - `bw-export diff <old> <new>` - Compare two exports

use anyhow::{Context, Result};
use bw_export::{ExportReader, ExportSummary};
use clap::{Parser, Subcommand};
use colored::Colorize;
use std::path::PathBuf;
use tabled::{Table, Tabled};

#[derive(Parser)]
#[command(name = "bw-export")]
#[command(about = "CLI tool for Blackwing export files")]
#[command(version)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Show export information
    Info {
        /// Path to export file
        file: PathBuf,
    },

    /// List entities in the export
    List {
        /// Path to export file
        file: PathBuf,
        /// Entity type (ships, players, sectors, missions)
        entity_type: String,
        /// Output as JSON
        #[arg(long)]
        json: bool,
    },

    /// Extract entities to stdout
    Extract {
        /// Path to export file
        file: PathBuf,
        /// Entity type to extract
        entity_type: String,
        /// Output format
        #[arg(long, default_value = "json")]
        format: String,
    },

    /// List scripts in the export
    Scripts {
        /// Path to export file
        file: PathBuf,
    },

    /// List definitions in the export
    Definitions {
        /// Path to export file
        file: PathBuf,
    },

    /// Show a script's content
    Script {
        /// Path to export file
        file: PathBuf,
        /// Script path within export
        script_path: String,
    },

    /// Compare two exports
    Diff {
        /// Old export file
        old: PathBuf,
        /// New export file
        new: PathBuf,
    },

    /// Validate export integrity
    Validate {
        /// Path to export file
        file: PathBuf,
    },
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Info { file } => cmd_info(&file),
        Commands::List { file, entity_type, json } => cmd_list(&file, &entity_type, json),
        Commands::Extract { file, entity_type, format } => cmd_extract(&file, &entity_type, &format),
        Commands::Scripts { file } => cmd_scripts(&file),
        Commands::Definitions { file } => cmd_definitions(&file),
        Commands::Script { file, script_path } => cmd_script(&file, &script_path),
        Commands::Diff { old, new } => cmd_diff(&old, &new),
        Commands::Validate { file } => cmd_validate(&file),
    }
}

fn cmd_info(file: &PathBuf) -> Result<()> {
    let reader = ExportReader::open(file)
        .with_context(|| format!("Failed to open export: {}", file.display()))?;

    let summary: ExportSummary = (&reader).into();
    let manifest = reader.manifest();

    println!("{}", "Export Information".bold().cyan());
    println!("{}", "=".repeat(50));
    println!("  {}: {}", "Name".bold(), manifest.name);
    println!("  {}: {}", "Version".bold(), manifest.version);
    println!("  {}: {}", "Tick".bold(), manifest.tick);

    // Format timestamp
    let created = chrono::DateTime::from_timestamp(manifest.created_at as i64, 0)
        .map(|dt: chrono::DateTime<chrono::Utc>| dt.format("%Y-%m-%d %H:%M:%S UTC").to_string())
        .unwrap_or_else(|| manifest.created_at.to_string());
    println!("  {}: {}", "Created".bold(), created);

    // Source
    let source = match &manifest.source {
        bw_export::ExportSource::Live => "Live Server".to_string(),
        bw_export::ExportSource::Playtest { name, .. } => format!("Playtest: {}", name),
        bw_export::ExportSource::Export { path, .. } => format!("Export: {}", path),
    };
    println!("  {}: {}", "Source".bold(), source);

    if let Some(notes) = &manifest.notes {
        println!("  {}: {}", "Notes".bold(), notes);
    }

    println!();
    println!("{}", "Contents".bold().cyan());
    println!("{}", "-".repeat(50));

    // Entity counts
    if !summary.entity_counts.is_empty() {
        println!("  {}:", "Entities".bold());
        for (entity_type, count) in &summary.entity_counts {
            println!("    - {}: {}", entity_type, count.to_string().green());
        }
        println!("    {} {}", "Total:".bold(), summary.total_entities.to_string().green());
    }

    // Other contents
    let mut flags = Vec::new();
    if summary.has_scripts {
        flags.push(format!("Scripts ({})", summary.script_count));
    }
    if summary.has_definitions {
        flags.push(format!("Definitions ({})", summary.definition_count));
    }
    if summary.has_config {
        flags.push("Config".to_string());
    }
    if summary.has_sqlite {
        flags.push("SQLite Snapshot".to_string());
    }

    if !flags.is_empty() {
        println!("  {}: {}", "Includes".bold(), flags.join(", "));
    }

    // Size
    let size_mb = summary.uncompressed_size as f64 / (1024.0 * 1024.0);
    println!("  {}: {:.2} MB (uncompressed)", "Size".bold(), size_mb);

    Ok(())
}

fn cmd_list(file: &PathBuf, entity_type: &str, json: bool) -> Result<()> {
    let mut reader = ExportReader::open(file)
        .with_context(|| format!("Failed to open export: {}", file.display()))?;

    let entities: Vec<serde_json::Value> = reader.read_entities(entity_type)
        .with_context(|| format!("Failed to read {} entities", entity_type))?;

    if json {
        println!("{}", serde_json::to_string_pretty(&entities)?);
    } else {
        println!("{} {} entities:", entities.len(), entity_type);
        for entity in &entities {
            // Try to extract id and name
            let id = entity.get("id")
                .and_then(|v| v.as_str())
                .unwrap_or("?");
            let name = entity.get("name")
                .or_else(|| entity.get("username"))
                .or_else(|| entity.get("title"))
                .and_then(|v| v.as_str())
                .unwrap_or("unnamed");

            println!("  {} - {}", id.chars().take(8).collect::<String>().yellow(), name);
        }
    }

    Ok(())
}

fn cmd_extract(file: &PathBuf, entity_type: &str, format: &str) -> Result<()> {
    let mut reader = ExportReader::open(file)
        .with_context(|| format!("Failed to open export: {}", file.display()))?;

    let entities: Vec<serde_json::Value> = reader.read_entities(entity_type)
        .with_context(|| format!("Failed to read {} entities", entity_type))?;

    match format {
        "json" => {
            println!("{}", serde_json::to_string_pretty(&entities)?);
        }
        "jsonl" => {
            for entity in &entities {
                println!("{}", serde_json::to_string(entity)?);
            }
        }
        _ => {
            anyhow::bail!("Unsupported format: {}. Use 'json' or 'jsonl'.", format);
        }
    }

    Ok(())
}

fn cmd_scripts(file: &PathBuf) -> Result<()> {
    let reader = ExportReader::open(file)
        .with_context(|| format!("Failed to open export: {}", file.display()))?;

    let scripts = reader.list_scripts();

    if scripts.is_empty() {
        println!("No scripts in export.");
    } else {
        println!("{} scripts:", scripts.len());
        for script in scripts {
            println!("  {}", script);
        }
    }

    Ok(())
}

fn cmd_definitions(file: &PathBuf) -> Result<()> {
    let reader = ExportReader::open(file)
        .with_context(|| format!("Failed to open export: {}", file.display()))?;

    let defs = reader.list_definitions();

    if defs.is_empty() {
        println!("No definitions in export.");
    } else {
        println!("{} definitions:", defs.len());
        for def in defs {
            println!("  {}", def);
        }
    }

    Ok(())
}

fn cmd_script(file: &PathBuf, script_path: &str) -> Result<()> {
    let mut reader = ExportReader::open(file)
        .with_context(|| format!("Failed to open export: {}", file.display()))?;

    let content = reader.read_script(script_path)
        .with_context(|| format!("Failed to read script: {}", script_path))?;

    println!("{}", content);

    Ok(())
}

#[derive(Tabled)]
struct DiffRow {
    #[tabled(rename = "Type")]
    entity_type: String,
    #[tabled(rename = "Old")]
    old_count: String,
    #[tabled(rename = "New")]
    new_count: String,
    #[tabled(rename = "Change")]
    change: String,
}

fn cmd_diff(old_path: &PathBuf, new_path: &PathBuf) -> Result<()> {
    let old_reader = ExportReader::open(old_path)
        .with_context(|| format!("Failed to open old export: {}", old_path.display()))?;
    let new_reader = ExportReader::open(new_path)
        .with_context(|| format!("Failed to open new export: {}", new_path.display()))?;

    let old_summary: ExportSummary = (&old_reader).into();
    let new_summary: ExportSummary = (&new_reader).into();

    println!("{}", "Export Comparison".bold().cyan());
    println!("{}", "=".repeat(50));
    println!("Old: {} (tick {})", old_summary.name, old_reader.manifest().tick);
    println!("New: {} (tick {})", new_summary.name, new_reader.manifest().tick);
    println!();

    // Collect all entity types
    let mut all_types: Vec<_> = old_summary.entity_counts.keys()
        .chain(new_summary.entity_counts.keys())
        .collect();
    all_types.sort();
    all_types.dedup();

    let mut rows = Vec::new();
    for entity_type in all_types {
        let old_count = old_summary.entity_counts.get(entity_type).copied().unwrap_or(0);
        let new_count = new_summary.entity_counts.get(entity_type).copied().unwrap_or(0);

        let change = match new_count.cmp(&old_count) {
            std::cmp::Ordering::Greater => format!("+{}", new_count - old_count).green().to_string(),
            std::cmp::Ordering::Less => format!("-{}", old_count - new_count).red().to_string(),
            std::cmp::Ordering::Equal => "=".to_string(),
        };

        rows.push(DiffRow {
            entity_type: entity_type.clone(),
            old_count: old_count.to_string(),
            new_count: new_count.to_string(),
            change,
        });
    }

    let table = Table::new(rows);
    println!("{}", table);

    // Summary changes
    println!();
    let old_total = old_summary.total_entities;
    let new_total = new_summary.total_entities;
    let total_change = new_total as i64 - old_total as i64;
    println!(
        "Total entities: {} -> {} ({})",
        old_total,
        new_total,
        if total_change >= 0 {
            format!("+{}", total_change).green()
        } else {
            format!("{}", total_change).red()
        }
    );

    Ok(())
}

fn cmd_validate(file: &PathBuf) -> Result<()> {
    let mut reader = ExportReader::open(file)
        .with_context(|| format!("Failed to open export: {}", file.display()))?;

    println!("{}", "Validating export...".cyan());

    let mut errors = 0;
    let entries = reader.entries().to_vec();

    for entry in entries {
        match reader.read_raw(&entry.path) {
            Ok(_) => {
                println!("  {} {}", "OK".green(), entry.path);
            }
            Err(e) => {
                println!("  {} {} - {}", "ERR".red(), entry.path, e);
                errors += 1;
            }
        }
    }

    println!();
    if errors == 0 {
        println!("{}", "Export is valid!".green().bold());
    } else {
        println!("{} {} errors found", "INVALID:".red().bold(), errors);
    }

    Ok(())
}
