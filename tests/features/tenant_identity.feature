Feature: Tenant identity and domain primitives

  Scenario: Create a tenant with valid slug and display name
    Given a tenant slug "acme-corp" with display name "Acme Corporation"
    When the tenant is created
    Then the tenant has a unique identifier
    And the tenant slug is "acme-corp"
    And the tenant display name is "Acme Corporation"
    And the tenant status is "active"

  Scenario: Reject tenant creation with invalid slug
    Given a tenant slug "INVALID SLUG!!" with display name "Bad Tenant"
    When the tenant creation is attempted
    Then tenant creation fails with a slug validation error

  Scenario: Reject tenant creation with empty display name
    Given a tenant slug "valid-slug" with display name "   "
    When the tenant creation is attempted
    Then tenant creation fails with an empty display name error

  Scenario: Tenant slug normalises to lowercase
    Given a tenant slug "My-Tenant" with display name "My Tenant"
    When the tenant is created
    Then the tenant slug is "my-tenant"
