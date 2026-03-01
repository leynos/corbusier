Feature: MCP server lifecycle management

  Scenario: Register, start, and query tools from an MCP server
    Given a stdio MCP server named "workspace_tools" with command "mcp-server"
    And tool "search_code" is available on that server
    When the server is registered
    And the server is started
    Then listing all servers returns 1 entries
    And the server lifecycle state is "running"
    And querying tools returns 1 entries

  Scenario: Reject duplicate MCP server registration
    Given a stdio MCP server named "workspace_tools" with command "mcp-server"
    When the server is registered twice
    Then registration fails with a duplicate server name error

  Scenario: Stopped server rejects tool queries
    Given a stdio MCP server named "workspace_tools" with command "mcp-server"
    When the server is registered
    And the server is started
    And the server is stopped
    Then the server lifecycle state is "stopped"
    And querying tools is rejected because the server is not running
