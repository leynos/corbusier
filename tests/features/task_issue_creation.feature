Feature: Issue-to-task creation and tracking

  Scenario: Create task from issue and retrieve by reference
    Given an external issue "github" "corbusier/core" #120
    And the issue has title "Track issue metadata"
    When the issue is converted into a task
    Then the task is created with draft state and lifecycle timestamps
    And the task can be retrieved by the external issue reference

  Scenario: Reject duplicate task creation for the same issue
    Given an external issue "gitlab" "corbusier/core" #45
    And the issue has title "Prevent duplicate mapping"
    And a task has already been created from that issue
    When the issue is converted into a task
    Then task creation fails with a duplicate issue reference error

  Scenario: Return no task for an unknown issue reference
    Given an unknown issue reference "github" "corbusier/core" #9999
    When the task is requested by external issue reference
    Then no task is returned

  Scenario: Two tenants can create tasks from the same issue reference
    Given an external issue "github" "corbusier/core" #120
    And the issue has title "Track issue metadata"
    When tenant A converts the issue into a task
    And tenant B converts the same issue into a task
    Then both tenants successfully create distinct tasks from the same issue
    And each tenant can retrieve its own task by the external issue reference
