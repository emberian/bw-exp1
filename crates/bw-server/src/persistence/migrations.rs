//! Embedded migrations runner
//!
//! Runs SQL migrations on startup, tracking which have been applied.

use sqlx::{Pool, Sqlite};

use super::DbError;

/// Embedded migrations in order of execution.
const MIGRATIONS: &[(&str, &str)] = &[
    ("001_sqlite_schema", include_str!("../../../../migrations/001_sqlite_schema.sql")),
    ("002_seed_factions", include_str!("../../../../migrations/002_seed_factions.sql")),
    ("003_seed_sectors", include_str!("../../../../migrations/003_seed_sectors.sql")),
];

/// Run all pending migrations.
pub async fn run(pool: &Pool<Sqlite>) -> Result<(), DbError> {
    // Create migrations tracking table
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS _migrations (
            name TEXT PRIMARY KEY,
            applied_at TEXT NOT NULL DEFAULT (datetime('now'))
        )
        "#,
    )
    .execute(pool)
    .await?;

    for (name, sql) in MIGRATIONS {
        // Check if already applied
        let applied: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM _migrations WHERE name = ?")
            .bind(name)
            .fetch_one(pool)
            .await?;

        if applied.0 == 0 {
            tracing::info!("Running migration: {}", name);

            // Execute migration - split by semicolons and run each statement
            for statement in sql.split(';') {
                // Strip SQL comments and whitespace
                let statement: String = statement
                    .lines()
                    .map(|line| {
                        // Remove inline comments
                        if let Some(idx) = line.find("--") {
                            &line[..idx]
                        } else {
                            line
                        }
                    })
                    .collect::<Vec<_>>()
                    .join("\n");
                let statement = statement.trim();

                if !statement.is_empty() {
                    if let Err(e) = sqlx::query(statement).execute(pool).await {
                        return Err(DbError::Migration(format!(
                            "Failed to run migration '{}': {} (statement: {})",
                            name,
                            e,
                            &statement[..statement.len().min(100)]
                        )));
                    }
                }
            }

            // Mark as applied
            sqlx::query("INSERT INTO _migrations (name) VALUES (?)")
                .bind(name)
                .execute(pool)
                .await?;

            tracing::info!("Migration '{}' applied successfully", name);
        }
    }

    Ok(())
}
