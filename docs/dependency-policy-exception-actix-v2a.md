# Dependency policy exception: `actix_v2a` (Git revision)

Corbusier’s AGENTS mandate caret SemVer crate requirements and prohibit ad hoc
Git dependencies. This document records an **explicit, reviewer-approved**
exception until `actix_v2a` ships a crates.io release that meets the milestone’s
requirements.

## Rationale

The task slice milestone (`4.4.2`) standardizes slice-facing errors,
`Trace-Id`/trace payload handling, and `Idempotency-Key` parsing on that shared
crate. A published crates.io package was not yet available at integration time,
so Corbusier pins a **specific reviewed commit** rather than drifting on a bare
branch.

## Pinned artefact

- **Repository**: `https://github.com/leynos/actix-v2a.git`
- **Revision (full SHA)**: `7cc8d8c7aff4fcc333f6cf38a81207b1e27fe8fe`

Cargo declaration (see root `Cargo.toml`):

```toml
actix_v2a = { git = "https://github.com/leynos/actix-v2a.git", rev = "7cc8d8c7aff4fcc333f6cf38a81207b1e27fe8fe" }
```

## Steady state

When a suitable crates.io SemVer release exists, replace the Git dependency with
a caret requirement (for example `actix_v2a = "0.x.y"`) and remove this
exception file or mark it superseded.
