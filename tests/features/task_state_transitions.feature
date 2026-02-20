Feature: Task state transitions

  Scenario: Transition a draft task to in progress
    Given an external issue "github" "corbusier/core" #320
    And the issue has title "Transition happy path"
    And the issue has been converted into a task
    When the task is transitioned to "in_progress"
    Then the task state is "in_progress"

  Scenario: Reject transition from draft to done
    Given an external issue "github" "corbusier/core" #321
    And the issue has title "Transition invalid path"
    And the issue has been converted into a task
    When the task is transitioned to "done"
    Then the transition fails with an invalid state transition error

  Scenario: Reject transition from a terminal state
    Given an external issue "github" "corbusier/core" #322
    And the issue has title "Transition terminal path"
    And the issue has been converted into a task
    And the task has been transitioned to "abandoned"
    When the task is transitioned to "in_progress"
    Then the transition fails with an invalid state transition error

  Scenario: Reject transition with an invalid state string
    Given an external issue "github" "corbusier/core" #323
    And the issue has title "Transition parse error path"
    And the issue has been converted into a task
    When the task is transitioned to "invalid_state"
    Then the transition fails with an invalid state error
