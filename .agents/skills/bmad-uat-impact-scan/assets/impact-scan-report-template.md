# UAT 影响面扫描报告 — {{date}}

**变更范围:** {{range}}
**改动文件数:** {{changed_file_count}}

---

## 清单①：建议重验范围（代码动了，用例仍有效）

| story_key | 改动文件数 | 关联用例数 | 建议 |
|---|---|---|---|
| | | | |

## 清单②：疑似过期用例（story 语义已变，用例锚点落后）

| 用例编号 | story_key | 锚点commit | 当前commit | 过期原因 | 处置 |
|---|---|---|---|---|---|
| | | | | | ⚠️待复核 |

---

## 汇总建议

- **本次回归建议范围:** {{scope_summary}}
- **需先复核重建:** {{stale_count}} 条过期用例
- **推荐下一步:**
  1. 复核过期用例 → `bmad-uat-build-cases`（增量重建，人确认后）
  2. 备数据 → `bmad-uat-prepare-data`
  3. 执行回归 → `bmad-uat-run --mode=regression`
