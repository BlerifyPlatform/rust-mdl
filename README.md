# rust-mdl

Rust client library for Blerify's Issuance API. Wraps the 3-step mdoc assembly
protocol (generate → sign → assemble) for ISO 18013-5 mobile documents.

Intended for Rust-based issuer backends: hides the multi-call handshake behind a
single `issue()` entry point, manages service account tokens against the
configured OIDC provider, and surfaces typed errors for the caller.

## Status

Early scaffolding — no functional code yet.

## Scope

- Service account authentication (`client_credentials`) with token caching.
- ISO 18013-5 mDL output (CBOR mdoc, COSE_Sign1, EC P-256).
- Typed errors that map Issuance API failures to actionable variants for the caller.

Not in scope: wallet-side presentation, W3C VC issuance, ISO 23220.

## Contributing

Branch names: `feature/`, `bugfix/`, `hotfix/`, `release/`, `chore/`, `test/`, `experiment/`.

Commit messages follow Conventional Commits: `type(scope): lowercase description`
where `type ∈ { feat, fix, chore, docs, style, refactor, test, build, perf, revert }`.

All commits must be signed (GPG, SSH, or S/MIME) and show as **Verified** on
GitHub. See [GitHub's signing docs](https://docs.github.com/en/authentication/managing-commit-signature-verification/about-commit-signature-verification).

Branch and commit conventions are enforced by `.github/workflows/pr-validations.yml` on every PR.
