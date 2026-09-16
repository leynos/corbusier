# Developer's Guide

This guide collects the main documents for contributors working on Whitaker.

Start here:

- [Whitaker user's guide](whitaker-users-guide.md) for installation and
  consumer-facing configuration
- [User's guide](users-guide.md) for the broader Corbusier project context
- [Documentation style guide](documentation-style-guide.md) for Markdown
  conventions used in this repository
- [Roadmap](roadmap.md) for current delivery sequencing and scope

For architecture and implementation details, read the relevant ADRs and
execplans under [`docs/`](.).

## Tooling

## Maintainer requirements

This section records internal development requirements that contributors must
apply when changing Corbusier. The canonical agent instructions remain in
[`AGENTS.md`](../AGENTS.md); this guide summarizes the requirements that affect
day-to-day implementation choices.

The following sections intentionally mirror the development rules added to
`AGENTS.md`:

- Internal tooling and build gates are documented in
  [quality gates](#quality-gates), [additional tools](#additional-tools), and
  [dependency audit](#dependency-audit).
- Dependency-management requirements are documented in
  [dependency management](#dependency-management).
- Error-handling expectations are documented in
  [error handling](#error-handling).
- Observability and instrumentation rules are documented in
  [observability](#observability).
- Abstraction, port, and helper policy is documented in
  [abstractions, ports, and helpers](#abstractions-ports-and-helpers).

### Quality gates

Run the repository gates before committing code changes:

```sh
make check-fmt
make lint
make test
```

`make check-fmt` validates Rust formatting, `make lint` runs documentation
generation, Clippy, Whitaker, and the spelling policy with warnings denied, and
`make test` runs the workspace test suite through nextest. Documentation
changes should also run:

```sh
make markdownlint
make nixie
```

Run `make fmt` after documentation changes to apply Rust and Markdown
formatting, but review formatter output before committing. Markdown formatter
defects that affect this repository are tracked upstream in
[`leynos/mdtablefix`](https://github.com/leynos/mdtablefix).

### Abstractions, ports, and helpers

Before adding a new abstraction, hexagonal port, or extracted helper, sweep the
repository for an existing equivalent. If a new abstraction is still needed,
document its intended scope, ownership boundary, permitted call-sites, and
composition rules in the relevant design or maintainer document.

Use [`contents.md`](contents.md) to choose the right documentation destination.
Substantive architectural decisions belong in an ADR or the relevant design
document, not only in code comments.

### Dependency management

Cargo dependencies must use explicit SemVer-compatible caret requirements such
as `some-crate = "1.2.3"`. Avoid wildcard requirements and open-ended
inequality requirements because they make builds unpredictable. Use tilde
requirements only when patch-level locking is required for a documented reason.

When adding a dependency, update the relevant design or developer documentation
if the dependency changes architecture, runtime behaviour, operator workflow,
or contributor tooling. Run the dependency audit gate when the change affects
Rust or frontend dependency surfaces:

```sh
make audit
```

### Error handling

Use semantic error enums for library and domain errors that callers may
inspect, retry, or map to an HTTP status. Derive `std::error::Error` with
`thiserror` for those cases. Reserve opaque reports such as `eyre::Report` for
application boundaries, logs, and top-level task entry points.

Tests should prefer `.expect(...)` over `.unwrap()` for clearer diagnostics.
Production code and shared fixtures should return `Result` and propagate errors
with `?` instead of panicking. Keep the `expect_used` lint strict, and remember
that test-only lint allowances do not apply to helpers outside `#[cfg(test)]` or
`#[test]`.

Fallible `rstest` fixtures should be consumed by tests that return `Result`, so
fixture setup errors can be propagated with `?`.

### Observability

Use `tracing` for application diagnostics. Prefer structured
`tracing::{trace, debug, info, warn, error}` events and spans over `println!`,
`eprintln!`, or direct `log` macros. Include stable identifiers, state, and
error context as structured fields so subscribers can filter and correlate
events without parsing message text.

Instrument request handling, command execution, retries, background jobs, and
other meaningful work units with `#[tracing::instrument]` or explicit spans. Do
not hold `Span::enter()` guards across `.await`; use `Instrument::instrument`
or scoped synchronous spans instead.

Emit metrics through the `metrics` crate where usage, uptake, failure, or
mitigation metrics are required. Use low-cardinality labels and avoid user
input, request identifiers, raw paths with unbounded parameters, or raw error
strings as labels. Libraries may emit `metrics` and `tracing` instrumentation,
but applications own global exporter and subscriber initialization.

### Additional tools

Common local tools include `mbake` for Makefile validation, `shellcheck` for
shell scripts, `difft` for structural diffs, `srgn` for structural search,
`hyperfine` for benchmarking, and system inspection tools such as `strace`,
`ltrace`, `gdb`, `lldb`, `lsof`, `htop`, `iotop`, and `ncdu`. Prefer repository
`make` targets when they exist, and use direct tool invocations for diagnosis
or focused checks that do not have a Makefile wrapper.

### Frontend task slice tooling

The repository-owned frontend lives under `frontend-pwa/`. The task slice uses
an explicit port-and-adapter boundary, so route components do not import
transport code directly.

- `TaskGatewayProvider` provides a `TaskSliceGateway` implementation to React
  components. Mount it through `AppProviders` for application and route tests
  unless a unit test is exercising the context boundary directly.
- `useTaskGateway` retrieves the current gateway and throws
  `Task gateway provider is missing.` when no provider is mounted. Hook and
  route tests should assert this failure mode when they bypass `AppProviders`.
- `TaskNotFound` renders the localized task-detail not-found state and links
  back to `/tasks/new`. It contains no data fetching; `TaskDetailPage` owns the
  gateway error mapping.
- `createFixtureTaskGateway` is the default local and test adapter. It keeps
  task state in memory, serializes queued operations over that shared state,
  and preserves fixture-first development until the live HTTP gateway is
  selected in later roadmap work.

The shipped `4.4.1` slice boundary is recorded in
[`corbusier-design.md`](corbusier-design.md#713-repository-owned-frontend-workspace-boundary).
The Whitaker user's guide remains a tooling guide for the Rust lint runner and
does not describe these frontend-only APIs.

### TypeScript type checking for tests

`frontend-pwa/tsconfig.test.json` extends the base `tsconfig.json` and widens
the checked surface to include `src/`, `tests/`, `vite.config.ts`,
`vitest.config.ts`, `vitest.a11y.config.ts`, and `playwright.config.ts`. The
`make frontend-typecheck` target runs `bun run typecheck`, which executes both
`tsc --noEmit` for application types and
`tsc --project tsconfig.test.json --noEmit` for test and tooling types.

The separate test pass is required because tests and test configuration use
different inclusion patterns and ambient types from the production application
sources, while still depending on the application `src/` tree.

`frontend-pwa/tests/types/jest-axe.d.ts` provides ambient TypeScript
declarations for the `jest-axe` package because the package does not ship
bundled types. It exports `AxeRunner`, `configureAxe`, `axe`, and
`toHaveNoViolations`; the test-scoped validation keeps those declarations
checked alongside the accessibility test setup.

`frontend-pwa/tests/setup-vitest-a11y.ts` registers `toHaveNoViolations` on both
`@vitest/expect`'s `Assertion<T>` and `vitest`'s `Assertion<T>` interfaces.
Tests that call `axe(container)` and then
`expect(results).toHaveNoViolations()` rely on this setup file being listed in
the accessibility Vitest configuration's `setupFiles`.

## Frontend task slice APIs

### HTTP API test helpers

Rust HTTP API integration tests share helpers in
`tests/http_api_test_helpers.rs`.

- `HttpApiAuth` creates JSON Web Tokens (JWTs) and matching request contexts
  for Actix HTTP API tests.
- `BearerToken` is a raw bearer-token string wrapper produced by
  `HttpApiAuth`. It does not validate token syntax or claims; validation is
  performed by the HTTP auth layer under test.

### Markdown linting

Markdown linting uses
[`markdownlint-cli2`](https://github.com/DavidAnson/markdownlint-cli2). Run the
linting target with:

```shell
make markdownlint
```

The `MDLINT` variable resolves the executable automatically:

1. If `markdownlint-cli2` is found on `PATH`, that executable is used.
2. Otherwise, if `~/.bun/bin/markdownlint-cli2` exists and is executable, that
   fallback is used.
3. If neither lookup succeeds, `MDLINT` resolves to `markdownlint-cli2` and the
   shell reports the missing command when the target runs.

Override the resolved path explicitly if needed:

```shell
MDLINT=/path/to/markdownlint-cli2 make markdownlint
```

Install via Bun (recommended if Bun is already in use):

```shell
bun install --global markdownlint-cli2
```

Or via npm:

```shell
npm install --global markdownlint-cli2
```

### Spelling policy

The lint and Markdown gates run `typos` 1.48.0 with British English and Oxford
`-ize` conventions. Before checking maintained Markdown, the generator
refreshes the shared estate dictionary into an untracked local cache only when
the authority is newer, then merges `typos.local.toml`. The generated
`typos.toml` is reviewed and committed so a clean, network-restricted checkout
can still enforce the last known-good policy.

Add repository-only proper names or quoted upstream terms to
`typos.local.toml`; never edit generated entries in `typos.toml` by hand. The
spelling gate also runs the helper's Python 3.13 tests with at least 90% line
coverage.

### `TaskGatewayProvider` and `useTaskGateway`

Defined in `frontend-pwa/src/task_slice/application/task-gateway-context.tsx`.

`TaskGatewayProvider` injects a `TaskSliceGateway` implementation into React
context. Wrap the component tree with it in tests and in `AppProviders` to
supply the gateway:

```tsx
<TaskGatewayProvider gateway={myGateway}>
  <App />
</TaskGatewayProvider>
```

`useTaskGateway` retrieves the gateway from context. Call it inside any
component that belongs to the task slice. It throws with the message
`"Task gateway provider is missing."` if no provider ancestor is present.

### `TaskNotFound`

Defined in `frontend-pwa/src/task_slice/ui/task-not-found.tsx`.

Presentational component rendered by the task-detail route when the gateway
reports a `not_found` error. It displays a localized heading, body text, and a
navigation link back to `/tasks/new`. It carries no data-fetching or
error-boundary logic.

## Test utilities

### `BearerToken` (Rust integration tests)

Defined in `tests/http_api_test_helpers.rs`.

A newtype wrapper around `String` that represents a bearer token produced by
`HttpApiAuth::token()`. Use `BearerToken::as_str()` to obtain a `&str` suitable
for passing to `with_bearer`:

```rust
let token = BearerToken(auth.token()?);
let request = with_bearer(TestRequest::get().uri("/api/v1/tasks"), token.as_str());
```

Using `BearerToken` rather than a bare `&str` reduces string-argument
saturation and makes the intended role of each parameter unambiguous at call
sites.

## Dependency audit

The workspace ships a unified dependency-vulnerability gate. Run it with:

```sh
make audit
```

`make audit` runs both `make audit-node` (Bun/Node.js) and `make rust-audit`
(Cargo) in sequence. Either sub-target may be invoked individually.

`rust-audit` requires `cargo-audit` to be installed. Install it with:

```sh
cargo binstall cargo-audit
```

`cargo-audit` is installed automatically in CI via the workflow at
`.github/workflows/ci.yml`.

`make audit` runs its two halves as Make prerequisites, so a failing
`audit-node` stops the target before `rust-audit` runs. A green frontend audit
is therefore the only state in which the Rust half has been read at all, and a
newly cleared frontend advisory can reveal a Rust one that was there all along.

### Clearing a frontend advisory

`make audit-node` runs `frontend-pwa/scripts/run-audit.mjs`, which fails on any
advisory `bun audit` reports that the exception ledger at
`frontend-pwa/security/audit-exceptions.json` does not cover. Clear an advisory
in this order, and stop at the first step that works:

1. Refresh the lockfile. `bun update <package>` moves a transitive dependency
   inside the range its parent already allows, and changes no manifest.
2. Add or raise an entry in `overrides` in `frontend-pwa/package.json`. The
   floor named there is a security floor: it exists to keep the resolved
   version above the advisory's fixed version, so lowering one reopens the
   advisory it closed.
3. Only where neither works, add a ledger entry. Each carries an `expiresAt`
   date and a justification, and the gate fails once that date passes, so an
   accepted risk cannot sit there indefinitely. An entry is a deferral, not a
   fix.

Raising a declared dependency across a major version to clear an advisory is a
separate change with its own review; it is not part of clearing the advisory.

## Workflow contracts

The workflow files carry rules that nothing else can enforce. A pull
request changing a workflow is reviewed by reading it, and each of the
three rules below exists because reading it was not enough. Run them
with:

```sh
make workflow-contracts
```

CI runs the same target, as an unguarded step in `ci.yml`, and one of the
contracts asserts that: a contract nothing runs is a comment. The
assertion names the command rather than the step's name, because a step
keeps its name when its `run:` changes.

### One commit, and not a wrapper-less one

Every `leynos/shared-actions` reference in every workflow file names one
40-hex commit, and never one of the named pins whose `setup-rust`
exports no `RUSTC_WRAPPER`.

Both halves come from a real proposal. Dependabot's #167 moved every
reference in this repository to `57a33fa6`, a commit 37 commits behind
the one they should be on, in a routine group bump. At that pin
`setup-rust` installs sccache and exports no wrapper, so Cargo routes no
compilation through it: the cache is downloaded on every Rust job and
caches nothing, while the pull request reads as pins being brought up to
date. Before this change the references sat on two commits two months
apart, and nothing said so.

The references are matched by repository prefix rather than by an
enumerated list of action paths. An enumeration goes stale the moment a
workflow adopts another action from the same repository, and the
reference it misses is the one nobody thought to add. Both spellings are
read, a step's `uses:` and a job's, because the Dependabot automerge
caller is a reusable workflow and has to move with the rest.

### The coverage watchdog is stated, not inherited

Every job invoking `generate-coverage` declares
`RUN_RUST_CARGO_WAIT_TIMEOUT` at job level, and the contract pins the
value at 1,800 s rather than accepting any value. That is the action's
own default, so the declaration changes no behaviour today; what it
changes is that a later change to the action's default cannot move this
repository's budget silently. The variable takes precedence over the
action's `cargo-wait-timeout` input, which is why it is set as an
environment variable.

The value is 1,800 s against a measured worst case of 479 s for the
coverage step, across the six most recent runs of both lanes. A blank
declaration is refused as well as a missing one: `VAR: ""` parses to the
empty string and a valueless `VAR:` parses to `None`, and both mask the
outer scope.

### No runner placement carries a line break

A folded scalar whose continuation line is indented more deeply than its
first line keeps the line break rather than folding it into a space. A
`runs-on` written that way carries a newline inside the expression, and
GitHub evaluates it as written, so the run is green and nothing says
otherwise. The contract reads every job's `runs-on` from the parsed
document and refuses a line break anywhere inside the value, including
inside a list of labels or a `group`/`labels` mapping, and reports the
raw declaration beside it.

Every job here names a literal runner today, so this rule cannot be
proved by these files: parametrized over four correct documents it would
pass whether or not it discriminated anything. It is written as a
function over a parsed value and driven directly in
`scripts/tests/test_runner_placement_rule.py`, with the fork-fallback
lane folded correctly, the same lane indented one level deeper, and the
three shapes GitHub accepts. Refusing what the rule exists to permit is
as much a failure as accepting what it exists to refuse, and both
directions are asserted.
