Feature: Tool discovery and routing

  Scenario: Discover tools from a running MCP server
    Given a stdio MCP server named "workspace_tools" with command "mcp-server"
    And tool "read_file" is available on that server
    When the server is registered and started
    And tools are discovered
    Then the tool catalogue contains 1 entry
    And tool "read_file" is marked as available

  Scenario: Route a tool call to the correct server
    Given a stdio MCP server named "workspace_tools" with command "mcp-server"
    And tool "read_file" is available on that server
    And calling tool "read_file" on that server returns '{"content": "hello"}'
    When the server is registered and started
    And tools are discovered
    And tool "read_file" is called with parameters '{"path": "/tmp/test.txt"}'
    Then the tool call succeeds
    And the audit log contains 1 entry for tool "read_file"

  Scenario: Tool becomes unavailable when server stops
    Given a stdio MCP server named "workspace_tools" with command "mcp-server"
    And tool "read_file" is available on that server
    When the server is registered and started
    And tools are discovered
    And the server is stopped
    And tools are marked unavailable
    Then calling tool "read_file" is rejected as unavailable

  Scenario: Unknown tool call is rejected
    Given a stdio MCP server named "workspace_tools" with command "mcp-server"
    When the server is registered and started
    And tools are discovered
    Then calling tool "nonexistent_tool" is rejected as not found

  Scenario: Tool call stderr is captured in the log store
    Given a stdio MCP server named "workspace_tools" with command "mcp-server"
    And tool "read_file" is available on that server
    And calling tool "read_file" on that server returns '{"content": "hello"}'
    And calling tool "read_file" on that server produces stderr "debug: opening file"
    When the server is registered and started
    And tools are discovered
    And tool "read_file" is called with parameters '{"path": "/tmp/test.txt"}'
    Then the tool call succeeds
    And the audit log entry for tool "read_file" has a stderr log path
