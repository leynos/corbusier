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


### Frontend task slice tooling

The repository-owned frontend lives under `frontend-pwa/`. The task slice uses
an explicit port-and-adapter boundary so route components do not import
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
[`markdownlint-cli2`](https://github.com/DavidAnson/markdownlint-cli2).
Run the linting target with:

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

### `TaskGatewayProvider` and `useTaskGateway`

Defined in
`frontend-pwa/src/task_slice/application/task-gateway-context.tsx`.

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
reports a `not_found` error. It displays a localised heading, body text, and
a navigation link back to `/tasks/new`. It carries no data-fetching or
error-boundary logic.

## Test utilities


### `BearerToken` (Rust integration tests)

Defined in `tests/http_api_test_helpers.rs`.

A newtype wrapper around `String` that represents a bearer token produced by
`HttpApiAuth::token()`. Use `BearerToken::as_str()` to obtain a `&str`
suitable for passing to `with_bearer`:

```rust
let token = BearerToken(auth.token()?);
let request = with_bearer(TestRequest::get().uri("/api/v1/tasks"), token.as_str());
```

Using `BearerToken` rather than a bare `&str` reduces string-argument
saturation and makes the intended role of each parameter unambiguous at
call sites.
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
