# Security Policy

## Reporting a Vulnerability

**DO NOT** publicly disclose security vulnerabilities.

If you discover a security issue in AeroDB, please report it privately:

### How to Report

1. **Email**: Send details to the maintainers
2. **Include**:
   - Description of the vulnerability
   - Steps to reproduce
   - Potential impact
   - Suggested fix (if any)

### What to Expect

- **Acknowledgment**: Within 48 hours
- **Investigation**: We'll investigate and confirm the issue
- **Fix Timeline**: Critical issues fixed within 7 days
- **Public Disclosure**: After fix is released

### Vulnerability Types We Care About

**Critical:**
- Data loss or corruption
- Authentication bypass
- Unauthorized data access
- Denial of service (crash-inducing)
- Code execution vulnerabilities

**Medium:**
- Information disclosure
- Session hijacking
- CSRF attacks

**Low:**
- Minor information leakage
- Best practice deviations

## Supported Versions

| Version | Supported |
|---------|-----------|
| 0.1.x   | ✅ Yes    |

## Security Features

### Authentication
- Argon2 password hashing
- JWT-based session management
- Configurable token expiration
- Password complexity requirements

### Data Protection
- WAL-backed durability
- Authenticated write confirmation
- Crash-safe recovery
- Schema validation

### Network Security
- CORS protection
- HTTPS recommended for production
- Signed URLs for storage access

## Best Practices

### Deployment

1. **Use HTTPS**: Always run behind TLS in production
2. **Strong Passwords**: Enforce minimum 12-character passwords
3. **Regular Backups**: Automated snapshots
4. **Update Promptly**: Apply security patches immediately
5. **Least Privilege**: Grant minimal necessary permissions

### Configuration

```bash
# Strong JWT settings
JWT_EXPIRY=24h  # Or less
REFRESH_EXPIRY=7d

# Password requirements
PASSWORD_MIN_LENGTH=12
REQUIRE_UPPERCASE=true
REQUIRE_NUMBER=true
REQUIRE_SPECIAL=true
```

### Monitoring

- Enable audit logging
- Monitor failed authentication attempts
- Track unusual query patterns
- Alert on permission escalation attempts

## Known Limitations

- **No Built-in Encryption at Rest**: Use filesystem-level encryption (LUKS/dm-crypt)
- **Basic RBAC**: Row-level security in development
- **Rate Limiting**: Implement at reverse proxy level

## Security Updates

Security patches are released as soon as fixes are validated. Subscribe to:
- GitHub Security Advisories
- Release notifications

## Responsible Disclosure

We follow responsible disclosure practices:
1. Private report received
2. Fix developed and tested
3. Patch released
4. Public disclosure with credit

## Credits

We appreciate security researchers who help make AeroDB safer. Responsible disclosures will be credited in:
- CHANGELOG.md
- Security advisory
- Hall of fame (coming soon)

---

**Last Updated**: 2026-02-07
