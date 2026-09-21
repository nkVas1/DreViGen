# Security Policy

## Reporting a vulnerability

**Do not open a public issue.**

Use GitHub's private vulnerability reporting:
[**Report a vulnerability**](https://github.com/nkVas1/DreViGen/security/advisories/new).

If that is unavailable to you, open an issue titled "Security contact request" with no details,
and a private channel will be arranged.

### What to include

- What the vulnerability is, and what an attacker gains.
- Steps to reproduce, or a proof of concept.
- Affected version, platform and deployment mode (local-only, or with a sync server).
- Whether it is already public anywhere.

### What to expect

| | |
|---|---|
| Acknowledgement | Within 72 hours |
| Initial assessment | Within 7 days |
| Fix or mitigation plan | Within 30 days for high severity |
| Credit | Offered in the advisory and the changelog, unless you prefer otherwise |

This is a project maintained by one person. Those targets are honest intentions, not a
contractual SLA, and they will be communicated if they slip.

## Supported versions

Pre-1.0: only the latest release receives fixes. From 1.0, the current minor and the one before
it.

## Scope

**In scope**

- The desktop, mobile and web applications.
- The self-hosted sync server (`apps/server`) and its default deployment configuration.
- The sync and contribution protocol.
- Anything that lets one tree member read or modify data they were not granted.
- Any path by which data about a living person escapes its configured privacy level — including
  through GEDCOM export, web publish, print output or file metadata. **This is treated as high
  severity regardless of how obscure the path is.**
- Dependency vulnerabilities that are actually reachable from our code.

**Out of scope**

- Self-hosted instances misconfigured contrary to the documentation — though if the
  documentation makes the insecure configuration easy, that *is* in scope and worth reporting.
- Social engineering of tree members.
- Denial of service against a self-hosted instance by someone already granted access.
- Vulnerabilities in a user's own optional external AI provider.

## Security posture

| Area | Measure |
|---|---|
| Authentication | Passkeys (WebAuthn) first; emailed magic link as fallback. No passwords stored, because none exist. |
| Authorisation | Enforced server-side on every endpoint. The client's role is a rendering hint and never a gate. |
| Transport | TLS only, HSTS, certificates automated by Caddy. |
| At rest | Optional per-tree local database encryption; server-side media encryption for trees marked private. |
| Living persons | Redaction applied at the serialisation boundary, so no export path can bypass it. |
| Uploads | Content-type sniffing, size limits, EXIF stripped from published derivatives. Originals retain their metadata and stay private. |
| Supply chain | Exact pinned versions, committed lock files, `cargo audit` and `pnpm audit` blocking in CI, Dependabot, no dependency outside MIT / Apache-2.0 / BSD / OFL. |
| Secrets | Environment variables only. `.env` git-ignored, secret scanning enabled on the repository. |
| Local data | A tree is a file on the user's machine. It is theirs. Nothing is transmitted anywhere without an explicit, user-initiated sync. |

## A note on threat model

DreViGen holds material that is unusually sensitive in a way that is easy to underestimate:
names, birth dates and places of living relatives; genetic relationships; religious affiliation
and ethnicity inferred from historical records; and in post-Soviet archival research, records of
repression, deportation and military service.

This data is a gift from the family to whoever maintains the tree. Treat a privacy leak as a
correctness bug of the highest severity, not as a feature gap.
