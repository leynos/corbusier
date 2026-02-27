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

  Scenario: Invalid boolean parameter is rejected
    Given a slash command service with built-in commands
    When I execute the slash command "/review action=sync include_summary=notabool"
    Then the slash command fails with invalid boolean parameter "include_summary" for command "review"

  Scenario: Invalid tool arguments template is rejected
    Given a slash command service with an invalid tool arguments template
    When I execute the slash command "/broken value=test"
    Then the slash command fails with invalid tool arguments template for tool "broken_tool"

  Scenario: Quoted values preserve spaces
    Given a slash command service with built-in commands
    When I execute the slash command '/task action=start issue="ENG 123"'
    Then the command expansion records issue parameter "ENG 123"
