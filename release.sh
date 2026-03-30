#!/bin/bash
# 手动发布脚本 - 当网络恢复后执行

set -e

echo "=================================="
echo "OpenAPI MCP Bridge 发布脚本"
echo "=================================="
echo ""

# 检查网络连接
echo "检查 GitHub 连接..."
if ! curl -s -m 10 https://github.com > /dev/null; then
    echo "❌ 无法连接到 GitHub，请检查网络连接"
    exit 1
fi
echo "✓ GitHub 连接正常"
echo ""

# 推送最新提交
echo "推送最新代码..."
git push
echo "✓ 代码推送成功"
echo ""

# 删除旧标签
echo "删除旧标签 v0.1.0..."
git push --delete origin v0.1.0 2>/dev/null || true
git tag -d v0.1.0 2>/dev/null || true
echo "✓ 旧标签已删除"
echo ""

# 创建新标签
echo "创建新标签 v0.1.0..."
git tag v0.1.0 -m "Release v0.1.0 - Initial release with 80 OpenAPI tools support"
echo "✓ 新标签已创建"
echo ""

# 推送标签
echo "推送标签到 GitHub..."
git push origin v0.1.0
echo "✓ 标签推送成功"
echo ""

# 等待 GitHub Actions 开始
echo "等待 GitHub Actions 开始构建..."
sleep 10

# 检查构建状态
echo "检查构建状态..."
gh run list --limit 1
echo ""

echo "=================================="
echo "✓ 发布流程已触发！"
echo "=================================="
echo ""
echo "查看构建进度："
echo "  https://github.com/juncaifeng/openapi-mcp-bridge/actions"
echo ""
echo "构建完成后（约3-5分钟），用户可以从以下地址下载："
echo "  https://github.com/juncaifeng/openapi-mcp-bridge/releases/tag/v0.1.0"
