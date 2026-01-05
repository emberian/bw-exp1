//! Embedded migrations runner
//!
//! Runs SQL migrations on startup, tracking which have been applied.

use sea_orm::{ConnectionTrait, DatabaseConnection, DbBackend, Statement};

use super::DbError;

/// Embedded migrations in order of execution.
const MIGRATIONS: &[(&str, &str)] = &[
    ("001_sqlite_schema", include_str!("../../../../migrations/001_sqlite_schema.sql")),
    ("002_seed_factions", include_str!("../../../../migrations/002_seed_factions.sql")),
    ("003_seed_sectors", include_str!("../../../../migrations/003_seed_sectors.sql")),
];

/// Run all pending migrations using SeaORM.
pub async fn run_sea(conn: &DatabaseConnection) -> Result<(), DbError> {
    // Create migrations tracking table
    conn.execute_unprepared(
        r#"
        CREATE TABLE IF NOT EXISTS _migrations (
            name TEXT PRIMARY KEY,
            applied_at TEXT NOT NULL DEFAULT (datetime('now'))
        )
        "#,
    )
    .await?;

    for (name, sql) in MIGRATIONS {
        // Check if already applied
        let result = conn
            .query_one(Statement::from_sql_and_values(
                DbBackend::Sqlite,
                "SELECT COUNT(*) as count FROM _migrations WHERE name = ?",
                [(*name).into()],
            ))
            .await?;

        let applied = result
            .map(|row| {
                use sea_orm::TryGetable;
                i64::try_get(&row, "", "count").unwrap_or(0)
            })
            .unwrap_or(0);

        if applied == 0 {
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
                    if let Err(e) = conn.execute_unprepared(statement).await {
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
            conn.execute(Statement::from_sql_and_values(
                DbBackend::Sqlite,
                "INSERT INTO _migrations (name) VALUES (?)",
                [(*name).into()],
            ))
            .await?;

            tracing::info!("Migration '{}' applied successfully", name);
        }
    }

    Ok(())
}
