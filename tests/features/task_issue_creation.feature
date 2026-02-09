Feature: Issue-to-task creation and tracking

  Scenario: Create task from issue and retrieve by reference
    Given an external issue "github" "corbusier/core" #120 with title "Track issue metadata"
    When the issue is converted into a task
    Then the task is created with draft state and lifecycle timestamps
    And the task can be retrieved by the external issue reference

  Scenario: Reject duplicate task creation for the same issue
    Given an external issue "gitlab" "corbusier/core" #45 with title "Prevent duplicate mapping"
    And a task has already been created from that issue
    When the issue is converted into a task
    Then task creation fails with a duplicate issue reference error

  Scenario: Return no task for an unknown issue reference
    Given an unknown issue reference "github" "corbusier/core" #9999
    When the task is requested by external issue reference
    Then no task is returned

