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

send({"jsonrpc": "2.0", "id": 1, "method": "initialize",
      "params": {"protocolVersion": "2024-11-05", "capabilities": {},
                 "clientInfo": {"name": "test", "version": "1.0"}}})
recv()

send({"jsonrpc": "2.0", "method": "notifications/initialized"})

send({"jsonrpc": "2.0", "id": 2, "method": "tools/list", "params": {}})
resp = recv()

if resp and "result" in resp:
    tools = resp["result"]["tools"]
    print(f"Total: {len(tools)} tools\n")

    # 按前缀分组
    groups = {}
    for tool in tools:
        name = tool['name']
        prefix = name.split('_')[0] if '_' in name else name
        if prefix not in groups:
            groups[prefix] = []
        groups[prefix].append(name)

    for prefix in sorted(groups.keys())[:10]:  # 前10组
        print(f"{prefix}: {len(groups[prefix])} tools")
        for name in sorted(groups[prefix])[:5]:
            print(f"  - {name}")

proc.terminate()
