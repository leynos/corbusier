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


### Markdown linting

Markdown linting uses
[`markdownlint-cli2`](https://github.com/DavidAnson/markdownlint-cli2). The
Makefile target is:

```shell
make markdownlint
```

The `MDLINT` variable resolves the executable automatically. If
`markdownlint-cli2` is installed under `~/.bun/bin/` (for example via
`bun install --global markdownlint-cli2`), the Makefile uses that copy. If not,
it falls back to whatever `markdownlint-cli2` is on `PATH`. To override, set
`MDLINT` explicitly:

```shell
MDLINT=/path/to/markdownlint-cli2 make markdownlint
```

Install via Bun (recommended for contributors who already use Bun):

```shell
bun install --global markdownlint-cli2
```

Or via npm:

```shell
npm install --global markdownlint-cli2
```
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
