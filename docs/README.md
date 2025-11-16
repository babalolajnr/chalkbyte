# Chalkbyte Documentation

Welcome to the Chalkbyte documentation. This directory contains all the technical and user documentation for the project.

## 📚 Documentation Index

### Getting Started

- **[SETUP_GUIDE.md](./SETUP_GUIDE.md)** - Complete setup walkthrough from scratch
  - Prerequisites
  - Database setup
  - Creating system admin
  - First school and admin setup
  - Troubleshooting

- **[QUICK_REFERENCE.md](./QUICK_REFERENCE.md)** - Quick command reference card
  - Common commands
  - API endpoints
  - Permission matrix
  - Database queries

### Core Concepts

- **[USER_ROLES.md](./USER_ROLES.md)** - Role system and permissions
  - Role hierarchy
  - Permissions matrix
  - User creation workflow
  - API endpoints by role

- **[AUTHENTICATION.md](./AUTHENTICATION.md)** - Authentication guide
  - JWT token structure
  - Login process
  - Protected routes
  - Token expiration

- **[MFA_QUICK_START.md](./MFA_QUICK_START.md)** - Multi-Factor Authentication Quick Start
  - 3-step MFA setup
  - Login with MFA
  - Recovery codes
  - Management commands

- **[MFA_GUIDE.md](./MFA_GUIDE.md)** - Complete MFA Documentation
  - TOTP authenticator apps
  - QR code enrollment
  - Recovery code system
  - API endpoints
  - Security considerations

### For Administrators

- **[QUICKSTART_SYSTEM_ADMIN.md](./QUICKSTART_SYSTEM_ADMIN.md)** - Quick start for system admins
  - Creating schools
  - Managing school admins
  - User management
  - Common operations

### Technical Details

- **[SYSTEM_ADMIN_IMPLEMENTATION.md](./SYSTEM_ADMIN_IMPLEMENTATION.md)** - Technical implementation
  - Architecture overview
  - Database schema
  - Authorization enforcement
  - Security features
  - API endpoint details

## 🚀 Quick Links

### For First-Time Users
1. Start with [SETUP_GUIDE.md](./SETUP_GUIDE.md)
2. Reference [QUICK_REFERENCE.md](./QUICK_REFERENCE.md) for commands

### For System Administrators
1. Read [USER_ROLES.md](./USER_ROLES.md) to understand the role system
2. Follow [QUICKSTART_SYSTEM_ADMIN.md](./QUICKSTART_SYSTEM_ADMIN.md) for daily operations

### For Developers
1. Check [SYSTEM_ADMIN_IMPLEMENTATION.md](./SYSTEM_ADMIN_IMPLEMENTATION.md) for technical details
2. See [AUTHENTICATION.md](./AUTHENTICATION.md) for auth implementation
3. Review `.github/copilot-instructions.md` for coding guidelines

## 🔑 Key Concepts

### Role Hierarchy
```
System Admin (CLI-created)
    ↓
Schools + School Admins
    ↓
Teachers + Students
```

### Security Model
- ✅ No public registration
- ✅ CLI-only system admin creation
- ✅ School isolation for admins
- ✅ Role-based authorization
- ✅ JWT authentication
- ✅ Multi-Factor Authentication (MFA) support

## 📖 Interactive API Documentation

When the server is running, you can access interactive API documentation at:

- **Swagger UI**: http://localhost:3000/swagger-ui
- **Scalar UI**: http://localhost:3000/scalar

## 🔐 MFA Frontend Example

For a complete MFA enrollment UI example, open:
- **HTML Example**: `docs/mfa_frontend_example.html`

## 🆘 Getting Help

1. Check the relevant documentation file above
2. Review the troubleshooting sections in setup guides
3. Check server logs for detailed error messages
4. Inspect database state with SQL queries (see QUICK_REFERENCE.md)

## 📝 Documentation Updates

When updating documentation:
- Keep examples up-to-date with actual API
- Update permission matrices when roles change
- Add new CLI commands to QUICK_REFERENCE.md
- Update SYSTEM_ADMIN_IMPLEMENTATION.md for technical changes

---

**Last Updated**: 2025-11-12  
**Version**: 1.0.0
