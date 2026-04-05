# Roadmap writing guide

This guide defines how Corbusier documents roadmap items and planning
hierarchies. It is intentionally separate from
`docs/documentation-style-guide.md` so the core documentation style rules stay
project-agnostic while roadmap and execution-planning conventions remain
discoverable for repository contributors.

## Purpose

Use this guide when writing or reviewing roadmap entries, milestone breakdowns,
or other planning artefacts that need to align with Corbusier's planning model.

## Roadmap task writing guidelines

When documenting development roadmap items, write them to be achievable,
measurable, and structured. This ensures the roadmap functions as a practical
planning tool rather than a vague wishlist. Do not commit to timeframes in the
roadmap. Development effort should be roughly consistent from task to task.

### Principles for roadmap tasks

- Define outcomes, not intentions: phrase tasks in terms of the capability
  delivered (e.g. “Implement role-based access control for API endpoints”), not
  aspirations like “Improve security”.
- Quantify completion criteria: attach measurable finish lines (e.g. “90% test
  coverage for new modules”, “response times under 200ms”, “all endpoints
  migrated”).
- Break into atomic increments: ensure tasks can be completed in weeks, not
  quarters. Large goals should be decomposed into clear, deliverable units.
- Tie to dependencies and sequencing: document prerequisites, so tasks can be
  scheduled realistically (e.g. “Introduce central logging service” before “Add
  error dashboards”).
- Bound scope explicitly: note both in-scope and out-of-scope elements (e.g.
  “Build analytics dashboard (excluding churn prediction)”).

### Hierarchy of scope

Roadmaps should be expressed in three layers of scope to maintain clarity and
navigability:

- Phases (strategic milestones) – broad outcome-driven stages that represent
  significant capability shifts. Why the work matters.
- Steps (epics / workstreams) – mid-sized clusters of related tasks grouped
  under a phase. What will be built.
- Tasks (execution units) – small, measurable pieces of work with clear
  acceptance criteria. How it gets done.

This hierarchy should align with the Goals, Ideas, Steps, and Tasks (GIST)
framework:

- Phases correspond to strategic Goals and their associated Ideas. A phase is
  the strategic umbrella that groups related Steps and Tasks under one shared
  intent, rather than a delivery bucket without a higher-level objective.
- Steps correspond to GIST-style workstreams. A step must describe a coherent
  body of delivery work with one clear objective, explicit sequencing value,
  and a practical learning loop. A step is not just a heading used to group
  unrelated tasks.
- Tasks correspond to implementation-level execution units. A task should be a
  concrete build activity, not an aspiration, research topic, or status label.

### Roadmap formatting conventions

- **Dotted numbering:** Number phases, steps, and headline tasks using dotted
  notation:
  - Phases: 1, 2, 3, …
  - Steps: 1.1, 1.2, 1.3, …
  - Headline tasks: 1.1.1, 1.1.2, 1.1.3, …
- **Checkboxes:** Precede task and sub-task items with a GitHub-flavoured
  Markdown (GFM) checkbox (`[ ]`) to track completion status.
- **Dependencies:** Note non-linear dependencies explicitly. Where a task
  depends on another task outside its immediate sequence, cite the dependency
  using dotted notation (e.g. “Requires 2.3.1”).
- **Success criteria:** Include explicit success criteria only where not
  immediately obvious from the task description.
- **Design document citations:** Where applicable, cite the relevant design
  document section for each task (e.g. “See design-doc.md §3.2”).

### Roadmap example

```markdown
## 1. Core infrastructure

### 1.1. Logging subsystem

- [ ] 1.1.1. Introduce central logging service
  - Define log message schema. See design-doc.md §2.1.
  - Implement log collector daemon.
  - Add structured logging to API layer.
- [ ] 1.1.2. Add error dashboards. Requires 1.1.1.
  - Deploy Grafana instance.
  - Create error rate dashboard (target: <1% error rate visible within 5 min).

### 1.2. Authentication

- [ ] 1.2.1. Implement role-based access control (RBAC). Requires 1.1.1.
  - Define role hierarchy. See design-doc.md §4.3.
  - Add RBAC middleware to API endpoints.
  - Write integration tests for permission boundaries.
```

### Writing GIST-aligned steps

When writing a roadmap step, make it function as a real workstream:

- Give the step a concrete objective that describes what will exist when the
  workstream is complete.
- State the learning opportunity for the step when that learning affects later
  sequencing or design choices.
- Group only tasks that serve the same delivery objective. If the tasks do not
  share one operational purpose, split the step.
- Sequence steps so that each workstream either unlocks the next one or reduces
  a specific class of delivery risk.

Avoid using steps as passive document structure. Headings such as “Backend
changes”, “Frontend work”, or “Other tasks” are not sufficient unless they are
framed as real workstreams with a defined outcome.
