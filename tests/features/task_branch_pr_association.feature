Feature: Branch and pull request association with tasks

  Scenario: Associate a branch with a task and retrieve by reference
    Given an external issue "github" "corbusier/core" #200
    And the issue has title "Implement branch tracking"
    And the issue has been converted into a task
    When a branch "github" "corbusier/core" "feature/branch-tracking" is associated with the task
    Then the task has an associated branch reference
    And the task can be retrieved by the branch reference

  Scenario: Associate a pull request with a task and verify state
    Given an external issue "github" "corbusier/core" #201
    And the issue has title "Implement PR tracking"
    And the issue has been converted into a task
    When a pull request "github" "corbusier/core" #42 is associated with the task
    Then the task has an associated pull request reference
    And the task state is in_review

  Scenario: Reject second branch association on the same task
    Given an external issue "github" "corbusier/core" #202
    And the issue has title "Duplicate branch test"
    And the issue has been converted into a task
    And a branch is already associated with the task
    When a second branch is associated with the task
    Then branch association fails with a branch already associated error

  Scenario: Reject second pull request association on the same task
    Given an external issue "github" "corbusier/core" #203
    And the issue has title "Duplicate PR test"
    And the issue has been converted into a task
    And a pull request is already associated with the task
    When a second pull request is associated with the task
    Then pull request association fails with a PR already associated error
