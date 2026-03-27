# 故障排除指南

## 问题：MCP Server is not running

### 诊断步骤

#### 1. 检查可执行文件是否存在
```powershell
Test-Path E:\git\mcp-build\tags\openapi-mcp-bridge\target\release\openapi-mcp-bridge.exe
```
应该返回 `True`

#### 2. 测试服务器能否启动
```powershell
cd E:\git\mcp-build\tags\openapi-mcp-bridge
$env:OPENAPI_SPEC_PATH="http://192.168.90.123:9999/openapi.json"
$env:API_BASE_URL="http://192.168.90.123:9999"
.\target\release\openapi-mcp-bridge.exe
```

如果看到 `Extracted 80 tools` 说明服务器启动成功！

#### 3. 测试网络连接
```powershell
curl http://192.168.90.123:9999/openapi.json
```
应该能下载 OpenAPI 规范文件

#### 4. 检查 Claude Desktop 配置

配置文件位置：
```
%APPDATA%\Claude\claude_desktop_config.json
```

完整路径示例：
```
C:\Users\YourName\AppData\Roaming\Claude\claude_desktop_config.json
```

### 解决方案 A：直接配置（推荐）

```json
{
  "mcpServers": {
    "projax": {
      "command": "E:\git\mcp-build\tags\openapi-mcp-bridge\target\release\openapi-mcp-bridge.exe",
      "env": {
        "OPENAPI_SPEC_PATH": "http://192.168.90.123:9999/openapi.json",
        "API_BASE_URL": "http://192.168.90.123:9999"
      }
    }
  }
}
```

**注意：**
- 使用双反斜杠 `\` 作为路径分隔符
- 使用 `release` 版本而不是 `debug` 版本
- 确保 `env` 字段中的环境变量名称正确

### 解决方案 B：使用启动脚本

如果方案 A 不工作，使用启动脚本：

1. 配置：
```json
{
  "mcpServers": {
    "projax": {
      "command": "E:\git\mcp-build\tags\openapi-mcp-bridge\start-projax.bat"
    }
  }
}
```

2. 启动脚本 `start-projax.bat`：
```batch
@echo off
set OPENAPI_SPEC_PATH=http://192.168.90.123:9999/openapi.json
set API_BASE_URL=http://192.168.90.123:9999
E:\git\mcp-build\tags\openapi-mcp-bridge\target\release\openapi-mcp-bridge.exe
```

### 解决方案 C：使用绝对路径和正斜杠

```json
{
  "mcpServers": {
    "projax": {
      "command": "E:/git/mcp-build/tags/openapi-mcp-bridge/target/release/openapi-mcp-bridge.exe",
      "env": {
        "OPENAPI_SPEC_PATH": "http://192.168.90.123:9999/openapi.json",
        "API_BASE_URL": "http://192.168.90.123:9999"
      }
    }
  }
}
```

### 验证步骤

1. 修改配置后，**完全关闭** Claude Desktop（检查任务管理器确保没有残留进程）
2. 重新启动 Claude Desktop
3. 在 Claude 中输入 `/mcp` 查看服务器状态
4. 应该看到 `projax` 服务器显示为绿色/运行状态

### 查看日志

Claude Desktop 日志位置：
```
%APPDATA%\Claude\logs\
```

查看最新的日志文件，搜索 "projax" 或 "mcp" 查看错误信息。

### 常见错误

#### 错误 1：环境变量未找到
```
Error: environment variable not found
```
**原因**：`env` 配置没有生效
**解决**：使用启动脚本（方案 B）

#### 错误 2：找不到文件
```
The system cannot find the file specified
```
**原因**：路径错误
**解决**：检查路径是否正确，使用绝对路径

#### 错误 3：网络超时
```
Failed to fetch spec from http://192.168.90.123:9999/openapi.json
```
**原因**：无法访问 API
**解决**：检查网络连接，确保 API 服务器正在运行

### 手动测试 MCP 协议

运行调试脚本：
```powershell
.\debug-server.bat
```

如果看到 JSON 响应包含 `"result"` 字段，说明服务器正常工作。

### 仍然无法解决？

1. 运行 `debug-server.bat` 并将输出发送给我
2. 检查 Claude Desktop 日志文件
3. 确认你的 Claude Desktop 版本支持 MCP（需要最新版本）
4. 尝试使用其他 MCP 服务器验证 MCP 功能是否正常
