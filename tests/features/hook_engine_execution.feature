Feature: Hook engine execution

  Scenario: Pre-commit hook executes successfully
    Given a pre-commit hook is configured
    When the pre-commit hook trigger fires
    Then the hook execution is recorded as success

  Scenario: Post-deploy hook failure is recorded
    Given a post-deploy hook is configured to fail
    When the post-deploy hook trigger fires
    Then the hook execution is recorded as failure
