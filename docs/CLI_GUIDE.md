# Chalkbyte CLI Guide

The Chalkbyte CLI is a separate binary for administrative tasks. It supports both interactive and non-interactive modes, making it flexible for different use cases.

## Installation

The CLI is built automatically with the project:

```bash
cargo build --bin chalkbyte-cli
```

Or build in release mode for production:

```bash
cargo build --release --bin chalkbyte-cli
```

## Commands

### create-sysadmin

Create a new system administrator account. This is the only way to create system admins for security reasons.

#### Interactive Mode

Run without arguments to be prompted for each field:

```bash
# Using cargo
cargo run --bin chalkbyte-cli -- create-sysadmin

# Using built binary
./target/debug/chalkbyte-cli create-sysadmin

# Using justfile
just create-sysadmin-interactive
```

The interactive mode will prompt for:
- First name
- Last name
- Email address
- Password (hidden input with confirmation)

#### Non-Interactive Mode

Provide all arguments for scripting or automation:

```bash
# Long form
cargo run --bin chalkbyte-cli -- create-sysadmin \
  --first-name John \
  --last-name Doe \
  --email john@example.com \
  --password secure123

# Short form
cargo run --bin chalkbyte-cli -- create-sysadmin \
  -f John \
  -l Doe \
  -e john@example.com \
  -p secure123

# Using justfile
just create-sysadmin John Doe john@example.com secure123
```

#### Mixed Mode

Provide some arguments and be prompted for the rest:

```bash
# Provide names, prompted for email and password
cargo run --bin chalkbyte-cli -- create-sysadmin \
  --first-name John \
  --last-name Doe

# Provide email, prompted for names and password
cargo run --bin chalkbyte-cli -- create-sysadmin \
  --email john@example.com
```

This is useful when:
- You want to avoid storing passwords in shell history
- You want to specify some fields but interactively enter sensitive data
- You're partially automating the process

## Options

All options are optional. If not provided, you'll be prompted interactively.

| Option | Short | Description |
|--------|-------|-------------|
| `--first-name` | `-f` | First name of the system admin |
| `--last-name` | `-l` | Last name of the system admin |
| `--email` | `-e` | Email address |
| `--password` | `-p` | Password (prompted securely if omitted) |

## Security Best Practices

1. **Use Interactive Mode for Passwords**: When creating admins manually, use interactive mode or omit the `--password` flag to avoid storing passwords in shell history:
   ```bash
   cargo run --bin chalkbyte-cli -- create-sysadmin -f John -l Doe -e john@example.com
   ```

2. **Environment Variables**: For automation, consider reading passwords from environment variables:
   ```bash
   cargo run --bin chalkbyte-cli -- create-sysadmin \
     -f Admin -l User -e admin@example.com -p "$ADMIN_PASSWORD"
   ```

3. **Secure Scripts**: If using in scripts, ensure the script file has appropriate permissions (e.g., `chmod 700 script.sh`)

## Examples

### Quick Interactive Creation
```bash
just create-sysadmin-interactive
# Enter details when prompted
```

### Scripted Creation with Secure Password
```bash
#!/bin/bash
read -sp "Enter password: " password
echo
cargo run --bin chalkbyte-cli -- create-sysadmin \
  -f System -l Admin \
  -e admin@school.com \
  -p "$password"
```

### Docker Environment
```bash
docker compose exec app ./target/release/chalkbyte-cli create-sysadmin
```

## Help

Get help at any time:

```bash
# General help
cargo run --bin chalkbyte-cli -- --help

# Command-specific help
cargo run --bin chalkbyte-cli -- create-sysadmin --help
```

## Troubleshooting

### "DATABASE_URL must be set"
Ensure your `.env` file exists and contains `DATABASE_URL`, or set it manually:
```bash
export DATABASE_URL=postgresql://user:password@localhost/chalkbyte_db
cargo run --bin chalkbyte-cli -- create-sysadmin
```

### "Failed to connect to database"
- Ensure PostgreSQL is running: `docker compose up -d postgres`
- Check your DATABASE_URL is correct
- Verify network connectivity to the database

### "User with this email already exists"
The email address is already registered. Use a different email or remove the existing user from the database.
