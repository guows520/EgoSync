# Story 9.1: 管家与角色设置分组支持折叠和批量展开

Status: review

## Story

As a 使用管家设置或角色设置的用户,
I want 每个设置分组可以独立折叠，并可一键全部展开或全部折叠,
so that 我能快速聚焦当前要编辑的设置，减少长页面滚动和视觉负担。

## Acceptance Criteria

1. **AC-1 管家设置分组可独立折叠**
   - Given 用户打开管家设置页
   - When 点击任一设置分组标题或其折叠按钮
   - Then 仅该分组内容在展开/折叠之间切换
   - And 其它分组状态保持不变
   - And 标题、摘要、状态提示仍然可见

2. **AC-2 角色设置分组可独立折叠**
   - Given 用户打开任一角色的设置页
   - When 点击任一设置分组标题或其折叠按钮
   - Then 仅该分组内容在展开/折叠之间切换
   - And 不触发保存、导入、删除或 MCP 权限变更

3. **AC-3 提供独立的全部展开与全部折叠按钮**
   - Given 页面存在多个设置分组
   - Then 页面同时提供“全部展开”和“全部折叠”两个独立按钮
   - When 点击“全部展开”
   - Then 当前页面所有一级设置分组均展开
   - When 点击“全部折叠”
   - Then 当前页面所有一级设置分组均折叠
   - And 已达到目标状态的按钮可保持可用或禁用，但不得产生副作用

4. **AC-4 折叠状态持久化且作用域隔离**
   - Given 用户调整过折叠状态
   - When 关闭并重新打开对应设置页或重启应用
   - Then 恢复上次状态
   - And 管家设置与角色设置使用不同的 localStorage key
   - And 读取到损坏或过期数据时安全回退到默认状态，不阻断页面渲染

5. **AC-5 嵌套 Skill 分组行为不回归**
   - Given “Skill 配置”内部存在内置、外部或自定义 Skill 的二级展开逻辑
   - When 折叠或展开一级“Skill 配置”分组
   - Then 二级 Skill 状态不被重置
   - And 导入、发现、启停、删除等现有功能保持可用

6. **AC-6 键盘与深色模式可用**
   - Given 用户使用键盘或深色模式
   - Then 分组标题使用可聚焦的原生按钮或等价语义控件
   - And Enter/Space 可切换，焦点环可见，`aria-expanded` 正确
   - And 折叠标题、边框、摘要、hover/active 状态在深色模式下可读

7. **AC-7 回归验证**
   - Then 管家使命宣言保存、简报时间、归档角色、Skill 管理、角色信息、主动性、Skill、MCP 等既有事务边界不变
   - And `npm run test:frontend` 与 `npm run build` 通过

## Tasks / Subtasks

- [x] Task 1: 提炼最小可复用折叠结构（AC: #1, #2, #3, #6）
  - [x] 1.1 以现有 `ButlerSettingsGroupedDemo.tsx` 与 `GroupedSettingsDemo.tsx` 为视觉和状态模型参考
  - [x] 1.2 优先在各生产页面内保留简单的 `Record<SectionId, boolean>`；只有两页出现真实重复且抽取更少代码时才创建共享组件
  - [x] 1.3 分组头使用 `button type="button"`、`aria-expanded`、Chevron 图标和明确焦点样式
  - [x] 1.4 顶部同时放置“全部展开”和“全部折叠”两个独立按钮，不合并为单一切换按钮

- [x] Task 2: 改造管家设置页（AC: #1, #3, #4, #5, #7）
  - [x] 2.1 明确并固定所有一级分组 ID，加入 `openSections`
  - [x] 2.2 增加全部展开/全部折叠控制
  - [x] 2.3 用独立 localStorage key 保存状态，异常 JSON 回退默认值
  - [x] 2.4 保持使命宣言、Skill 二级展开及所有保存逻辑原位，不改变调用顺序

- [x] Task 3: 改造角色设置页（AC: #2, #3, #4, #5, #7）
  - [x] 3.1 明确并固定所有一级分组 ID，加入 `expandedSections`
  - [x] 3.2 增加全部展开/全部折叠控制和独立持久化 key
  - [x] 3.3 确认折叠操作不会提交表单或调用 service

- [x] Task 4: 测试（AC: #1-#7）
  - [x] 4.1 覆盖单组切换、全部展开、全部折叠和互不影响
  - [x] 4.2 覆盖持久化恢复、损坏数据回退、键盘语义
  - [x] 4.3 覆盖折叠后重新展开时输入值与 Skill 二级状态未丢失
  - [x] 4.4 运行 `npm run test:frontend` 与 `npm run build`

## Dev Notes

- 生产页当前没有一级分组折叠状态；Demo 已提供两套可复用思路，应迁移行为而不是把 Demo 直接搬入生产页。
- 不要通过条件渲染导致分组内部状态被卸载并丢失。优先保留 DOM 并控制可见性；如使用条件渲染，必须证明所有编辑状态都由父层持有且不会重置。
- 不要引入新的状态管理库或手风琴依赖；当前需求用本地 React state 即可完成。
- 本 Story 不处理角色页面的信息重排、按钮文案和新增角色目标字段，它们属于 Story 9.2。

### Project Structure Notes

- 主要修改：
  - `egosync-app/src/components/butler/ButlerSettingsContent.tsx`
  - `egosync-app/src/components/role/SettingsTab.tsx`
- 参考实现：
  - `egosync-app/src/components/butler/ButlerSettingsGroupedDemo.tsx`
  - `egosync-app/src/components/role/GroupedSettingsDemo.tsx`
- 测试应放入现有前端测试目录并遵循当前 Vitest + Testing Library 习惯。

### References

- [Source: `_bmad-output/implementation-artifacts/investigations/ui-settings-display-packaging-issues-investigation.md` — Follow-up #3/#4/#7]
- [Source: `egosync-app/src/components/butler/ButlerSettingsContent.tsx`]
- [Source: `egosync-app/src/components/butler/ButlerSettingsGroupedDemo.tsx:174-193`]
- [Source: `egosync-app/src/components/role/SettingsTab.tsx`]
- [Source: `egosync-app/src/components/role/GroupedSettingsDemo.tsx:113-164`]

## Dev Agent Record

### Agent Model Used

Codex（GPT-5.6）

### Debug Log References

- `npm run test:frontend`：42 个测试文件、381 项测试通过。
- `npm run build`：TypeScript 与 Vite 生产构建通过；保留既有 chunk size 警告。
- `npx tauri build --bundles msi,nsis`：MSI 与 NSIS 两种 Windows bundle 构建通过；保留既有 Rust 编译警告。

### Completion Notes List

- 管家与角色设置均采用页面内 `Record<SectionId, boolean>`，提供独立分组切换、全部展开和全部折叠。
- 折叠状态使用隔离的 localStorage key 持久化；损坏 JSON 回退全部展开，内容 DOM 不卸载以保留输入和 Skill 二级状态。
- 新增单组、批量、持久化、损坏数据回退和输入状态保留测试。

### File List

- `egosync-app/src/components/butler/ButlerSettingsContent.tsx`
- `egosync-app/src/components/butler/ButlerSettingsContent.test.tsx`
- `egosync-app/src/components/role/SettingsTab.tsx`
- `egosync-app/src/components/role/SettingsTab.test.tsx`

### Change Log

- 2026-07-22：完成实现、自动化测试、生产构建与 Windows bundle 验证，状态更新为 Review。
