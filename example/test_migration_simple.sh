#!/bin/bash
set -e

echo "Testing sea-orm-migration..."

# Create a completely fresh database
psql postgresql://postgres:postgres@localhost:5432/postgres -c "DROP DATABASE IF EXISTS migration_test;" 2>/dev/null || true
psql postgresql://postgres:postgres@localhost:5432/postgres -c "CREATE DATABASE migration_test;"

# Export the connection string
export DATABASE_URL="postgresql://postgres:postgres@localhost:5432/migration_test"

# Try running migrations programmatically
cd /Users/cuongtran/Desktop/repo/etl-rust/example

# Create a minimal test program
cat > /tmp/test_migration.rs << 'EOF'
use sea_orm::Database;
use sea_orm_migration::MigratorTrait;

mod migration;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let db_url = std::env::var("DATABASE_URL")?;
    println!("Connecting to: {}", db_url);

    let db = Database::connect(&db_url).await?;
    println!("Connected!");

    println!("Running migrations...");
    migration::Migrator::up(&db, None).await?;
    println!("Migrations completed!");

    Ok(())
}
EOF

echo "Test created - run manually if needed"
