# Security Policy

## Supported versions

| Version | Supported |
|---------|-----------|
| 0.3.x  | ✅ |
| < 0.3  | ❌ |

## Reporting a vulnerability

Please report security issues privately rather than opening a public issue.

- Use GitHub's **Report a vulnerability** button on the Security tab of this
  repository (this opens a private advisory)

You can expect an initial response within 72 hours. Once a fix is ready, a
patch release will be published and you will be credited in the changelog
unless you prefer to stay anonymous.

## Scope

**In scope**

- Any code path that opens a source device, image or file for writing
- The read-only guarantee (`src-tauri/src/read_only.rs`) being bypassed
- Path traversal in the recovery output directory
- Arbitrary file write during recovery
- Unsafe handling of malformed filesystem metadata or carved signatures
- Command injection through the `forsmost` sidecar invocation

**Out of scope**

- Data loss caused by SSD TRIM — the app warns about this explicitly
- Users attempting to recover files they are not authorised to access
- Vulnerabilities in upstream `unearth`, `forsmost` or Tauri itself
  (report those to the respective maintainers)

## The one rule

PhoenixRecover must **never write to a source device**. Any finding that shows
a write path to a source is treated as critical.
