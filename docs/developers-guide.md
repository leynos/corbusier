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

`make check-fmt` validates Rust formatting, `make lint` runs the workflow
contracts, documentation generation, Clippy, Whitaker, and the spelling policy
with warnings denied, and `make test` runs the workspace test suite through
nextest. Documentation changes should also run:

```sh
make markdownlint
make nixie
```

Run `make fmt` after documentation changes to apply Rust and Markdown
formatting, but review formatter output before committing. Markdown formatter
defects that affect this repository are tracked upstream in
[`leynos/mdtablefix`](https://github.com/leynos/mdtablefix).

`make fmt` and `make check-fmt` run `mdtablefix` (version 0.6.0, the same
release CI installs); install it once with
`cargo install --locked mdtablefix --version 0.6.0`. `make fmt` also runs
`markdownlint-cli2`, which CI provides through its GitHub action; locally
install it with `bun install -g markdownlint-cli2` (or `npm install -g`).

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
2. Otherwise, `MDLINT` resolves to `$HOME/.bun/bin/markdownlint-cli2`, the
   path a global `bun install -g markdownlint-cli2` creates; no existence check
   is made, so the shell reports the missing command when the target runs.

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

The lint and Markdown gates enforce British English in the Oxford `-ize` style
through `typos-config-builder gate`. Run it on its own with `make spelling`.
Every run regenerates `typos.toml` from the live shared dictionary and the
repository overlay, then checks the maintained Markdown, so the generated file
is never drift-checked in continuous integration.

Add repository-only proper names or quoted upstream terms to
`typos.local.toml`, which holds the en-GB-oxendict overlay; never edit
generated entries in `typos.toml` by hand.

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

Neither half of the gate may be silenced casually. An advisory with no
reachable fixed release is suppressed only through the mechanism for its
ecosystem, and only alongside a document that records the rationale, the
exposure, and the trigger for re-review:

- Node.js advisories are suppressed by an entry in
  `frontend-pwa/security/audit-exceptions.json`, which requires an expiry date
  and is enforced by `frontend-pwa/scripts/run-audit.mjs`.
- Rust advisories are suppressed by an `ignore` entry in `.cargo/audit.toml`,
  paired with a dependency-policy exception document under `docs/`. See
  [Dependency policy exception: h2 empty DATA frames](dependency-policy-exception-h2-empty-data-frames.md)
  for the current example.

## Coverage publication and CodeScene

CodeScene belongs to one workflow, `.github/workflows/coverage-main.yml`, which
runs on a push to `main` and on `workflow_dispatch`. No pull-request lane talks
to CodeScene. The command-line tool calls CodeScene's API at job time, and that
API has changed shape without notice: on 2026-09-21 the service stopped
returning a gates configuration, and every pull-request lane that ran
`cs-coverage check` went red for a reason no change here could have caused. On
a push lane the same failure delays a coverage upload instead of blocking a
merge.

The pull-request lane in `ci.yml` still measures coverage. Its
`generate-coverage` step sets `with-ratchet: 'true'`, so a drop below the
baseline fails the pull request, and `publish-artefact: 'false'`, because
nothing reads the report as an artefact. The publisher writes the ratchet
baseline on each push to `main`, and pull requests compare against it.

The publisher's shape is load-bearing in four places:

- A step with the id `codescene-token` runs exactly
  `echo "available=${{ secrets.CS_ACCESS_TOKEN != '' }}" >> "$GITHUB_OUTPUT"`,
  with no `if:` and no `env`. The expression is evaluated before the shell
  runs, so the token enters no process and no `env`. The uploader is a
  composite action that hands its step's `env` to nested steps, so the upload
  receives the token only as its `access-token` input.
- The upload runs only when
  `steps.codescene-token.outputs.available == 'true' && github.ref == 'refs/heads/main'`.
  A dispatch can name any branch, and the uploader checks neither ref nor
  event, so the ref test is what stops a feature branch's coverage being
  published as `main`'s.
- The concurrency group is `coverage-main-${{ github.ref }}` with
  `cancel-in-progress: false`. Runs never overlap, and a newer trigger replaces
  an older pending run; no commit order is promised, since GitHub does not
  promise to start runs in trigger order. A manual re-run keeps its run id, so
  it republishes that commit's coverage but replaces no baseline saved under a
  run-keyed cache key. A dispatch that replaces a pending push uploads the same
  or a newer commit, but `generate-coverage` saves the ratchet baseline only on
  a push, so the baseline then stays one commit behind until the next push.
- Merges made by the Dependabot automerge workflow use `GITHUB_TOKEN`, which
  fires no push event, so the publisher does not run for them. This is a known
  exception: dispatch the publisher by hand when such a merge needs fresh
  coverage.

The shared-actions pins for `generate-coverage` and `upload-codescene-coverage`
must descend from `a5765019`. From that revision the uploader installs the
CodeScene CLI from a committed manifest and refuses the retired
`installer-checksum` input; earlier revisions install `latest`, which no longer
resolves. The `CODESCENE_CLI_SHA256` repository variable fed only that input
and is not read anywhere.

### Workflow contracts

`make test-workflow-contracts` runs the pytest modules under
`tests/workflow_contracts/`, which assert the rule above against the workflow
files. They run as their own CI step before anything compiles, and as a
prerequisite of `make lint`, so deleting the step does not stop them running.

- `workflow_loader.py` is the only parser. It refuses a mapping that declares a
  key twice, because PyYAML otherwise keeps the last value silently, and it
  reads `.yml` and `.yaml` in any case.
- `workflow_calls.py` decides which `uses:` values run a checked-out workflow:
  a leading `./` or `$/` is stripped and the rest must name a file directly
  under `.github/workflows/`. A call to this repository's workflows at an
  `@ref` runs the file at that ref, which the checkout does not hold, so it is
  refused rather than followed.
- `codescene_placement_reader.py` reads triggers as a scalar, a sequence or a
  mapping, under both the `on` string key and the boolean YAML 1.1 makes of it,
  and refuses a workflow declaring both. It builds the pull-request closure:
  every workflow started by a pull-request event (`pull_request`,
  `pull_request_target`, `merge_group`, `workflow_run`, the review events and
  `issue_comment`) plus every local workflow those call, transitively. A
  `workflow_call`-only workflow called with `secrets: inherit` runs on the pull
  request with the token.
- `ci_codescene_placement_test.py` refuses, anywhere in the closure, a
  CodeScene action, a `cs-coverage` command, the token by any reference, the
  `codescene.io` host in any scalar (a workflow-level `defaults.run.shell`
  included), `secrets: inherit` into another repository, and a call to this
  repository by ref. It also requires the pull-request lane to keep its
  unconditional, ratcheted, unpublished coverage step.
- `codescene_publisher_test.py` requires the publisher's triggers, its `main`
  push filter, `mode: upload`, the token check and upload condition compared
  whole, the token in no `env`, a full-SHA uploader pin with no
  `installer-checksum`, and the concurrency group compared whole.
- `codescene_placement_policy.py` holds the reviewed values the tests compare
  against. Change a value there only with the workflow it describes.

Each reading is proved against constructed trees in the `*_test.py` modules
beside it. The contracts load no Rust and need only `uv`; the Makefile pins
pytest and PyYAML and disables bytecode writing.

## Cancelling superseded pull-request runs

Every push to a pull request starts a fresh run of each gate. The run already
in flight is answering a question about a commit nobody will merge, and left
alone it holds a runner until it finishes, so the branch pays twice for one
answer. Every workflow a pull request can start therefore carries a concurrency
block:

```yaml
concurrency:
  group: ${{ github.workflow }}-${{ github.event.pull_request.number || github.run_id }}
  cancel-in-progress: ${{ github.event_name == 'pull_request' }}
```

Two halves matter, and each fails in a way nothing else would notice.

- **The group keys on the pull request, and falls back to the run.** A
  constant group puts every open pull request in one queue, so the first push
  anywhere cancels the gates running everywhere else. Outside a pull request
  the number is empty and the group falls back to `github.run_id`. A shared
  fallback such as `github.ref` would let a third dispatch replace a pending
  second one that was meant to complete. The run id belongs in that fallback
  position alone: as the whole group it would match no predecessor and cancel
  nothing.
- **Cancellation is conditioned on the event.** A literal
  `cancel-in-progress: true` reads as the stricter setting and is a regression.
  A push to `main`, a schedule, and a dispatch have no successor waiting, and
  the run on `main` writes the warm cache and records the coverage that no
  later run repeats.

`pull_request_target` workflows are out of scope. They run against the base
repository to carry a token, and the ones here automate pull-request
housekeeping rather than building, so cancelling one mid-flight is a hazard
with no minutes to win.

### The cancellation contract

The rules live in `tests/workflow_contracts/concurrency_rules.py`, as functions
over a parsed workflow. `concurrency_test.py` holds this repository's workflows
to them: for every workflow a `pull_request` event starts, a concurrency group
is declared, the group is exactly the expression above, and
`cancel-in-progress` is exactly the expression above. Discovery reads triggers
through `codescene_placement_reader.triggers`, which refuses a missing `on:`, a
shape it cannot model and a workflow declaring both `on` keys, so no workflow
leaves discovery in silence. A floor test asserts that discovery still finds
`ci.yml`, so a broken read cannot empty the list and turn the rest into a
vacuous pass.

`concurrency_rules_test.py` drives the same functions with constructed
workflows: every trigger form under both key spellings, the refused shapes, and
each wrong group, shorthand block and unconditioned cancellation the rules must
reject. Run both with `make test-workflow-contracts`.
