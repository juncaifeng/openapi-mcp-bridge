#!/usr/bin/env python3
import subprocess
import json

proc = subprocess.Popen(
    ["E:/git/mcp-build/tags/openapi-mcp-bridge/target/release/openapi-mcp-bridge.exe"],
    stdin=subprocess.PIPE,
    stdout=subprocess.PIPE,
    stderr=subprocess.DEVNULL,
    env={"OPENAPI_SPEC_PATH": "test_mcp.json"},
    bufsize=0
)

def send(req):
    proc.stdin.write((json.dumps(req) + "\n").encode())
    proc.stdin.flush()

def recv():
    line = proc.stdout.readline().decode()
    return json.loads(line.strip()) if line else None

# Initialize
send({"jsonrpc": "2.0", "id": 1, "method": "initialize",
      "params": {"protocolVersion": "2024-11-05", "capabilities": {},
                 "clientInfo": {"name": "test", "version": "1.0"}}})
recv()

send({"jsonrpc": "2.0", "method": "notifications/initialized"})

# List tools
send({"jsonrpc": "2.0", "id": 2, "method": "tools/list", "params": {}})
resp = recv()

if resp and "result" in resp:
    tools = resp["result"]["tools"]
    print(f"Total tools: {len(tools)}\n")

    # 查找创建类工具
    for tool in tools:
        if "create" in tool["name"]:
            print(f"Tool: {tool['name']}")
            print(f"Description: {tool.get('description', 'N/A')}")
            schema = tool.get("inputSchema", {})
            properties = schema.get("properties", {})
            required = schema.get("required", [])

            if properties:
                print(f"Parameters ({len(properties)}):")
                for name, prop in properties.items():
                    req_mark = " *" if name in required else ""
                    print(f"  - {name}{req_mark}: {prop.get('type', 'unknown')}")
                    if 'description' in prop:
                        print(f"    {prop['description']}")
            else:
                print("  No parameters defined")
            print()

proc.terminate()
