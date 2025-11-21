# ETL Streaming Benchmarks

This directory contains comprehensive benchmark tests for the ETL pipeline manager, focusing on streaming data extraction, transformation, and loading without database dependencies.

## Overview

The benchmarks simulate a realistic ETL scenario similar to the database example (`example/src/main.rs`) but with in-memory data generation and processing. They measure:

- **Throughput**: Records processed per second
- **Performance**: Time taken for different configurations
- **Scalability**: How performance scales with different parameters

## Benchmark Scenarios

### 1. Batch Size Comparison (`bench_streaming_etl_batch_sizes`)
Tests different batch sizes to find optimal batching configuration:
- Batch sizes: 10, 50, 100, 500
- Fixed: 1000 records, 4 workers
- **Purpose**: Determine the most efficient batch size for your workload

### 2. Worker Count Scaling (`bench_streaming_etl_worker_counts`)
Tests parallel processing with different worker counts:
- Worker counts: 1, 2, 4, 8
- Fixed: 1000 records, batch size 100
- **Purpose**: Understand how parallelism affects performance

### 3. Data Volume Performance (`bench_streaming_etl_data_volumes`)
Tests performance with varying data volumes:
- Record counts: 100, 500, 1000, 2000
- Fixed: batch size 100, 4 workers
- **Purpose**: Understand performance characteristics at different scales

### 4. Parallel Pipelines (`bench_streaming_etl_parallel_pipelines`)
Tests running multiple ETL pipelines concurrently:
- Pipeline counts: 1, 2, 4
- Fixed: 500 records per pipeline, batch size 100
- **Purpose**: Test concurrent pipeline execution

### 5. Optimal Configuration (`bench_streaming_etl_optimal_config`)
Tests a high-performance configuration:
- Records: 5000
- Batch size: 500
- Workers: 8
- **Purpose**: Demonstrate peak performance capability

## Running Benchmarks

### Run All Benchmarks
```bash
cargo bench
```

### Run Specific Benchmark Group
```bash
# Test batch size variations
cargo bench -- streaming_etl_batch_sizes

# Test worker count variations
cargo bench -- streaming_etl_worker_counts

# Test data volume scaling
cargo bench -- streaming_etl_data_volumes

# Test parallel pipelines
cargo bench -- streaming_etl_parallel_pipelines

# Test optimal configuration
cargo bench -- streaming_etl_optimal
```

### Run with Specific Filter
```bash
# Run only batch_size tests
cargo bench -- batch_size

# Run only worker tests
cargo bench -- workers
```

## Understanding Results

Criterion outputs detailed statistics including:
- **time**: Average execution time
- **thrpt**: Throughput (elements/second)
- **change**: Performance change from baseline (after first run)

Example output:
```
streaming_etl_batch_sizes/batch_size/100
                        time:   [45.234 ms 45.891 ms 46.612 ms]
                        thrpt:  [21.45K elem/s 21.79K elem/s 22.10K elem/s]
```

## Mock Data Structure

The benchmark generates complex mock user data similar to the database example:
- User profile (name, email, age, address)
- Education and experience records
- Activity logs and transactions
- Messages with attachments
- Social media data (posts, groups, connections)
- Large binary data blobs (50KB per user)

This simulates real-world ETL complexity with:
- Nested data structures
- One-to-many relationships
- Large data payloads
- Complex transformations

## Benchmark Reports

Criterion generates HTML reports in `target/criterion/`:
- Open `target/criterion/report/index.html` in a browser
- View detailed charts, statistics, and comparisons
- Track performance changes over time

## Tips for Accurate Benchmarks

1. **Close other applications** to reduce system noise
2. **Run multiple times** - Criterion automatically does statistical analysis
3. **Don't run on battery** - CPU throttling can affect results
4. **Keep system cool** - Thermal throttling affects performance
5. **Use release mode** - Benchmarks automatically use optimized builds

## Customizing Benchmarks

To add new benchmark scenarios, edit `etl_streaming_benchmark.rs`:

```rust
fn bench_my_scenario(c: &mut Criterion) {
    let mut group = c.benchmark_group("my_group");
    let runtime = tokio::runtime::Runtime::new().unwrap();
    
    group.bench_function("my_test", |b| {
        b.to_async(&runtime).iter(|| async {
            // Your benchmark code
        });
    });
    
    group.finish();
}

// Add to criterion_group! macro
criterion_group!(benches, ..., bench_my_scenario);
```

## Performance Expectations

Typical performance on modern hardware (8-core CPU):
- **Small batches** (10-50): Good for low latency, moderate throughput
- **Medium batches** (100-200): Balanced latency and throughput
- **Large batches** (500+): Best throughput, higher latency
- **Worker scaling**: Near-linear up to available CPU cores
- **Throughput**: 10K-50K records/second depending on configuration

## Notes

- Benchmarks use synthetic data generation (no I/O overhead)
- Real-world performance with databases will be slower
- Network and disk I/O are major factors not captured here
- Use these benchmarks to optimize ETL pipeline configuration
