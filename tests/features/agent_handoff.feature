Feature: Preserve context across agent handoffs
  In order to maintain conversation continuity
  As a multi-agent orchestration system
  I want to preserve context when handing off between agents

  Background:
    Given an active agent session for a conversation

  Scenario: Successful handoff to a different agent
    When the current agent initiates a handoff to a specialist agent
    Then a handoff record is created with initiated status
    And a context snapshot is captured for the source session
    And the source session is marked as handed off

  Scenario: Complete handoff when target agent accepts
    Given an initiated handoff to a target agent
    When the target agent creates a new session
    And the handoff is completed
    Then the handoff record links source and target sessions
    And the handoff status is completed

  Scenario: Cancel a pending handoff
    Given an initiated handoff to a target agent
    When the handoff is cancelled
    Then the source session is reverted to active state
    And no target session is created

  Scenario: Handoff references prior turn and tool calls
    Given a conversation with tool calls
    When a handoff is initiated with tool call references
    Then the handoff metadata includes the prior turn id
    And the handoff metadata includes the triggering tool calls

  Scenario: Multiple handoffs in a conversation chain
    Given a completed handoff from agent A to agent B
    When agent B initiates a handoff to agent C
    Then the conversation history shows all agent sessions
    And each handoff is linked in sequence
