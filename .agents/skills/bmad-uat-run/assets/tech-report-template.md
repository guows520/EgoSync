# UAT 技术诊断报告 — {{date}}

**验收范围:** {{scope}}　**执行模式:** {{mode}}
**关联业务报告:** {{business_report_path}}（按用例编号关联）

> 本报告供 `bmad-investigate` 立案与 `bmad-quick-dev` / `bmad-dev-story` 修复使用。
> UAT 为黑盒验证：提供业务证据与接口锚点，不做代码 path:line 归因（由 bmad-investigate 完成）。

---

## 失败用例（每条含五要素证据 DNA）

### {{用例编号}} — {{业务场景}}

1. **是什么失败**
   - 用例编号: {{用例编号}}
   - story_key: {{story_key}}
   - 业务场景: {{业务场景}}

2. **怎么复现**
   - 测试步骤: {{步骤摘要}}
   - 数据快照ID / 专属实体ID: {{snapshot_id}}（可重放）
   - 执行模式: {{exec_mode}}　破坏性: {{destructive}}

3. **期望 vs 实际**
   - 期望: {{expected}}
   - 实际: {{actual}}
   - 精确 diff: {{diff}}

4. **第一现场证据（stronghold 候选）**
   - 请求: {{request}}
   - 响应: {{response}}
   - 报错堆栈: {{stack}}
   - 日志时间戳: {{log_ts}}

5. **接口锚点**
   - 失败 API 端点 / 接口路径: {{endpoint}}

---

## 阻塞用例（Blocked）

| 用例编号 | story_key | 阻塞原因 | 阻塞证据 |
|---|---|---|---|
| | | | |

## 交接建议

- 建议对上述 Fail 项运行 `bmad-investigate`，以"接口锚点 + 第一现场证据"为 stronghold 立案。
- 根因确认后由 `bmad-quick-dev` 或 `bmad-dev-story` 修复，修复后重跑本范围。
