# Database Seeders

This document explains how to use the database seeders to populate your Chalkbyte database with fake data for development and testing.

## Overview

The seeder system uses the `fake-rs` library to generate realistic fake data for schools and users. It creates:

- Schools with realistic names and addresses
- Users with different roles (Admin, Teacher, Student)
- Unique email addresses for all users
- Hashed passwords for authentication

## Performance

The seeder is highly optimized for speed:

- **Parallel data generation** using Rayon for CPU-intensive operations
- **Batch inserts** with multi-value INSERT statements (up to 1000 records per batch)
- **Single transaction** per batch to minimize database overhead
- **Password hash reuse** - hashes once and reuses for all users (bcrypt cost 4 for seeding)
- **Pre-allocated vectors** with exact capacity to avoid reallocation

**Benchmarks:**
- 6,300 users in ~800ms
- 24,000 users in ~2.5s
- 100 schools with batch insert in ~20ms

## Commands

### Seed Database

Create schools and users with fake data:

```bash
cargo run --bin chalkbyte-cli -- seed [OPTIONS]
```

**Options:**

- `-s, --schools <NUMBER>` - Number of schools to create (default: 5)
- `--admins <NUMBER>` - Number of admins per school (default: 2)
- `--teachers <NUMBER>` - Number of teachers per school (default: 5)
- `--students <NUMBER>` - Number of students per school (default: 20)

**Examples:**

```bash
# Use default values (5 schools, 2 admins, 5 teachers, 20 students per school)
cargo run --bin chalkbyte-cli -- seed

# Create 10 schools with custom user counts
cargo run --bin chalkbyte-cli -- seed -s 10 --admins 3 --teachers 8 --students 30

# Minimal seed for quick testing
cargo run --bin chalkbyte-cli -- seed -s 2 --admins 1 --teachers 2 --students 5
```

### Clear Seeded Data

Remove all seeded data from the database (keeps system admins):

```bash
cargo run --bin chalkbyte-cli -- clear-seed
```

This command:
- Deletes all users with `@example.com` email addresses (except system admins)
- Deletes all schools
- Preserves system administrators

## Generated Data

### Schools

Each school is generated with:
- **Name**: Random city + random suffix + "School" (e.g., "Springfield Oakwood School")
- **Address**: Realistic US address with street, city, state, and ZIP code

### Users

All users are generated with:
- **First Name**: Random first name from fake-rs
- **Last Name**: Random last name from fake-rs
- **Email**: Format `firstname.lastname+roleN@example.com` (unique per user)
- **Password**: `password123` (same for all seeded users)
- **Role**: Admin, Teacher, or Student
- **School ID**: Assigned to their school

**Email Pattern:**

The email format ensures uniqueness:
```
firstname.lastname+admin0@example.com
firstname.lastname+teacher5@example.com
firstname.lastname+student12@example.com
```

The number suffix is calculated as: `school_index * 100 + user_index`

## Default Password

**All seeded users have the password: `password123`**

This makes it easy to test login functionality during development.

## Use Cases

### Development

Populate your local database with test data:

```bash
# Full dataset for realistic testing
cargo run --bin chalkbyte-cli -- seed

# Login as any user
# Email: check the output from the seed command
# Password: password123
```

### Testing

Create minimal data for unit/integration tests:

```bash
# Small dataset for faster tests
cargo run --bin chalkbyte-cli -- seed -s 1 --admins 1 --teachers 1 --students 3
```

### Cleanup

Remove test data before committing or deploying:

```bash
cargo run --bin chalkbyte-cli -- clear-seed
```

## Notes

- The seeder preserves system administrators when clearing data
- Schools must have unique names (enforced by database constraint)
- All emails are suffixed with `@example.com` for easy identification
- User roles are correctly assigned with school associations
- The seeder can be run multiple times (will create new data each time)

## Integration with Development Workflow

```bash
# 1. Start fresh
cargo run --bin chalkbyte-cli -- clear-seed

# 2. Seed with test data
cargo run --bin chalkbyte-cli -- seed -s 3

# 3. Start the server
cargo run

# 4. Test with seeded data
# Login with any generated user (password: password123)

# 5. Clean up when done
cargo run --bin chalkbyte-cli -- clear-seed
```

## Implementation Details

### Dependencies

- `fake = { version = "4", features = ["derive", "chrono", "uuid"] }` - Fake data generation
- `rayon` - Parallel processing for data generation
- `bcrypt` - Password hashing (cost 4 for seeding performance)

### Modules

- `src/cli/seeder.rs` - Core seeder logic
- `src/bin/cli.rs` - CLI command handlers

### Performance Optimizations

1. **Parallel Data Generation (Rayon)**
   - Schools generated in parallel using `into_par_iter()`
   - User data generated in parallel across all CPU cores
   - Pre-allocated vectors with exact capacity

2. **Batch Database Inserts**
   - Schools: Batch size 500 per chunk
   - Users: Batch size 1000 per chunk
   - Multi-value INSERT statements reduce round-trips
   - Single transaction per batch for atomicity

3. **Password Hashing Strategy**
   - Hash password once with bcrypt cost 4 (~20ms)
   - Reuse hash for all users (same default password)
   - Avoids 24,000+ individual hash operations

4. **Query Optimization**
   - Pre-build parameterized queries
   - Use explicit type casts for enums
   - Minimize database round-trips

### Database Operations

The seeder uses SQLx to:
1. Generate fake data in parallel using Rayon
2. Insert schools in batches with single transaction
3. Hash password once and reuse for all users
4. Insert users in batches of 1000 with proper role enums
5. Maintain referential integrity (school_id foreign keys)
6. Provide detailed timing information for each step