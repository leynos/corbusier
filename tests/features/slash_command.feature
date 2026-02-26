Feature: Slash command parsing and template execution
  To standardize orchestration workflows
  As a Corbusier operator
  I want slash commands to expand into deterministic auditable tool plans

  Scenario: Valid command expands into a tool plan
    Given a slash command service with built-in commands
    When I execute the slash command "/task action=start issue=123"
    Then the command expansion is recorded
    And a deterministic tool plan is produced

  Scenario: Unknown command is rejected
    Given a slash command service with built-in commands
    When I execute the slash command "/missing action=start"
    Then the slash command fails with unknown command "missing"

  Scenario: Missing required parameter is rejected
    Given a slash command service with built-in commands
    When I execute the slash command "/task issue=123"
    Then the slash command fails with missing parameter "action" for command "task"

  Scenario: Repeated command execution is deterministic
    Given a slash command service with built-in commands
    When I execute the slash command twice "/review action=sync include_summary=true"
    Then both executions produce identical tool plans
