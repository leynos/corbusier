Feature: Agent turn orchestration and session continuity

  Scenario: Execute a turn with routed tool calls
    Given an active backend named "claude_code_sdk"
    And the runtime returns assistant text "hello" with tool "search_docs"
    When I execute a turn for conversation "alpha"
    Then the turn succeeds
    And one tool result is returned
    And all tool audits are "succeeded"

  Scenario: Reuse an active session before expiry
    Given an active backend named "claude_code_sdk"
    And an existing active session for conversation "beta"
    And the runtime returns assistant text "reused" with no tools
    When I execute a turn for conversation "beta"
    Then the turn succeeds
    And the existing session is reused

  Scenario: Rotate a session when it is expired
    Given an active backend named "claude_code_sdk"
    And an expired active session for conversation "gamma"
    And the runtime returns assistant text "rotated" with no tools
    When I execute a turn for conversation "gamma"
    Then the turn succeeds
    And the session is rotated

  Scenario: Surface tool routing failure
    Given an active backend named "claude_code_sdk"
    And the runtime returns assistant text "oops" with tool "broken_tool"
    And the tool router fails for tool "broken_tool"
    When I execute a turn for conversation "delta"
    Then the turn fails with a tool routing error

  Scenario: Concurrent turns on same backend/conversation
    Given an active backend named "claude_code_sdk"
    And the runtime returns assistant texts "first" and "second" with no tools
    When I execute two concurrent turns for conversation "epsilon"
    Then both concurrent turns succeed
    And only one active session remains for conversation "epsilon"
