Feature: Hook-backed tool policy enforcement

  Scenario: A pre-tool-use policy permits a tool call and is queryable by conversation
    Given a pre-tool-use policy hook permits tool calls
    When a tool call executes with conversation tracking
    Then the policy audit is retrievable by conversation

  Scenario: A pre-tool-use policy denies a tool call and is queryable by task
    Given a pre-tool-use policy hook denies tool calls
    When a tool call executes with task tracking
    Then the policy audit is retrievable by task

  Scenario: A post-tool-use hook records an audit event retrievable by hook event
    Given a post-tool-use policy hook records an allow decision
    When a successful tool call completes
    Then the policy audit is retrievable by hook event

  Scenario: A policy hook emits an invalid payload and the tool call fails with a governance error
    Given a pre-tool-use policy hook emits an invalid payload
    When a tool call executes with task tracking
    Then the tool call fails with a governance error
    And no policy audit is persisted for the task
