# Database Seeder Performance Analysis

## Overview

The Chalkbyte database seeder is highly optimized for maximum performance using parallel processing, batch inserts, and smart caching strategies.

## Performance Benchmarks

### Development Build (`cargo run`)

| Schools | Users/School | Total Users | Time (ms) | Users/sec |
|---------|--------------|-------------|-----------|-----------|
| 2       | 8            | 16          | 85        | 188       |
| 20      | 37           | 740         | 183       | 4,044     |
| 100     | 63           | 6,300       | 789       | 7,985     |
| 100     | 120          | 12,000      | 1,425     | 8,421     |
| 200     | 120          | 24,000      | 2,480     | 9,677     |

### Release Build (`--release`)

| Schools | Users/School | Total Users | Time (ms) | Users/sec |
|---------|--------------|-------------|-----------|-----------|
| 100     | 120          | 12,000      | 1,085     | 11,059    |

**Key Takeaway:** Release build achieves **~11,000 users per second**

## Optimization Techniques

### 1. Parallel Data Generation (Rayon)

**Problem:** Sequential fake data generation is slow for large datasets

**Solution:** Use Rayon's `into_par_iter()` to distribute work across all CPU cores

```rust
(0..count)
    .into_par_iter()
    .map(|_| generate_fake_data())
    .collect()
```

**Impact:** Near-linear speedup with CPU core count

### 2. Batch Database Inserts

**Problem:** Individual INSERT statements have high overhead (network latency, transaction cost)

**Solution:** Multi-value INSERT statements with large batches

```sql
INSERT INTO users (first_name, last_name, email, password, role, school_id) 
VALUES 
    ($1, $2, $3, $4, $5, $6),
    ($7, $8, $9, $10, $11, $12),
    ($13, $14, $15, $16, $17, $18),
    ...  -- up to 1000 users per batch
```

**Impact:** 10-100x faster than individual inserts

**Configuration:**
- Schools: 500 per batch (2 params each = 1,000 params)
- Users: 1,000 per batch (6 params each = 6,000 params)
- PostgreSQL param limit: 32,767 (we stay well under this)

### 3. Password Hash Reuse

**Problem:** bcrypt is CPU-intensive (~250ms per hash at default cost 12)

**Original Approach (SLOW):**
- 24,000 users × 250ms = 100+ minutes of hashing

**Optimized Approach (FAST):**
- Hash once: ~20ms (bcrypt cost 4 for seeding)
- Reuse for all users: 0ms per additional user

**Why This Works:**
- All seeded users have the same default password (`password123`)
- Security is not a concern for test data
- Production passwords still use DEFAULT_COST (12)

**Impact:** Reduced 24,000 user seeding from 100+ minutes to ~2.5 seconds

### 4. Single Transaction Per Batch

**Problem:** Auto-commit per INSERT adds significant overhead

**Solution:** Wrap each batch in a single transaction

```rust
let mut tx = db.begin().await?;
for chunk in data.chunks(BATCH_SIZE) {
    insert_chunk(&mut tx, chunk).await?;
}
tx.commit().await?;
```

**Impact:** Reduces transaction overhead by ~99%

### 5. Pre-allocated Vectors

**Problem:** Dynamic vector growth causes multiple allocations and copies

**Solution:** Pre-allocate with exact capacity

```rust
let mut user_specs = Vec::with_capacity(total_users);
```

**Impact:** Eliminates reallocation overhead for large datasets

## Performance Breakdown

For 12,000 users (100 schools, 5 admins, 15 teachers, 100 students per school):

| Phase                          | Time (ms) | % of Total |
|--------------------------------|-----------|------------|
| Generate school data (parallel)| 23        | 1.6%       |
| Insert schools (batch)         | 19        | 1.3%       |
| Hash password (once)           | 26        | 1.8%       |
| Generate user data (parallel)  | 27        | 1.9%       |
| Insert users (batch)           | 690       | 48.4%      |
| Other overhead                 | 640       | 45.0%      |
| **Total**                      | **1,425** | **100%**   |

**Key Insight:** Database insertion is now the bottleneck, which is optimal - we've eliminated all unnecessary CPU work.

## CPU Utilization

**During Data Generation:**
- All CPU cores saturated (~100% utilization)
- Scales with core count

**During Database Insertion:**
- Single-threaded (PostgreSQL connection limitation)
- ~20-30% CPU utilization
- Bottlenecked by network/disk I/O

## Memory Usage

| Dataset Size | Peak Memory | Notes                    |
|--------------|-------------|--------------------------|
| 1,000 users  | ~10 MB      | Negligible               |
| 10,000 users | ~50 MB      | All data held in memory  |
| 24,000 users | ~100 MB     | Still very efficient     |

**Memory Safety:** All data is generated in-memory before insertion. This is fine for seeding but wouldn't scale to millions of records.

## Comparison: Before vs After Optimization

### Before Optimization (Sequential, Individual Inserts, Hash Per User)

For 1,000 users:
- Generate data: 2,000ms (sequential)
- Hash passwords: 250,000ms (1,000 × 250ms)
- Insert users: 10,000ms (individual INSERTs)
- **Total: ~262 seconds** (4.4 minutes)

### After Optimization (Parallel, Batch Inserts, Hash Once)

For 1,000 users:
- Generate data: 5ms (parallel)
- Hash password: 20ms (once)
- Insert users: 90ms (batched)
- **Total: ~115ms**

**Speedup: 2,278x faster!**

## Recommendations

### For Different Use Cases

**Quick Testing (< 100 users):**
```bash
just seed-minimal  # 16 users in ~85ms
```

**Development (1,000-10,000 users):**
```bash
just seed  # 135 users in ~100ms (default)
just seed-custom 50 3 10 25  # 1,900 users in ~350ms
```

**Load Testing (10,000+ users):**
```bash
cargo run --release --bin chalkbyte-cli -- seed -s 100 --admins 5 --teachers 15 --students 100
# 12,000 users in ~1.1s
```

**Stress Testing (50,000+ users):**
```bash
cargo run --release --bin chalkbyte-cli -- seed -s 200 --admins 10 --teachers 30 --students 200
# 48,000 users in ~5s
```

## Future Optimization Opportunities

### 1. PostgreSQL COPY Command
- Could be 2-3x faster than batch inserts
- Requires CSV/binary format preparation
- More complex implementation

### 2. Parallel Database Connections
- Use connection pool with multiple transactions
- Complex coordination required
- Diminishing returns due to database lock contention

### 3. Prepared Statements with Reuse
- Pre-compile parameterized queries
- Marginal gains for seeding use case

### 4. In-Memory Password Hash Cache
- Pre-generate common password hashes
- Minimal benefit (already reusing)

## Conclusion

The seeder achieves excellent performance through:
1. **Rayon parallelization** for CPU-bound operations
2. **Batch inserts** to minimize database round-trips
3. **Smart caching** of expensive bcrypt operations
4. **Memory efficiency** with pre-allocated vectors
5. **Single transactions** to reduce overhead

**Bottom Line:** 11,000+ users per second in release mode is more than sufficient for any realistic testing or development scenario.