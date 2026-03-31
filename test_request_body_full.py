#!/usr/bin/env python3
import subprocess
import json

proc = subprocess.Popen(
    ["E:/git/mcp-build/tags/openapi-mcp-bridge/target/release/openapi-mcp-bridge.exe"],
    stdin=subprocess.PIPE,
    stdout=subprocess.PIPE,
    stderr=subprocess.DEVNULL,
    env={"OPENAPI_SPEC_PATH": "api_docs_temp.json"},
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
    print(f"[OK] Total tools: {len(tools)}\n")

    # 查找创建类工具
    create_tools = [t for t in tools if "create" in t["name"]]
    print(f"[INFO] Found {len(create_tools)} create tools:\n")

    for tool in create_tools[:5]:  # 显示前5个
        print(f"Tool: {tool['name']}")
        print(f"Description: {tool.get('description', 'N/A')}")
        schema = tool.get("inputSchema", {})
        properties = schema.get("properties", {})
        required = schema.get("required", [])

        if properties:
            print(f"[OK] Parameters ({len(properties)}):")
            for name, prop in list(properties.items())[:10]:  # 显示前10个参数
                req_mark = " *" if name in required else ""
                prop_type = prop.get('type', 'unknown')
                print(f"  - {name}{req_mark}: {prop_type}")
        else:
            print("[ERROR] No parameters defined")
        print()

proc.terminate()
