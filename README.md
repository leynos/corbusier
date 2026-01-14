# Corbusier

**The "best room," not the smartest agent.**

Corbusier is an AI agent orchestration platform that lets multiple specialised
AI agents work together seamlessly. Rather than forcing one agent to do
everything, Corbusier owns the workflow—managing conversations, tools, safety
policies, and version control—while different AI backends handle what they do
best.

## The Big Picture

Orchestrating multiple AI agents is hard. Each one speaks a different language,
has different safety rules, and keeps its own state. Without a coordinator, you
end up with fragmented workflows, lost context, and duplicate logic everywhere.

Corbusier is that coordinator. It sits between your agents and the outside
world, translating between them, enforcing consistent policies, and making sure
conversations stay coherent even as work moves from one specialist agent to
another.

**We're not trying to build the smartest agent. We're building the best team
lead.**

## What Corbusier Does

### Conversation and Task Orchestration

- Maintains canonical conversation history across agent handoffs
- Tracks tasks from creation through completion with full audit trails
- Coordinates turn execution between different agent backends (Claude Code,
  Codex, etc.)
- Preserves context so agents don't lose track of what they're doing.

### Unified Tool Surface
- Hosts MCP (Model Context Protocol) servers with consistent discovery and
  routing
- Enforces Weaver as the authoritative file editor for all changes
- Translates tool schemas per backend, so every agent speaks the same language
- Provides encapsulated workspaces (via Podbot) for safe tool execution

### Safety Policies and Governance

- Executes configurable hooks for commit, merge, and deploy events
- Enforces consistent quality gates across different AI agents
- Captures policy violations with full audit metadata
- Ensures no changes bypass your safety requirements

## Current Status: Work in Progress

Corbusier is in active development. The core orchestration engine is designed
but not yet implemented.

We're building this iteratively in three phases:

1. **Core orchestration foundation** — Conversation management, task lifecycle,
   agent backend orchestration
2. **Tool plane and workflow governance** — MCP hosting, Weaver integration,
   hook engine, workspace encapsulation
3. **External integrations and interfaces** — VCS adapters, HTTP APIs,
   real-time event streaming, operator UIs

See our [development roadmap](docs/roadmap.md) for detailed progress,
dependencies, and success criteria for each phase.

## Quick Start

Since Corbusier is in early development, the best way to explore right now is
to dive into the design and roadmap:

1. **Understand the vision**: Read
   [docs/corbusier-design.md](docs/corbusier-design.md) for the full technical
   specification
2. **Track progress**: Check [docs/roadmap.md](docs/roadmap.md) to see what's
   being built and in what order
3. **Explore the codebase**: The project uses hexagonal architecture with
   strict quality gates

### Building and Testing

```bash
# Format code
make fmt

# Run linter
make lint

# Run tests
make test

# Build the project
make build
```

Note: The implementation is still in its early stages. Many features are
designed but not yet implemented.

## Learn More

- **[Corbusier Design](docs/corbusier-design.md)** — Deep dive into
  architecture, components, and design decisions
- **[Development Roadmap](docs/roadmap.md)** — What's being built, in what
  order, and why
- **[User Guide](docs/users-guide.md)** — Practical guidance on how Corbusier
  works, including quirks and edge cases (the "nicknacks")

## Contributing

Corbusier is being built in the open. If you're interested in contributing,
check out [AGENTS.md](AGENTS.md) for development guidelines and workflow
requirements.

The project enforces strict code quality standards:

- Comprehensive Clippy linting (pedantic mode with additional deny rules)
- Mandatory code formatting via `cargo fmt`
- Full test coverage requirements
- British English spelling in documentation

## Licence

Copyright © 2026 Payton McIntosh

Corbusier is released under the ISC Licence. See [LICENSE](LICENSE) for details.
## Credit

Built with care by [df12 Productions](https://df12.studio).
