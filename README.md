# OpenAPI MCP Bridge

A Rust MCP (Model Context Protocol) server that dynamically converts OpenAPI specifications into MCP tools.

## Features

- **Dynamic Tool Generation**: Automatically converts OpenAPI 3.0 operations into MCP tools
- **Multiple Transport Support**: Supports loading OpenAPI specs from local files or remote URLs
- **Flexible Configuration**: All configuration via environment variables
- **Authentication Support**: Supports Bearer token and API key authentication
- **YAML & JSON Support**: Automatically detects and parses both YAML and JSON OpenAPI specs
- **Error Handling**: Automatically fixes common OpenAPI spec issues (e.g., numeric values in boolean fields)

## Installation

```bash
cargo build --release
```

The executable will be at `target/release/openapi-mcp-bridge.exe`

## Configuration

Configure the server using environment variables:

### Required

- `OPENAPI_SPEC_PATH`: Path or URL to your OpenAPI specification file
  - Local file: `./openapi.json` or `/path/to/spec.yaml`
  - Remote URL: `https://api.example.com/openapi.json`

### Optional

- `API_BASE_URL`: Base URL for API requests (e.g., `https://api.example.com`)
  - If your OpenAPI spec already includes servers, this can be omitted
- `API_AUTH_TOKEN`: Bearer token for authentication
  - Adds `Authorization: Bearer <token>` header to all requests
- `API_KEY`: API key for authentication
  - Adds `X-API-Key: <key>` header to all requests
- `SKILLS_MD_PATH`: Path to a skills.md file for tool defaults (future feature)

## Usage

### Running the Server

```bash
# Set environment variables
export OPENAPI_SPEC_PATH="./openapi.json"
export API_BASE_URL="https://api.example.com"
export API_AUTH_TOKEN="your-bearer-token"

# Run the server
./target/release/openapi-mcp-bridge
```

### Claude Desktop Integration

Add to your Claude Desktop configuration (`claude_desktop_config.json`):

```json
{
  "mcpServers": {
    "openapi-bridge": {
      "command": "path/to/openapi-mcp-bridge.exe",
      "env": {
        "OPENAPI_SPEC_PATH": "https://api.example.com/openapi.json",
        "API_BASE_URL": "https://api.example.com",
        "API_AUTH_TOKEN": "your-bearer-token"
      }
    }
  }
}
```

## How It Works

1. **On Startup**: The server loads the OpenAPI specification from the specified path/URL
2. **Tool Discovery**: Each GET and POST operation in the spec becomes an MCP tool
   - Tool names are derived from `operationId` (e.g., `getUsers`, `createOrder`)
   - Tool descriptions come from `summary` or `description` fields
3. **Tool Execution**: When Claude calls a tool:
   - The server constructs the full URL using `API_BASE_URL` + operation path
   - Adds authentication headers if configured
   - For GET requests: query parameters from tool arguments
   - For POST requests: request body from tool arguments
   - Returns the API response to Claude

## Example OpenAPI Spec

```yaml
openapi: 3.0.0
info:
  title: Sample API
  version: 1.0.0
servers:
  - url: https://api.example.com/v1
paths:
  /users:
    get:
      operationId: getUsers
      summary: Get all users
      parameters:
        - name: limit
          in: query
          schema:
            type: integer
        - name: offset
          in: query
          schema:
            type: integer
      responses:
        '200':
          description: Success
    post:
      operationId: createUser
      summary: Create a new user
      requestBody:
        content:
          application/json:
            schema:
              type: object
              properties:
                name:
                  type: string
                email:
                  type: string
      responses:
        '201':
          description: Created
```

This would generate two MCP tools:
- `getUsers`: GET /users with optional limit/offset parameters
- `createUser`: POST /users with name/email body parameters

## Development

### Running Tests

```bash
cargo test
```

### Running with Debug Logging

```bash
RUST_LOG=debug cargo run
```

### Building for Production

```bash
cargo build --release
```

## Limitations

Currently supports:
- ✅ GET and POST operations
- ✅ Query parameters (GET)
- ✅ JSON request bodies (POST)
- ✅ Bearer token authentication
- ✅ API key authentication
- ✅ Local file and remote URL specs
- ✅ YAML and JSON formats

Coming soon:
- 🔲 PUT, DELETE, PATCH operations
- 🔲 Path parameters
- 🔲 Headers from OpenAPI spec
- 🔲 Request validation against schemas
- 🔲 Skills.md integration

## Troubleshooting

### "invalid type: floating point `0.0`, expected a boolean"

This error occurs when the OpenAPI spec uses numeric values (like `0.0` or `1.0`) instead of `true`/`false` for boolean fields. The server automatically fixes these issues:

- Converts `0.0` → `false` and any non-zero number → `true`
- Fixes common boolean fields: `deprecated`, `required`, `nullable`, `readOnly`, `writeOnly`, `uniqueItems`

If you see this error in logs, the server will attempt to fix it automatically and should work correctly.

### "Connection closed" or "Failed to start"

1. **Check your OpenAPI spec URL**: Ensure `OPENAPI_SPEC_PATH` is accessible
2. **Enable debug logging**: Set `RUST_LOG=debug` to see detailed error messages
3. **Test the URL**: Verify you can access the spec in a browser or with curl
4. **Check authentication**: Ensure `API_AUTH_TOKEN` or `API_KEY` are correct if required

### Debug Mode

Run with verbose logging:

```bash
export RUST_LOG=debug
./target/debug/openapi-mcp-bridge.exe
```

This will show:
- Spec loading process
- Number of tools extracted
- API requests being made
- Detailed error messages

## Architecture

```
src/
├── main.rs        # Entry point, sets up MCP server with stdio transport
├── lib.rs         # MCP handler implementation (list_tools, call_tool)
├── config.rs      # Environment variable configuration
├── state.rs       # Shared state (tools list, HTTP client, config)
├── openapi.rs     # OpenAPI spec loading and tool extraction
└── tools.rs       # Tool execution logic (makes HTTP requests)
```

## License

MIT

## Contributing

Contributions welcome! Please open an issue or submit a pull request.
