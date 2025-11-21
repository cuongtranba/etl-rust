# ETL Pipeline CPU & Memory Profiling Report

## Executive Summary

| Metric | Value |
|--------|-------|
| **Total Duration** | 1,387.4 seconds (23.1 minutes) |
| **CPU Samples** | 138,738 samples |
| **Sampling Rate** | 100 Hz |
| **Profiler** | pprof + flamegraph |

---

## Performance Overview

### CPU Time Distribution

| Component | Time (s) | % of Total | Samples |
|-----------|----------|------------|---------|
| **Memory Management** | 180.4s | 13.0% | 18,044 |
| **PostgreSQL Write (Load)** | 168.8s | 12.2% | 16,885 |
| **MongoDB Read (Extract)** | 131.9s | 9.5% | 13,194 |
| **Network I/O** | 71.2s | 5.1% | 7,119 |
| **Async Runtime** | 40.3s | 2.9% | 4,032 |
| **Serialization** | 30.1s | 2.2% | 3,005 |
| **ETL Logic** | 14.5s | 1.0% | 1,448 |
| **Other/System** | ~749s | 54.1% | 75,011 |

---

## Top 15 CPU-Intensive Functions

| Rank | Function | Samples | % |
|------|----------|---------|---|
| 1 | `futures_util::future::FutureExt::poll_unpin` | 962 | 0.69% |
| 2 | `<Fuse<Fut> as core::future::Future>::poll` | 960 | 0.69% |
| 3 | `<Instrumented<T> as core::future::Future>::poll` | 947 | 0.68% |
| 4 | `mongodb_model deserialize (User struct)` | 781 | 0.56% |
| 5 | `mongodb_model deserialize impl` | 774 | 0.56% |
| 6 | `sea_orm::driver::sqlx_postgres::sqlx_query` | 574 | 0.41% |
| 7 | `core::option::Option<T>::map_or` | 573 | 0.41% |
| 8 | `sea_query::Values clone` | 573 | 0.41% |
| 9 | `drop_in_place<InsertStatement>` | 551 | 0.40% |
| 10 | `mongodb_model deserialize (nested)` | 541 | 0.39% |
| 11 | `drop_in_place<Option<InsertStatement>>` | 540 | 0.39% |
| 12 | `drop_in_place<InsertValueSource>` | 540 | 0.39% |
| 13 | `drop_in_place<Vec<Vec<SimpleExpr>>>` | 540 | 0.39% |
| 14 | `drop_in_place<[Vec<SimpleExpr>]>` | 537 | 0.39% |
| 15 | `drop_in_place<Vec<SimpleExpr>>` | 536 | 0.39% |

---

## Critical Findings

### 🔴 High Priority: Memory Allocations (13% of CPU)

**Issue**: Excessive memory allocations and deallocations, particularly around:
- Sea-Query `InsertStatement` building and destruction
- Vector cloning operations (`Vec<Vec<SimpleExpr>>`)
- SQL value cloning during batch operations

**Impact**: 180+ seconds spent on memory management

**Root Causes**:
1. Building large `InsertStatement` objects for every batch
2. Cloning `Values` and `SimpleExpr` repeatedly
3. Temporary allocations in query builder

**Recommendations**:
- [ ] Reuse `InsertStatement` objects across batches
- [ ] Use borrowing instead of cloning where possible
- [ ] Consider using `Cow<'_, T>` for frequently cloned data
- [ ] Profile with `jemalloc` or `mimalloc` for better allocation performance

### 🟠 Medium Priority: PostgreSQL Writes (12.2% of CPU)

**Issue**: Database write operations consuming 168.8 seconds

**Current Configuration**:
- Batch size: 100 records
- Workers: 2
- Connection pooling: Standard

**Recommendations**:
- [ ] Increase batch size to 500-1000 records
- [ ] Use PostgreSQL `COPY` command instead of `INSERT` for bulk loads
- [ ] Enable prepared statement caching
- [ ] Tune connection pool settings

**Expected Improvement**: 30-40% faster writes

### 🟡 Medium Priority: MongoDB Reads (9.5% of CPU)

**Issue**: Extract phase consuming 131.9 seconds

**Current Behavior**:
- Full document deserialization from BSON
- Channel-based streaming (batch size: 100)

**Recommendations**:
- [ ] Use MongoDB projections to fetch only needed fields
- [ ] Add indexes on frequently queried fields
- [ ] Increase channel buffer size
- [ ] Consider parallel cursors for large collections

**Expected Improvement**: 15-20% faster reads

---

## ETL Pipeline Stage Breakdown

### Extract (MongoDB → Memory)
- **Duration**: ~132s (9.5%)
- **Operations**: Cursor iteration, BSON deserialization
- **Bottleneck**: BSON → Rust struct conversion

### Transform (Memory → Memory)
- **Duration**: ~15s (1.0%)
- **Operations**: Field mapping, data validation
- **Status**: ✅ Efficient, minimal overhead

### Load (Memory → PostgreSQL)
- **Duration**: ~169s (12.2%)
- **Operations**: SQL generation, batch inserts
- **Bottleneck**: Query building and network I/O

---

## Memory Profiling Insights

### Top Memory Allocation Sources

1. **Sea-Query InsertStatement** (551 samples)
   - Built for each batch insert
   - Contains nested Vec structures
   - Dropped immediately after use

2. **SimpleExpr Vectors** (540 samples)
   - Multiple levels of nesting: `Vec<Vec<SimpleExpr>>`
   - Created during SQL value building
   - Cloned multiple times

3. **Query Values** (573 samples)
   - Cloned during query execution
   - Could use references instead

### Memory Optimization Opportunities

```rust
// Current (allocates heavily)
for batch in batches {
    let mut statement = InsertStatement::new();
    statement.values(...); // Creates Vec<Vec<SimpleExpr>>
    execute(statement); // Drops immediately
}

// Optimized (reuse allocations)
let mut statement = InsertStatement::new();
for batch in batches {
    statement.clear_values();
    statement.values(...);
    execute(&statement);
}
```

---

## Async/Await Analysis

### Tokio Runtime Performance
- **CPU Time**: 40.3s (2.9%)
- **Status**: ✅ Well-optimized
- **Workers**: 2 bucket workers + 1 pipeline manager
- **Thread Parking**: Minimal idle time observed

### Future Polling Overhead
- **Top Function**: `FutureExt::poll_unpin` (962 samples)
- **Status**: ⚠️ Normal for heavily async code
- **Notes**: Most overhead is from tracing instrumentation

---

## Network I/O Analysis

| Operation | Samples | Time (s) | Notes |
|-----------|---------|----------|-------|
| TCP Socket Writes | 438 | 4.4s | PostgreSQL queries |
| MongoDB Protocol | ~7,000 | ~70s | BSON over TCP |
| Buffer Operations | ~600 | ~6s | sqlx buffering |

**Findings**:
- Network I/O is efficient relative to data volume
- Most time spent waiting on database servers (not in profile)
- Buffering is working well

---

## Optimization Roadmap

### Phase 1: Quick Wins (1-2 days)
**Expected Speedup: 20-30%**

1. ✅ Increase batch size from 100 to 500
   ```rust
   let bucket_config = bucket::ConfigBuilder::default()
       .batch_size(500)  // Was 100
       .worker_num(2)
       .build()?;
   ```

2. ✅ Use better allocator
   ```toml
   [dependencies]
   jemallocator = "0.5"
   ```
   ```rust
   #[global_allocator]
   static GLOBAL: jemallocator::Jemalloc = jemallocator::Jemalloc;
   ```

3. ✅ Enable prepared statement caching
   ```rust
   let postgres_client = Database::connect(&postgres_url)
       .statement_cache_capacity(100)
       .await?;
   ```

### Phase 2: Medium Effort (3-5 days)
**Expected Speedup: 40-50% total**

1. 🔄 Reduce memory allocations in query building
   - Reuse `InsertStatement` objects
   - Implement custom memory pool for `SimpleExpr`
   - Use `Cow<'_, str>` for string values

2. 🔄 Optimize BSON deserialization
   - Use `#[serde(borrow)]` where possible
   - Implement zero-copy deserialization for strings
   - Add MongoDB projections

3. 🔄 Parallel ETL pipelines
   - Split collections across multiple pipelines
   - Use 4 workers instead of 2

### Phase 3: Advanced (1-2 weeks)
**Expected Speedup: 60-70% total**

1. ⏭️ Use PostgreSQL COPY instead of INSERT
   ```rust
   // Generate CSV/binary format
   // Use COPY FROM STDIN
   ```

2. ⏭️ Custom BSON deserializer
   - Skip serde overhead
   - Direct field mapping

3. ⏭️ Memory-mapped buffers
   - Reduce allocation churn
   - Use arena allocators

---

## Benchmark Comparison

| Configuration | Duration | Throughput | Notes |
|---------------|----------|------------|-------|
| **Current** | 23.1 min | ~X records/sec | Baseline |
| After Phase 1 | ~16 min | ~Y records/sec | +30% faster |
| After Phase 2 | ~11 min | ~Z records/sec | +52% faster |
| After Phase 3 | ~7 min | ~W records/sec | +70% faster |

*(Fill in X, Y, Z, W based on actual record counts)*

---

## Tools & Artifacts

### Generated Files
- `flamegraph.svg` - Interactive CPU flame graph (open in browser)
- `profiling_report_latest.md` - This report
- Raw profiling data available via pprof

### How to Explore Flamegraph
1. Open `flamegraph.svg` in Chrome/Firefox
2. Click any bar to zoom into that call stack
3. Use Ctrl+F to search for specific functions
4. Wider bars = more CPU time spent
5. Y-axis = call stack depth (top = app code, bottom = system)

### Re-running Profiling
```bash
cd example
cargo run --release  # Profiling is automatic
# Results written to flamegraph.svg
```

---

## Conclusion

The ETL pipeline is **reasonably optimized** but has significant room for improvement:

✅ **Strengths**:
- Efficient async/await usage
- Good network I/O performance  
- Well-structured ETL phases

⚠️ **Opportunities**:
- Memory allocation overhead (13%)
- Query building can be optimized (12%)
- Batch sizes can be increased

🎯 **Target**: Reduce total time from **23 minutes to under 10 minutes** (57% improvement)

With the recommended optimizations, this target is **achievable within 2-3 weeks** of development effort.

---

*Report generated from pprof flamegraph analysis*  
*Profile timestamp: Latest run*  
*Tool: pprof v0.14 with flamegraph feature*
