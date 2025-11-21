# ETL Database Example

A complete example demonstrating the `etl-rust` framework with real database integration, migrating data from MongoDB to PostgreSQL.

## Features

- **Extract** from MongoDB using async cursors
- **Transform** complex nested documents to relational models
- **Load** with batch inserts to PostgreSQL
- **Automated database migrations** using sea-orm-migration
- **Performance optimizations**: batch processing, parallel workers
- **Graceful error handling** and migration tracking

## Architecture

### Source: MongoDB
- Document-based storage
- Complex nested structures
- Collections: `users` with embedded documents

### Destination: PostgreSQL  
- Relational database
- Normalized schema with 15 tables
- Foreign key relationships

## Database Schema

The PostgreSQL schema consists of 14 related tables:

| Table | Description | Relationship |
|-------|-------------|--------------|
| `users` | Primary user records | Parent |
| `addresses` | User addresses with coordinates | 1:1 with users |
| `profiles` | User bio, interests, skills | 1:1 with users |
| `education` | Education history | 1:many with profiles |
| `experience` | Work experience | 1:many with profiles |
| `preferences` | User preferences | 1:1 with users |
| `settings` | Individual settings | 1:many with preferences |
| `activity_log` | Activity history | 1:many with users |
| `transactions` | Transaction records | 1:many with users |
| `messages` | User messages | 1:many with users |
| `attachments` | Message attachments | 1:many with messages |
| `social_media` | Social profiles | 1:1 with users |
| `posts` | Social media posts | 1:many with social_media |
| `groups` | Group memberships | 1:many with social_media |
| `large_data` | Large blob data | 1:1 with users |

## Setup

### Prerequisites

- **MongoDB**: Running on `localhost:27017` with authentication
- **PostgreSQL**: Running on `localhost:5432`
- **Rust**: 1.70+ with cargo

### 1. Configure Database Connections

Create `.env` file:

```bash
MONGODB_URL=mongodb://admin:password123@localhost:27017/
POSTGRES_URL=postgresql://postgres:postgres@localhost:5432/etl_example
```

### 2. Create PostgreSQL Database

```bash
createdb etl_example
```

### 3. Generate Sample Data

Generate 10,000 sample users in MongoDB:

```bash
echo "10000" | python3 generate_large_dataset.py
```

Or manually seed with the JavaScript script:

```bash
mongosh "mongodb://admin:password123@localhost:27017/" < seed_mongodb.js
```

### 4. Run Migrations & ETL

The application automatically runs database migrations on startup:

```bash
cargo run --release
```

## PostgreSQL Schema Setup

### Automatic Migrations (Recommended)

The application uses **sea-orm-migration** to automatically create and manage database schema. Migrations run automatically when you start the application.

**No manual setup required!** Just run:

```bash
cargo run --release
```

The migrations will:
1. Create the `seaql_migrations` tracking table
2. Apply all 15 table migrations in order
3. Track which migrations have been applied
4. Skip already-applied migrations on subsequent runs

### Manual SQL Script (Alternative)

If you prefer manual schema management, use the provided SQL script:

```bash
# Create database
createdb etl_example

# Apply schema
psql postgresql://postgres:postgres@localhost:5432/etl_example -f create_schema.sql
```

### Migration Files

All 15 migrations are defined in `src/migration.rs` with proper **manual `MigrationName` trait implementation**:

**Note**: The migrations are organized as nested modules in a single file. Each migration struct manually implements the `MigrationName` trait to provide unique migration names, as the `DeriveMigrationName` macro derives names from the file path (not module names) and would cause all migrations to have the same name.

- `m20231121_000001` - Create users table
- `m20231121_000002` - Create addresses table  
- `m20231121_000003` - Create profiles table
- `m20231121_000004` - Create education table
- `m20231121_000005` - Create experience table
- `m20231121_000006` - Create preferences table
- `m20231121_000007` - Create settings table
- `m20231121_000008` - Create activity_log table
- `m20231121_000009` - Create transactions table
- `m20231121_000010` - Create messages table
- `m20231121_000011` - Create attachments table
- `m20231121_000012` - Create social_media table
- `m20231121_000013` - Create posts table
- `m20231121_000014` - Create groups table
- `m20231121_000015` - Create large_data table

### Reset Database

To start fresh:

```bash
dropdb etl_example
createdb etl_example
psql postgresql://postgres:postgres@localhost:5432/etl_example -f create_schema.sql
```

## Performance Configuration

Configured in `main.rs`:

```rust
let bucket_config = bucket::ConfigBuilder::default()
    .batch_size(100usize)      // Process 100 items per batch
    .worker_num(2usize)          // Use 2 parallel workers
    .build()
    .unwrap();

let manager_config = ManagerConfig { 
    worker_num: 1                // 1 pipeline manager worker
};
```

### Tuning

- **batch_size**: Larger = fewer DB round-trips, more memory
- **worker_num**: More workers = higher parallelism (CPU-bound)
- Recommended: Start with defaults, monitor, then adjust

## Example Output

```
ETL with Database Example
=========================

Connecting to databases...
Connected successfully!

Running PostgreSQL migrations...
✓ Migrations completed successfully!

✓ Adding User ETL Pipeline (MongoDB -> PostgreSQL)

--- Starting ETL pipeline ---

Starting ETL pipeline...
Batch inserting 100 users...
Batch inserting 100 addresses...
Batch inserting 100 profiles...
...
✓ Batch inserted 100 users with all related data!
...
ETL pipeline completed successfully!

=== ETL process completed successfully in 2.89s ===
```

## Performance Metrics

**Test Configuration:**
- 10,000 users with nested data
- ~345,000 total records
- Batch size: 100
- Workers: 2

**Results:**
- **Processing Time**: 2.89 seconds  
- **Throughput**: ~3,460 users/second
- **Record Rate**: ~119,000 records/second

## Troubleshooting

### Migration Errors

**Error**: `duplicate key value violates unique constraint "seaql_migrations_pkey"`

**Solution**: Migrations already applied. The application handles this automatically and continues.

### Connection Errors

**MongoDB**:
```bash
# Verify MongoDB is running
mongosh "mongodb://admin:password123@localhost:27017/" --eval "db.version()"
```

**PostgreSQL**:
```bash
# Verify PostgreSQL is running
psql $POSTGRES_URL -c "SELECT version();"
```

### No Data to Migrate

If MongoDB is empty:

```bash
# Generate sample data
python3 generate_large_dataset.py
```

## Project Structure

```
example/
├── src/
│   ├── main.rs              # ETL application entry point
│   ├── migration.rs         # PostgreSQL schema migrations (15 tables)
│   ├── mongodb_model.rs     # MongoDB document models
│   └── postgres_model.rs    # PostgreSQL entity models
├── Cargo.toml               # Dependencies
├── .env                     # Database connection strings
├── create_schema.sql        # Manual schema creation (optional)
├── seed_mongodb.js          # Sample data (2 users)
├── generate_large_dataset.py # Large dataset generator (10K+ users)
└── README.md                # This file
```

## License

Same as parent project (etl-rust)
