# Story 9.2: 角色设置结构优化并在新建角色时填写目标

Status: review

## Story

As a 管理数字角色的用户,
I want 在创建角色时直接填写目标，并在角色设置中按清晰的事务边界编辑信息、Skill 与 MCP Server,
so that 角色从创建开始就具备明确目标，后续配置也不会因为模糊的统一保存按钮而产生误解。

## Acceptance Criteria

1. **AC-1 新建角色弹窗提供目标输入**
   - Given 用户打开“添加新角色”弹窗
   - Then 在角色名称附近显示“目标”多行输入框
   - And 文案说明目标用于描述该角色长期负责或达成的方向
   - And 目标允许为空，不新增无依据的必填限制

2. **AC-2 创建请求携带目标**
   - Given 用户填写角色名称和目标
   - When 点击创建
   - Then `roleService.create()` payload 包含去除首尾空白后的 `goal`
   - And 空目标按 `undefined` 传递，不传空白字符串
   - And 成功创建后新角色头部与设置页能显示该目标

3. **AC-3 自定义 Skill 纳入 Skill 配置**
   - Given 用户打开角色设置页
   - Then “自定义 Skill”作为“Skill 配置”内部内容呈现
   - And 不再作为与“Skill 配置”并列的一级设置分组
   - And 导入、删除、复用范围和运行时刷新逻辑保持不变

4. **AC-4 角色信息拥有独立保存按钮**
   - Given 用户修改角色名称、目标、图标或颜色
   - Then “角色信息”分组内显示“保存角色信息”按钮
   - And 按钮视觉层级与管家页面“保存使命宣言”相似
   - When 点击该按钮
   - Then 复用现有 `handleSave` 只保存角色信息及当前既有基础设置事务
   - And 显示保存中、成功和失败反馈

5. **AC-5 移除底部统一保存按钮**
   - Then 页面最底部不再显示“保存更改”按钮
   - And Skill/MCP 的现有即时保存行为不被改造成统一提交
   - And 页面不得存在两个会调用相同 `handleSave` 的主按钮

6. **AC-6 MCP 文案更新**
   - Then “外部 MCP 工具”更改为“MCP Server配置”
   - And 相关可见标题、辅助文本及前端测试断言同步更新
   - And 后端 command、类型名、数据库字段和协议名称不因文案改变而重命名

7. **AC-7 深色模式和无障碍不回归**
   - Then 新增目标输入、保存按钮及重排后的 Skill/MCP 区域包含现有风格一致的 dark variants
   - And label 与输入框正确关联，保存按钮可键盘操作

8. **AC-8 测试与构建通过**
   - Then 增加创建 payload、空目标、保存按钮位置、底部按钮移除、Skill 归属与 MCP 文案测试
   - And `npm run test:frontend` 与 `npm run build` 通过

## Tasks / Subtasks

- [x] Task 1: 新角色目标字段（AC: #1, #2, #7）
  - [x] 1.1 在 `AddRoleModal` 增加 `goal` state 和多行输入
  - [x] 1.2 创建时传 `goal: goal.trim() || undefined`
  - [x] 1.3 沿用现有错误和 loading 状态，不增加新的后端接口

- [x] Task 2: 重排角色设置页面（AC: #3, #4, #5, #6）
  - [x] 2.1 将“自定义 Skill”完整 JSX 移入“Skill 配置”，保留所有事件处理函数和 state
  - [x] 2.2 将现有 `handleSave` 按钮移入角色信息分组并改文案为“保存角色信息”
  - [x] 2.3 删除页面底部统一按钮，不删除 `handleSave`
  - [x] 2.4 将“外部 MCP 工具”显示文案改为“MCP Server配置”

- [x] Task 3: 事务边界回归检查（AC: #3-#6）
  - [x] 3.1 验证角色信息按钮只调用既有角色更新路径
  - [x] 3.2 验证 Skill 启停、导入、删除仍按原路径即时持久化
  - [x] 3.3 验证 MCP 权限切换仍按原路径即时持久化

- [x] Task 4: 测试（AC: #1-#8）
  - [x] 4.1 mock `roleService.create` 并断言有值/空值 payload
  - [x] 4.2 断言“保存角色信息”位于角色信息分组，且“保存更改”不存在
  - [x] 4.3 断言“自定义 Skill”不再是一级分组，“MCP Server配置”可见
  - [x] 4.4 运行 `npm run test:frontend` 与 `npm run build`

## Dev Notes

- `Role`/创建接口的数据模型已经支持 `goal`，缺口仅在新增弹窗 state、输入和 payload；不要创建新的数据库迁移或 Rust command。
- `SettingsTab` 已有 `handleSave`，不要复制第二套保存函数。
- Skill/MCP 当前具有独立持久化逻辑。本 Story 的核心是让按钮位置与真实事务边界一致，而不是重写保存架构。
- Story 9.1 会为一级分组增加折叠结构；实施时如果 9.1 已完成，应把重排内容放入其对应 section body，不应绕开折叠组件。

### Project Structure Notes

- 主要修改：
  - `egosync-app/src/components/modals/AddRoleModal.tsx`
  - `egosync-app/src/components/role/SettingsTab.tsx`
- 相关模型与服务（优先只读验证）：
  - `egosync-app/src/types/role.ts`
  - `egosync-app/src/services/roleService.ts`
- 不修改 Rust Prompt。

### References

- [Source: `_bmad-output/implementation-artifacts/investigations/ui-settings-display-packaging-issues-investigation.md` — Follow-up #3/#4/#7]
- [Source: `egosync-app/src/components/modals/AddRoleModal.tsx:14-34`]
- [Source: `egosync-app/src/components/role/SettingsTab.tsx:209-240`]
- [Source: `egosync-app/src/components/role/SettingsTab.tsx:551`]
- [Source: `egosync-app/src/components/role/SettingsTab.tsx:786`]
- [Source: `egosync-app/src/components/role/SettingsTab.tsx:1012-1013`]

## Dev Agent Record

### Agent Model Used

Codex（GPT-5.6）

### Debug Log References

- `npm run test:frontend`：42 个测试文件、381 项测试通过。
- `npm run build`：TypeScript 与 Vite 生产构建通过；保留既有 chunk size 警告。
- `npx tauri build --bundles msi,nsis`：MSI 与 NSIS 两种 Windows bundle 构建通过；保留既有 Rust 编译警告。

### Completion Notes List

- 新增角色弹窗支持填写目标，并以 `goal.trim() || undefined` 维持既有创建契约。
- “保存角色信息”移入角色信息分组，移除底部统一保存按钮；自定义 Skill 保持在 Skill 配置内，MCP 分组改名为“MCP Server配置”。
- 角色信息、Skill 与 MCP 的既有持久化边界保持不变并由回归测试覆盖。

### File List

- `egosync-app/src/components/modals/AddRoleModal.tsx`
- `egosync-app/src/components/modals/AddRoleModal.test.tsx`
- `egosync-app/src/components/role/SettingsTab.tsx`
- `egosync-app/src/components/role/SettingsTab.test.tsx`

### Change Log

- 2026-07-22：完成实现、自动化测试、生产构建与 Windows bundle 验证，状态更新为 Review。
