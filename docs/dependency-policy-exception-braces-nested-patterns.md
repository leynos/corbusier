# Dependency policy exception: `braces` nested patterns (GHSA-vfj7-8cjw-p6xm)

Corbusier's frontend audit gate (`make audit-node`, which wraps `bun audit`
through `frontend-pwa/scripts/run-audit.mjs`) fails the build for any advisory
that `frontend-pwa/security/audit-exceptions.json` does not cover. This
document records a **time-bound exception** for one advisory that has no
patched release.

Recorded on 2026-10-09.

## Advisory

| Field            | Value                                                                    |
| ---------------- | ------------------------------------------------------------------------ |
| Identifier       | [GHSA-vfj7-8cjw-p6xm](https://github.com/advisories/GHSA-vfj7-8cjw-p6xm) |
| Package          | `braces`                                                                 |
| Resolved version | 3.0.3                                                                    |
| Severity         | High                                                                     |
| Class            | Denial of service by stack exhaustion on deeply nested brace patterns    |
| Affected range   | `<=3.0.3`                                                                |
| Fixed in         | No release                                                               |

Table 1: advisory covered by this exception.

## Why the advisory cannot be fixed

`braces` 3.0.3 is the newest release on the registry, and the advisory covers
it, so no upgrade or override reaches a patched version. Every path to it is a
development-time route rooted in `stylelint`:
`stylelint > fast-glob > micromatch > braces`,
`stylelint > micromatch > braces`, and a route through `globby > fast-glob`.
Replacing `stylelint` or its glob stack is out of scope for an audit fix.

## Exposure

- The chain runs only in development and continuous integration (CI) when
  `stylelint` expands file globs. It is not part of the built frontend and
  ships to no end user.
- The brace patterns come from `stylelint` and repository configuration, not
  from runtime input. A pull request can change that configuration and make its
  own CI run exhaust the stack and fail, but no deployed service is reachable
  through it and no secret is read by the affected process.

The exposure is therefore a self-inflicted CI failure in a change under review,
which the reviewer would see, rather than a production risk.

## Review trigger

The ledger entry `BRACES_NESTED_PATTERN_DOS_2026_10` in
`frontend-pwa/security/audit-exceptions.json` expires on 2026-12-09. The runner
treats a date-only expiry as inclusive, so `run-audit.mjs` rejects the entry,
and fails the gate, from 2026-12-10 00:00 UTC. Remove the entry and this
document when either becomes true:

- a patched `braces` release exists and the lockfile resolves it, or
- `stylelint` (or `fast-glob` and `micromatch`) no longer depends on `braces`
  3.

Review the exception at expiry even if neither has happened.

The same advisory is recorded for wildside in its ledger (wildside #529).
