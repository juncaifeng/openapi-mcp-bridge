# RESTful API 转换问题分析与修复报告

## 📊 问题概述

用户发现 OpenAPI 转 MCP 工具后，RESTful API 的核心特征丢失。

## ❌ 原始问题

### 命名问题对比

| 原始 API | 转换前 | 转换后 | 问题 |
|---------|--------|--------|------|
| `GET /terms/{id}` - 获取词汇详情 | `terms_detail` | `detail_1` | ❌ 丢失资源、方法信息 |
| `GET /terms` - 分页查询 | `terms_page` | `page_1` | ❌ 不知道是哪个资源 |
| `POST /terms` - 创建词汇 | `terms_create` | `create_1` | ❌ 不知道创建什么 |
| `GET /categories/tree` - 获取分类树 | `categories_tree` | `tree_1` | ❌ 完全失去语义 |

### 根本原因

**OpenAPI 规范设计问题**：
```json
{
  "/terms/{id}": {
    "get": { "operationId": "detail", ... },
    "put": { "operationId": "update", ... },
    "delete": { "operationId": "remove", ... }
  },
  "/tags/{id}": {
    "get": { "operationId": "detail", ... },  // 重复！
    "put": { "operationId": "update", ... },   // 重复！
    "delete": { "operationId": "remove", ... } // 重复！
  },
  "/categories/{id}": {
    "get": { "operationId": "detail", ... },   // 重复！
    ...
  }
}
```

**问题**：
1. `operationId` 不唯一（违反 OpenAPI 规范建议）
2. 多个资源使用相同的操作名称（detail, create, page, update 等）
3. 去重逻辑使用数字后缀 `_1`, `_2`, `_3`（完全失去语义）

## ✅ 修复方案

### 新的命名策略

**格式**：`{http_method}_{resource}_{operation}`

**示例**：

```
GET /terms/{id} + operationId=detail
→ get_terms_detail ✓

POST /terms + operationId=create
→ post_terms_create ✓

GET /categories/tree + operationId=tree
→ get_categories_tree ✓

PATCH /terms/{id}/status + operationId=changeStatus
→ patch_terms_changestatus ✓
```

### 核心改进

1. **HTTP 方法保留**
   ```rust
   method.to_lowercase()  // get, post, put, delete, patch
   ```

2. **资源名称提取**
   ```rust
   // 从路径提取第一个非参数段
   path.split('/')
       .filter(|s| !s.is_empty() && !s.starts_with('{'))
       .next()
   ```

3. **操作语义保留**
   ```rust
   op.operation_id.as_deref().unwrap_or("unknown")
   ```

4. **智能去重**
   ```rust
   // 使用计数器处理真正重复的情况
   let count = name_counter.entry(base_name).or_insert(0);
   *count += 1;
   if *count > 1 {
       format!("{}_{}", base_name, count)
   }
   ```

## 📈 修复效果对比

### 词汇管理（Terms）

| API | 修复前 | 修复后 | 改进 |
|-----|--------|--------|------|
| `GET /terms/{id}` | `detail_1` | `get_terms_detail` | ✅ 清晰明确 |
| `PUT /terms/{id}` | `update_1` | `put_terms_update` | ✅ 方法可见 |
| `DELETE /terms/{id}` | `remove_1` | `delete_terms_remove` | ✅ 资源可见 |
| `GET /terms` | `page_1` | `get_terms_page` | ✅ 操作可见 |
| `POST /terms` | `create_1` | `post_terms_create` | ✅ RESTful 语义 |
| `PATCH /terms/{id}/status` | `changestatus_1` | `patch_terms_changestatus` | ✅ 完整信息 |

### 标签管理（Tags）

| API | 修复前 | 修复后 |
|-----|--------|--------|
| `GET /tags/{id}` | `detail_2` | `get_tags_detail` |
| `POST /tags` | `create_2` | `post_tags_create` |

### 分类管理（Categories）

| API | 修复前 | 修复后 |
|-----|--------|--------|
| `GET /categories/tree` | `tree_1` | `get_categories_tree` |
| `POST /categories` | `create_3` | `post_categories_create` |

## 🎯 核心优势

### 1. RESTful 语义保留
- ✅ HTTP 方法一目了然（GET/POST/PUT/DELETE/PATCH）
- ✅ 资源名称清晰（terms/tags/categories）
- ✅ 操作语义明确（detail/create/page/tree）

### 2. 可读性提升
```python
# 修复前（完全不知道是什么）
tools = ["detail_1", "detail_2", "detail_3", "create_1", "create_2"]

# 修复后（清晰明了）
tools = [
    "get_terms_detail",      # 获取词汇详情
    "get_tags_detail",       # 获取标签详情
    "get_categories_detail", # 获取分类详情
    "post_terms_create",     # 创建词汇
    "post_tags_create"       # 创建标签
]
```

### 3. API 发现性
- 用户可以通过工具名直接推断 API 用途
- 遵循 RESTful 约定，符合开发者习惯
- 减少文档查阅需求

## ⚠️ 破坏性变更

**影响**：现有工具名称将改变

**权衡**：
- 短期：需要更新工具调用代码
- 长期：语义正确性和可维护性大幅提升

**建议**：接受破坏性变更，换取正确的 API 语义

## 📝 测试验证

**测试 API**：`http://192.168.70.186:8088/api/v3/api-docs/default`

**结果**：
- ✅ 提取 43 个工具
- ✅ 所有工具名称包含 HTTP 方法
- ✅ 所有工具名称包含资源名
- ✅ 所有工具名称包含操作语义
- ✅ 参数正确解析（id, type, query 等）

## 🔧 实现细节

### 代码改动

**文件**：`src/openapi.rs`

**改动**：
1. 修改 `extract_tools()` 添加 name_counter
2. 完全重写 `make_tool()` 命名逻辑
3. 支持 PUT/DELETE/PATCH 方法

**核心逻辑**：
```rust
let base_name = format!(
    "{}_{}_{}",
    method.to_lowercase(),
    resource.to_lowercase().replace('-', "_"),
    op_id.to_lowercase().replace('-', "_")
);
```

## 📊 总结

| 维度 | 修复前 | 修复后 |
|------|--------|--------|
| HTTP 方法 | ❌ 丢失 | ✅ 保留 |
| 资源名称 | ❌ 丢失 | ✅ 保留 |
| 操作语义 | ❌ 模糊 | ✅ 清晰 |
| RESTful 特征 | ❌ 破坏 | ✅ 保持 |
| 可读性 | ❌ 差 | ✅ 优秀 |
| API 发现性 | ❌ 困难 | ✅ 简单 |

**结论**：修复后完全保留了 RESTful API 的核心特征，解决了用户提出的所有问题。

---

**Commit**: `e3ae0e6` - fix: preserve RESTful API semantics in tool naming
**Date**: 2026-03-30
