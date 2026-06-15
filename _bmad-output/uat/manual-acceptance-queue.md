# UAT 人工验收队列 — 2026-06-15

> 执行模式：**dev（开发阶段验收）**　范围：全部 140 用例
> 本队列由 `bmad-uat-run` Stage B 生成，供验收人 **boss** 逐条人工验收。
> semi 类：机器驱动流程 + 人确认；manual 类：人工执行并判定。
> 标注 `[降级]` 的用例原为 auto，因项目无 E2E 自动化脚本（tests/e2e 为空）转入人工验收。

## 队列概览

| 分类 | 数量 |
|---|---|
| 可执行人工队列 | 129 |
| ├─ semi（机器跑+人确认） | 54 |
| ├─ manual（纯人工） | 42 |
| └─ [降级]原auto转人工 | 33 |
| Blocked（缺环境/物料） | 6 |

## 可执行人工验收清单（按 story 分组）

> 验收时请在对应用例 md 文件填写「实际结果」与「测试结论」（Pass/Fail）。

### 1-1-tauri-desktop-app-existing-ui

| 用例编号 | 测试模块 | 模式 | 隔离 | 优先级 | 备注 |
|---|---|---|---|---|---|
| UAT-APP-001 | 桌面应用启动 | semi | read-only | 高 |  |
| UAT-APP-004 | 桌面应用视觉一致性 | manual | read-only | 中 |  |
| UAT-APP-005 | 桌面应用启动异常处理 | semi | write-isolated | 中 |  |

### 1-2-rust-frontend-test-infrastructure

| 用例编号 | 测试模块 | 模式 | 隔离 | 优先级 | 备注 |
|---|---|---|---|---|---|
| UAT-QA-005 | Rust 测试自动发现 | semi | write-isolated | 中 |  |

### 1-3-component-domain-split-visual-zero-regression

| 用例编号 | 测试模块 | 模式 | 隔离 | 优先级 | 备注 |
|---|---|---|---|---|---|
| UAT-UI-002 | 移除 Pitch Mode 演示条 | manual | read-only | 高 |  |
| UAT-UI-003 | 弹窗交互保留 | manual | read-only | 中 |  |

### 1-4-theme-toggle-design-tokens

| 用例编号 | 测试模块 | 模式 | 隔离 | 优先级 | 备注 |
|---|---|---|---|---|---|
| UAT-THEME-001 | 主题切换即时生效 | semi | write-isolated | 高 |  |
| UAT-THEME-002 | 主题偏好持久化 | semi | write-isolated | 高 |  |
| UAT-THEME-003 | 首次打开主题默认值 | semi | write-isolated | 中 |  |
| UAT-THEME-004 | 无障碍-减少动效 | manual | write-isolated | 中 |  |
| UAT-THEME-007 | 字体加载与路径别名/质量门禁 | semi | read-only | 低 |  |

### 1-5-llm-api-key-keyring-storage

| 用例编号 | 测试模块 | 模式 | 隔离 | 优先级 | 备注 |
|---|---|---|---|---|---|
| UAT-KEY-001 | API Key 安全存储 | semi | write-isolated | 高 |  |
| UAT-KEY-002 | API Key 安全存储 | manual | write-isolated | 中 |  |
| UAT-KEY-003 | API Key 安全存储 | semi | read-only | 高 |  |
| UAT-KEY-004 | API Key 安全存储 | semi | write-isolated | 中 |  |

### 1-6-llm-provider-connection-test

| 用例编号 | 测试模块 | 模式 | 隔离 | 优先级 | 备注 |
|---|---|---|---|---|---|
| UAT-LLM-001 | LLM Provider 连接测试 | semi | write-isolated | 高 |  |
| UAT-LLM-002 | LLM Provider 连接测试 | auto | write-isolated | 高 | [降级]auto→人工 |
| UAT-LLM-003 | LLM Provider 连接测试 | semi | write-isolated | 中 |  |
| UAT-LLM-004 | LLM Provider 连接测试 | semi | write-isolated | 中 |  |
| UAT-LLM-005 | LLM Provider 连接测试 | auto | write-isolated | 中 | [降级]auto→人工 |
| UAT-LLM-006 | LLM Provider 连接测试 | semi | write-isolated | 高 |  |
| UAT-LLM-007 | LLM Provider 连接测试 | semi | write-isolated | 低 |  |

### 1-7-butler-first-streaming-conversation

| 用例编号 | 测试模块 | 模式 | 隔离 | 优先级 | 备注 |
|---|---|---|---|---|---|
| UAT-CHAT-001 | 管家流式对话 | semi | write-isolated | 高 |  |
| UAT-CHAT-002 | 管家流式对话 | semi | write-isolated | 高 |  |
| UAT-CHAT-003 | 管家流式对话 | manual | write-isolated | 高 |  |
| UAT-CHAT-004 | 管家流式对话 | semi | write-isolated | 中 |  |
| UAT-CHAT-005 | 管家流式对话 | semi | write-isolated | 中 |  |
| UAT-CHAT-006 | 管家流式对话 | semi | write-isolated | 中 |  |

### 1-8-onboarding-five-step-first-role

| 用例编号 | 测试模块 | 模式 | 隔离 | 优先级 | 备注 |
|---|---|---|---|---|---|
| UAT-ONBOARD-001 | 五步引导首角色 | semi | write-isolated | 高 |  |
| UAT-ONBOARD-002 | 五步引导首角色 | semi | write-isolated | 高 |  |
| UAT-ONBOARD-003 | 五步引导首角色 | auto | write-isolated | 高 | [降级]auto→人工 |
| UAT-ONBOARD-004 | 五步引导首角色 | semi | write-isolated | 中 |  |
| UAT-ONBOARD-005 | 五步引导首角色 | semi | write-isolated | 中 |  |

### 1-9-role-sidebar-breathing-animation

| 用例编号 | 测试模块 | 模式 | 隔离 | 优先级 | 备注 |
|---|---|---|---|---|---|
| UAT-SIDEBAR-001 | 角色侧栏呼吸动画 | semi | read-only | 高 |  |
| UAT-SIDEBAR-002 | 角色侧栏呼吸动画 | manual | read-only | 中 |  |
| UAT-SIDEBAR-003 | 角色侧栏呼吸动画 | manual | read-only | 中 |  |
| UAT-SIDEBAR-004 | 角色侧栏呼吸动画 | semi | read-only | 中 |  |

### 2-0-opencode-sidecar-agent-bridge

| 用例编号 | 测试模块 | 模式 | 隔离 | 优先级 | 备注 |
|---|---|---|---|---|---|
| UAT-SIDECAR-001 | 对话引擎可用性 | semi | write-isolated | 高 |  |
| UAT-SIDECAR-002 | 对话引擎自愈恢复 | semi | write-isolated | 高 |  |
| UAT-SIDECAR-003 | 应用退出资源回收 | manual | write-isolated | 中 |  |
| UAT-SIDECAR-004 | 引擎缺失时的降级可用性 | manual | write-isolated | 中 |  |

### 2-0b-role-agent-mapping-permissions

| 用例编号 | 测试模块 | 模式 | 隔离 | 优先级 | 备注 |
|---|---|---|---|---|---|
| UAT-AGENT-001 | 角色身份同步 | semi | write-isolated | 高 |  |
| UAT-AGENT-002 | 角色编辑生效 | semi | write-isolated | 中 |  |
| UAT-AGENT-003 | 角色归档与删除 | semi | write-isolated | 中 |  |
| UAT-PERM-001 | 角色权限边界 | manual | write-isolated | 高 |  |
| UAT-PERM-002 | 管家权限与全量一致性 | semi | write-isolated | 中 |  |

### 2-0c-dialog-engine-switch-to-opencode

| 用例编号 | 测试模块 | 模式 | 隔离 | 优先级 | 备注 |
|---|---|---|---|---|---|
| UAT-ENGINE-001 | 引擎切换后对话连续性 | semi | write-isolated | 高 |  |
| UAT-ENGINE-002 | 引擎不可用时的降级提示 | semi | write-isolated | 高 |  |
| UAT-ENGINE-003 | 停止按钮夺回控制权 | manual | write-isolated | 高 |  |
| UAT-ENGINE-004 | 角色对话路由 | semi | write-isolated | 高 |  |
| UAT-ENGINE-005 | 角色提议确认流程 | manual | write-isolated | 高 |  |
| UAT-ENGINE-006 | 引擎崩溃后对话自愈 | semi | write-isolated | 高 |  |

### 2-1-role-crud-archive-delete

| 用例编号 | 测试模块 | 模式 | 隔离 | 优先级 | 备注 |
|---|---|---|---|---|---|
| UAT-ROLE-001 | 角色管理-编辑 | auto | write-isolated | 高 | [降级]auto→人工 |
| UAT-ROLE-002 | 角色管理-归档 | auto | write-isolated | 高 | [降级]auto→人工 |
| UAT-ROLE-003 | 角色管理-恢复 | auto | write-isolated | 高 | [降级]auto→人工 |
| UAT-ROLE-004 | 角色管理-永久删除 | auto | write-isolated | 高 | [降级]auto→人工 |
| UAT-ROLE-005 | 角色管理-保底保护 | auto | write-isolated | 高 | [降级]auto→人工 |
| UAT-ROLE-006 | 角色管理-删除确认 | auto | write-isolated | 中 | [降级]auto→人工 |
| UAT-ROLE-007 | 角色管理-新建 | auto | write-isolated | 中 | [降级]auto→人工 |

### 2-10-role-skill-config-proactivity-ui

| 用例编号 | 测试模块 | 模式 | 隔离 | 优先级 | 备注 |
|---|---|---|---|---|---|
| UAT-SKILL-001 | 角色 Skill 配置 | auto | write-isolated | 高 | [降级]auto→人工 |
| UAT-SKILL-002 | 角色主动性配置 | auto | write-isolated | 高 | [降级]auto→人工 |
| UAT-SKILL-003 | 管家 Skill 配置 | auto | write-isolated | 中 | [降级]auto→人工 |
| UAT-SKILL-004 | 关闭 Skill 阻断 | semi | write-isolated | 高 |  |
| UAT-SKILL-005 | 关闭 Skill 阻断 | semi | write-isolated | 高 |  |
| UAT-SKILL-006 | Skill 生效边界 | semi | write-isolated | 中 |  |

### 2-11-custom-skill-md-import-role-binding

| 用例编号 | 测试模块 | 模式 | 隔离 | 优先级 | 备注 |
|---|---|---|---|---|---|
| UAT-IMPORT-001 | 自定义 Skill 导入 | auto | write-isolated | 高 | [降级]auto→人工 |
| UAT-IMPORT-002 | 自定义 Skill 导入校验 | manual | write-isolated | 高 |  |
| UAT-IMPORT-003 | 自定义 Skill 重复导入 | auto | write-isolated | 高 | [降级]auto→人工 |
| UAT-IMPORT-004 | 自定义 Skill 角色绑定 | auto | write-isolated | 高 | [降级]auto→人工 |
| UAT-IMPORT-005 | 元 Skill 向后兼容 | auto | write-isolated | 中 | [降级]auto→人工 |

### 2-12-opencode-ecosystem-skill-discovery-import

| 用例编号 | 测试模块 | 模式 | 隔离 | 优先级 | 备注 |
|---|---|---|---|---|---|
| UAT-DISCOVER-001 | opencode Skill 发现边界 | auto | write-isolated | 高 | [降级]auto→人工 |
| UAT-DISCOVER-002 | opencode Skill 扫描 | auto | write-isolated | 高 | [降级]auto→人工 |
| UAT-DISCOVER-003 | opencode Skill 无效项跳过 | manual | write-isolated | 中 |  |
| UAT-DISCOVER-004 | opencode Skill 导入与角色启用 | auto | write-isolated | 高 | [降级]auto→人工 |
| UAT-DISCOVER-005 | 管家 opencode Skill 发现导入 | auto | write-isolated | 中 | [降级]auto→人工 |
| UAT-DISCOVER-006 | V1 不做远程市场 | manual | read-only | 低 |  |

### 2-13-mcp-server-list-role-access

| 用例编号 | 测试模块 | 模式 | 隔离 | 优先级 | 备注 |
|---|---|---|---|---|---|
| UAT-MCP-001 | MCP server 管理 | auto | write-isolated | 高 | [降级]auto→人工 |
| UAT-MCP-002 | MCP secret 安全持久化 | auto | write-isolated | 高 | [降级]auto→人工 |
| UAT-MCP-003 | MCP 连接测试与失败降级 | semi | write-isolated | 高 |  |
| UAT-MCP-004 | 角色级 MCP 访问隔离 | semi | write-isolated | 高 |  |
| UAT-MCP-005 | opencode.json MCP 同步保留 | auto | write-isolated | 高 | [降级]auto→人工 |
| UAT-MCP-006 | MCP 删除与孤儿绑定清理 | auto | write-isolated | 中 | [降级]auto→人工 |

### 2-2-role-view-switch-butler

| 用例编号 | 测试模块 | 模式 | 隔离 | 优先级 | 备注 |
|---|---|---|---|---|---|
| UAT-NAV-001 | 角色视图切换 | semi | write-isolated | 高 |  |
| UAT-NAV-002 | 角色对话历史隔离 | auto | write-isolated | 高 | [降级]auto→人工 |
| UAT-NAV-003 | 切回管家 | semi | read-only | 高 |  |
| UAT-NAV-004 | 历史对话列表过滤 | auto | write-isolated | 中 | [降级]auto→人工 |
| UAT-NAV-005 | 角色身份呈现 | manual | write-isolated | 高 |  |

### 2-3-butler-intent-routing

| 用例编号 | 测试模块 | 模式 | 隔离 | 优先级 | 备注 |
|---|---|---|---|---|---|
| UAT-DELEG-001 | 管家委派 | manual | write-isolated | 高 |  |
| UAT-DELEG-002 | 多角色并行委派 | manual | write-isolated | 高 |  |
| UAT-DELEG-003 | 委派写入角色历史 | semi | write-isolated | 高 |  |
| UAT-DELEG-004 | 模糊意图追问 | manual | write-isolated | 中 |  |
| UAT-DELEG-005 | 委派目标无效兜底 | manual | write-isolated | 高 |  |
| UAT-DELEG-006 | 跨角色全局同步 | manual | write-isolated | 中 |  |
| UAT-DELEG-007 | 事实记忆与任务分流 | manual | write-isolated | 高 |  |
| UAT-DELEG-008 | 禁止嵌套委派 | manual | write-isolated | 中 |  |

### 2-4-role-personalized-tone

| 用例编号 | 测试模块 | 模式 | 隔离 | 优先级 | 备注 |
|---|---|---|---|---|---|
| UAT-TONE-001 | 个性描述编辑 | auto | write-isolated | 高 | [降级]auto→人工 |
| UAT-TONE-002 | 语调模板参考 | auto | read-only | 中 | [降级]auto→人工 |
| UAT-TONE-003 | 角色语调差异 | manual | write-isolated | 高 |  |
| UAT-TONE-004 | 个性更新即时生效 | manual | write-isolated | 高 |  |
| UAT-TONE-005 | 空个性兼容 | manual | write-isolated | 中 |  |
| UAT-TONE-006 | 角色身份隔离 | manual | write-isolated | 高 |  |

### 2-5-role-emergence-suggestion

| 用例编号 | 测试模块 | 模式 | 隔离 | 优先级 | 备注 |
|---|---|---|---|---|---|
| UAT-EMERG-001 | 角色涌现建议 | manual | write-isolated | 高 |  |
| UAT-EMERG-002 | 角色涌现建议 | semi | write-isolated | 高 |  |
| UAT-EMERG-003 | 角色涌现建议 | manual | write-isolated | 高 |  |
| UAT-EMERG-004 | 角色涌现建议 | manual | read-only |  |  |
| UAT-EMERG-005 | 角色涌现建议 | manual | write-isolated | 低 |  |

### 2-6-conversation-memory-extraction

| 用例编号 | 测试模块 | 模式 | 隔离 | 优先级 | 备注 |
|---|---|---|---|---|---|
| UAT-MEM-001 | 对话记忆自动提炼 | semi | write-isolated | 高 |  |
| UAT-MEM-002 | 对话记忆自动提炼 | semi | write-isolated | 高 |  |
| UAT-MEM-003 | 对话记忆自动提炼 | manual | write-isolated | 高 |  |
| UAT-MEM-004 | 对话记忆自动提炼 | manual | write-isolated | 中 |  |
| UAT-MEM-005 | 对话记忆自动提炼 | semi | write-isolated | 中 |  |
| UAT-MEM-006 | 对话记忆自动提炼 | semi | write-isolated | 中 |  |

### 2-7-memory-panel-traceability

| 用例编号 | 测试模块 | 模式 | 隔离 | 优先级 | 备注 |
|---|---|---|---|---|---|
| UAT-MEMPANEL-001 | 记忆面板与来源追溯 | auto | read-only | 高 | [降级]auto→人工 |
| UAT-MEMPANEL-002 | 记忆面板与来源追溯 | auto | read-only | 中 | [降级]auto→人工 |
| UAT-MEMPANEL-003 | 记忆面板与来源追溯 | semi | read-only | 高 |  |
| UAT-MEMPANEL-004 | 记忆面板与来源追溯 | auto | read-only | 中 | [降级]auto→人工 |
| UAT-MEMPANEL-005 | 记忆面板与来源追溯 | manual | read-only | 中 |  |
| UAT-MEMPANEL-006 | 记忆面板与来源追溯 | semi | write-isolated | 中 |  |

### 2-8-selective-memory-forget

| 用例编号 | 测试模块 | 模式 | 隔离 | 优先级 | 备注 |
|---|---|---|---|---|---|
| UAT-FORGET-001 | 选择性遗忘 | semi | write-isolated | 高 |  |
| UAT-FORGET-002 | 选择性遗忘 | auto | read-only | 高 | [降级]auto→人工 |
| UAT-FORGET-003 | 选择性遗忘 | semi | write-isolated | 高 |  |
| UAT-FORGET-004 | 选择性遗忘 | manual | write-isolated | 中 |  |
| UAT-FORGET-005 | 选择性遗忘 | semi | write-isolated | 中 |  |

### 2-9-reasoning-transparency-uncertainty

| 用例编号 | 测试模块 | 模式 | 隔离 | 优先级 | 备注 |
|---|---|---|---|---|---|
| UAT-REASON-001 | 推理透明与不确定性 | manual | read-only | 高 |  |
| UAT-REASON-002 | 推理透明与不确定性 | manual | read-only | 中 |  |
| UAT-REASON-003 | 推理透明与不确定性 | manual | read-only | 高 |  |
| UAT-REASON-004 | 推理透明与不确定性 | manual | read-only | 高 |  |
| UAT-REASON-005 | 推理透明与不确定性 | manual | write-isolated | 中 |  |
| UAT-REASON-006 | 推理透明与不确定性 | manual | write-isolated | 中 |  |

## Blocked 用例（需 boss 提供环境/物料后转入可执行）

| 用例编号 | 测试模块 | story_key | 阻塞原因 |
|---|---|---|---|
| UAT-APP-002 | 桌面应用打包分发 | 1-1-tauri-desktop-app-existing-ui | 需干净测试机/全新VM快照（未装过EgoSync的Windows） |
| UAT-APP-003 | 桌面应用启动性能 | 1-1-tauri-desktop-app-existing-ui | 需基准硬件（中端Win/M1 Mac），云虚机不算 |
| UAT-QA-004 | CI 三平台门禁 | 1-2-rust-frontend-test-infrastructure | 需GitHub仓库+启用Actions(含额度)触发三平台CI matrix |
| UAT-THEME-005 | 无障碍-对比度 | 1-4-theme-toggle-design-tokens | 需对比度检测工具(axe DevTools等)客观测量WCAG |
| UAT-THEME-006 | 浅色主题视觉零回归 | 1-4-theme-toggle-design-tokens | 需Story1.3完成态浅色截图作token化前基准 |
| UAT-UI-001 | 界面视觉零回归 | 1-3-component-domain-split-visual-zero-regression | 需拆分前6张基线截图作视觉零回归基准 |

> 详见 `data/data-requests/data-request-2026-06-14.md`。提供后重跑本范围即可解除阻塞。
