// Test migration setup
mod migration;

use sea_orm::Database;
use std::env;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    dotenvy::dotenv().ok();
    let postgres_url = env::var("POSTGRES_URL")?;
    
    println!("Connecting to PostgreSQL...");
    let db = Database::connect(&postgres_url).await?;
    println!("Connected!\n");
    
    println!("Running migrations...");
    match migration::run_migrations(&db).await {
        Ok(_) => println!("✓ Migrations completed successfully!"),
        Err(e) => {
            let error_msg = e.to_string();
            println!("Migration error: {}", error_msg);
            if error_msg.contains("duplicate key") && error_msg.contains("seaql_migrations") {
                println!("Migrations already applied (duplicate key), continuing...");
            } else {
                return Err(format!("Failed: {}", e).into());
            }
        }
    }
    
    Ok(())
}
