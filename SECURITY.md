# Security Policy

## Reporting a Vulnerability

Do not open a public issue for an unpatched vulnerability.

Report suspected vulnerabilities privately through
[GitHub Security Advisories](https://github.com/OpenAgentID/oas-glyph/security/advisories/new).
If private vulnerability reporting is unavailable, email
**security@openagent.id**.

Include the affected version or commit, impact, reproduction steps, and a
minimal proof of concept when possible. Never include private keys, production
credentials, or personal data in a report.

We aim to acknowledge reports within 3 business days and provide a remediation
plan or a reasoned rejection within 14 days. We coordinate disclosure and
credit with the reporter after a fix is available.

## Supported Versions

Security fixes are applied to the latest released version and the `main`
branch. Older releases may require upgrading to receive a fix.

## Scope

Security-sensitive areas include:

- malformed glyphs that cause memory-safety, denial-of-service, or parser
  boundary failures;
- glyph generation or scanning that returns a DID different from the encoded
  identity without reporting an error;
- validation bypasses that accept malformed `did:oas` identities;
- unsafe image, multipart, or HTTP API handling; and
- dependency vulnerabilities with a reachable exploit path.

The glyph is a visual transport for an OAS identity, not an authority signal.
Reports that rely only on visual similarity, without an encoding or
verification failure, are generally out of scope.
