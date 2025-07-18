# 旺文社国語辞典 API 使用文档

## 概述

这是一个基于旺文社国語辞典的日语词典查询API服务，提供精确和模糊搜索功能。

## 基础信息

- **服务地址**: `http://localhost:3000`
- **数据源**: 旺文社国語辞典 (80,615个词条)
- **响应格式**: JSON

## API 端点

### 1. 服务信息
```
GET /
```
返回API服务的基本信息和使用说明。

### 2. 词条查询
```
GET /search?word=<查询词>&search_type=<搜索类型>
```

#### 参数说明

| 参数 | 类型 | 必填 | 说明 |
|------|------|------|------|
| `word` | string | ✅ | 要查询的日语单词 |
| `search_type` | string | ❌ | 搜索类型，默认为精确搜索 |

#### 搜索类型

| 类型 | 说明 | 使用场景 |
|------|------|----------|
| `exact` (默认) | 精确搜索 | 查找特定词条，先假名匹配，后汉字匹配 |
| `kana` | 假名精确匹配 | 根据假名读音查找 |
| `kanji` | 汉字智能匹配 | 根据汉字查找，支持多重表记 |
| `fuzzy` | 模糊搜索 | 查找包含关键词的所有词条 |

## 使用示例

### 精确搜索（推荐，默认方式）
```bash
# 查询"愛"的精确词条
curl "http://localhost:3000/search?word=愛"

# 效果相同
curl "http://localhost:3000/search?word=愛&search_type=exact"
```

### 假名搜索
```bash
# 根据假名查询
curl "http://localhost:3000/search?word=あい&search_type=kana"
```

### 汉字搜索
```bash
# 汉字智能搜索（支持多重表记）
curl "http://localhost:3000/search?word=愛&search_type=kanji"
```

### 模糊搜索
```bash
# 查找所有包含"愛"的词条
curl "http://localhost:3000/search?word=愛&search_type=fuzzy"
```

## 响应格式

### 成功响应
```json
{
  "success": true,
  "count": 1,
  "entries": [
    {
      "id": 123,
      "headword": "あい【愛】",
      "kana_reading": "あい",
      "kanji_writing": "愛",
      "part_of_speech": "名",
      "conjugation": null,
      "definition_text": "❶かわいがりいつくしむ気持ち。❷こいしたう気持ち。",
      "definition_html": "<div>...</div>",
      "data_id": "1234567",
      "data_type": "9",
      "raw_mdx_content": "..."
    }
  ],
  "query_info": {
    "word": "愛",
    "search_type": "exact",
    "duration_ms": 15
  }
}
```

### 错误响应
```json
{
  "success": false,
  "error": "查询词不能为空"
}
```

## 数据库统计

### 获取统计信息
```
GET /stats
```

### 响应示例
```json
{
  "success": true,
  "database": {
    "path": "obunsha_dict.db",
    "total_entries": 80615,
    "unique_headwords": 75432,
    "status": "已连接"
  },
  "api": {
    "version": "1.0.0",
    "supported_search_types": ["exact", "fuzzy", "kana", "kanji"]
  }
}
```

## 搜索策略详解

### 精确搜索 (exact)
1. **假名优先**: 首先尝试 `kana_reading = 查询词`
2. **汉字备选**: 假名无结果时，尝试汉字智能匹配
3. **适用场景**: 查找特定词条，获得最准确的结果

### 汉字智能搜索 (kanji)
1. **精确匹配**: 先尝试 `kanji_writing = 查询词`
2. **多重表记**: 无结果时搜索带点号的变体（如：可愛·可愛らしい）
3. **应用层过滤**: 确保匹配的是完整汉字，而非子串

### 模糊搜索 (fuzzy)
1. **包含匹配**: 使用 `LIKE '%查询词%'`
2. **结果较多**: 可能返回大量相关词条
3. **适用场景**: 探索性搜索，查找相关词汇

## 性能说明

- **响应时间**: 通常 < 50ms
- **数据库索引**: 已对 `headword`、`kana_reading`、`data_id` 建立索引
- **并发支持**: 支持多个同时查询
- **线程安全**: 使用连接池确保并发安全

## 启动服务

```bash
# 编译并启动服务器
cargo run --bin dict server

# 服务将在 http://localhost:3000 启动
```

## 注意事项

1. **字符编码**: 请确保查询参数使用UTF-8编码
2. **特殊字符**: URL中的特殊字符需要进行URL编码
3. **空查询**: 查询词不能为空，否则返回400错误
4. **大小写**: 日语查询对大小写不敏感

## 示例脚本

### Python 示例
```python
import requests
import json

# 精确搜索
response = requests.get('http://localhost:3000/search?word=愛')
data = response.json()
print(f"找到 {data['count']} 个结果")

# 模糊搜索
response = requests.get('http://localhost:3000/search?word=愛&search_type=fuzzy')
data = response.json()
print(f"模糊搜索找到 {data['count']} 个结果")
```

### JavaScript 示例
```javascript
// 精确搜索
fetch('http://localhost:3000/search?word=愛')
  .then(response => response.json())
  .then(data => {
    console.log(`找到 ${data.count} 个结果`);
    console.log('第一个词条:', data.entries[0]);
  });
```

---

**版本**: 1.0.0  
**更新时间**: 2024年12月  
**联系方式**: 如有问题请提交 Issue 