# 2.3.2 hook policy enforcement and audit capture: commands and checklist

This companion document holds the concrete operator commands and validation
checklist that were removed from the primary ExecPlan to keep the roadmap
document concise and under the repository line limit.

Run all commands from `/home/user/project`.

## Concrete steps

1. Read the relevant code and add failing tests first.

   ```bash
   cargo test --workspace hook_engine::tests::domain_tests -- --nocapture
   ```

   Expected shape:

   ```plaintext
   running ... tests
   test ... fails because policy audit support is not implemented yet
   ```

2. Implement the hook-engine domain and audit repository changes, then run
   the focused tests.

   ```bash
   cargo test --workspace hook_engine -- --nocapture
   ```

   Expected shape:

   ```plaintext
   running ... tests
   test hook_engine::... ... ok
   ```

3. Add and verify in-memory integration coverage.

   ```bash
   cargo test --workspace in_memory::hook_engine_tests -- --nocapture
   cargo test --workspace in_memory::tool_discovery_routing_tests -- --nocapture
   ```

4. Add and verify PostgreSQL-backed coverage with the embedded cluster
   harness.

   ```bash
   cargo install pg-embedded-setup-unpriv
   cargo test --workspace postgres::hook_engine_tests -- --nocapture
   cargo test --workspace postgres::tool_discovery_routing_tests -- --nocapture
   ```

5. Add behavioural coverage.

   ```bash
   cargo test --workspace hook_policy_enforcement_scenarios -- --nocapture
   ```

6. Run the required repository quality gates with logs captured via `tee`.

   ```bash
   set -o pipefail; make check-fmt 2>&1 | tee /tmp/2-3-2-check-fmt.log
   set -o pipefail; make lint 2>&1 | tee /tmp/2-3-2-lint.log
   set -o pipefail; make test TEST_FLAGS='--profile long --all-targets --all-features' 2>&1 | tee /tmp/2-3-2-test.log
   set -o pipefail; make fmt 2>&1 | tee /tmp/2-3-2-fmt.log
   set -o pipefail; PATH=/root/.bun/bin:$PATH make markdownlint 2>&1 | tee /tmp/2-3-2-markdownlint.log
   set -o pipefail; make nixie 2>&1 | tee /tmp/2-3-2-nixie.log
   ```

   Expected shape:

   ```plaintext
   ... finished with status: success
   ... test result: ok
   ... markdownlint: 0 errors
   ... All diagrams validated successfully
   ```

## Validation evidence checklist

- [x] `make fmt`
- [x] `make check-fmt`
- [x] `make lint`
- [x] `make test TEST_FLAGS='--profile long --all-targets --all-features'`
- [x] `PATH=/root/.bun/bin:$PATH make markdownlint`
- [x] `make nixie`

## Artifacts and notes

The most important evidence to capture during implementation is the query
surface itself. Keep short proof points such as:

```plaintext
policy audit query by task returns 1 event for denied tool call
policy audit query by conversation returns 1 event for permitted tool call
policy audit query by trigger_context returns the exact hook event just executed
```

For the PostgreSQL adapter, a concise raw-SQL verification is appropriate in
integration tests, for example:

```sql
SELECT count(*)
FROM hook_policy_audit_events
WHERE tenant_id = $1
  AND task_id = $2;
```
