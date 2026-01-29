Feature: Conversation history with audit metadata

  Scenario: Persist tool call and agent response audit metadata
    Given an empty conversation history
    When a tool call and agent response are persisted
    Then the conversation history includes audit metadata

  Scenario: Missing tool call audit metadata is rejected
    Given an empty conversation history
    When a tool call audit is missing a call id
    Then the message is rejected with a metadata error
