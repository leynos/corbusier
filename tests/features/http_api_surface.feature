Feature: HTTP API surface

  Scenario: Create a conversation and append a message through HTTP
    Given an authenticated HTTP API client
    When I create a conversation through the API
    And I append the message "Hello over HTTP" as "user"
    And I request the conversation history
    Then the response status is 200
    And the response metadata version is "v1"
    And the conversation history includes 1 message

  Scenario: Create a task from issue metadata through HTTP
    Given an authenticated HTTP API client
    When I create a task from issue 42 in "owner/repo"
    Then the response status is 201
    And the response metadata version is "v1"
    And the task is returned in the response

  Scenario: Transition a task state through HTTP
    Given an authenticated HTTP API client
    And I created a draft task through the API
    When I transition the task state to "in_progress"
    Then the response status is 200
    And the response metadata version is "v1"
    And the task state is "in_progress"

  Scenario: List tools and invoke a tool through HTTP
    Given an authenticated HTTP API client
    When I list tools through the API
    Then the response status is 200
    And the response metadata version is "v1"
    And the response includes 1 tools
    When I call the "read_file" tool through the API
    Then the response status is 200
    And the response metadata version is "v1"
    And the tool call response names the tool "read_file"

  Scenario: Reject unauthenticated access
    Given an unauthenticated HTTP API client
    When I create a conversation through the API
    Then the response status is 401
    And the response metadata version is "v1"
