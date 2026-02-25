Feature: Agent backend registration and discovery

  Scenario: Register two backends and list them
    Given a backend named "claude_code_sdk" from provider "Anthropic"
    And a backend named "codex_cli" from provider "OpenAI"
    When both backends are registered
    Then listing all backends returns 2 entries
    And the backend "claude_code_sdk" can be found by name
    And the backend "codex_cli" can be found by name

  Scenario: Reject duplicate backend name
    Given a backend named "claude_code_sdk" from provider "Anthropic"
    And the backend has already been registered
    When a second backend with the same name is registered
    Then registration fails with a duplicate name error

  Scenario: Deactivate a backend and exclude from active listing
    Given a registered backend named "test_backend" from provider "Test"
    When the backend is deactivated
    Then listing active backends does not include "test_backend"
    And listing all backends still includes "test_backend"
