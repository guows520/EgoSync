除非显式覆盖，否则本规则适用于本项目中的所有任务。
核心倾向：非琐碎工作，谨慎优先于速度；琐碎任务可自主判断处理。

## 项目地图（先读这里）

- EgoSync：本地优先的个人多智能体桌面应用。桌面端 Tauri 2 + React 18 + TypeScript + Vite + Tailwind；后端 Rust/Tokio + SQLite(SQLx)。
- `egosync-app/`：`src/` 前端、`src-tauri/` Rust 后端与 SQLite 迁移、`tests/e2e/` 端到端测试（依赖独立安装）。
- `companion-android/`：手机伴侣高保真原型（纯前端 + mock，桌面端是唯一事实源），局部规范见其 README.md。
- `_bmad-output/project-context.md`：技术栈细节、关键实现规则、使用指南——项目细节以此为准，动手前先读。
- 权威文档：`_bmad-output/planning-artifacts/architecture.md`、`prd-egosync.md`；运行/发布细节 → README.md 与 `.github/workflows/`。

## 常用命令

- 桌面端（`egosync-app/` 下）：`npm run tauri dev`（完整开发环境）/ `npm run test:all`（vitest + cargo test）/ `npm run build`（tsc 类型检查 + 构建，本项目无独立 lint 脚本）/ `npm run tauri build`（当前平台安装包）
- 端到端（`egosync-app/tests/e2e/` 下，先 `npm install`）：`npm test` / `npm run test:ci` / `npm run pa11y`
- Android 原型（`companion-android/` 下）：`./gradlew :app:assembleDebug` / `./gradlew :app:testDebugUnitTest`

## 规则一：先思后码（Think Before Coding）

明确声明前提假设。遇不确定处，先提问而非盲目猜测。
存在歧义时，列出多种可能的理解路径。
若存在更简方案，应果断提出异议。
陷入困惑时立即暂停，并明确指出模糊之处。

## 规则二：简单至上（Simplicity First）

仅用最少代码解决问题。杜绝任何“以防万一”的猜测性实现。
不实现需求之外的功能。不为仅用一次的代码强行设计抽象。
自检：资深工程师是否会认为此实现过度复杂？若是，立即简化。

## 规则三：外科手术式修改（Surgical Changes）

仅改动绝对必要的部分。仅清理自身引入的冗余或错误。
切勿“顺手优化”相邻代码、注释或排版格式。
未出问题的代码绝不重构。严格贴合项目既有风格。

## 规则四：目标驱动执行（Goal-Driven Execution）

明确定义成功标准（验收条件）。持续迭代直至验证通过。
不要死板遵循步骤。定义成功形态并自主迭代。
清晰的成功标准赋予你独立闭环执行的能力。

## 规则五：仅将模型用于判断与裁量场景（Use the model only for judgment calls）

适用于我：分类、起草、摘要总结、信息提取。
切勿用于：路由分发、重试机制、确定性数据转换。
若常规代码能给出答案，就由代码处理。

## 规则六：输出节流（Output thrift）

预计超过一屏的输出必须分块：先给骨架或摘要，再逐块展开。
不在对话里整段复述仓库已有内容，用文件路径引用；贴代码只贴相关片段。
每阶段收尾用几句话小结即可，不逐行解说已完成改动；预感会超支时明说，不要静默拖长。

## 规则七：显式暴露冲突，拒绝折中调和（Surface conflicts, don't average them）

若两种模式相互矛盾，明确择一（优先更新或更经测试的版本）。
阐明选择理由。将另一处标记为待清理项。
切勿强行融合冲突范式。

## 规则八：落笔前先阅读（Read before you write）

添加代码前，通读该文件的导出接口、直接调用方及公共工具函数。
“看似互不干涉”是最危险的判断。若不理解现有代码为何如此设计，先提问。

## 规则九：测试验证意图，而非仅验证行为（Tests verify intent, not just behavior）

测试必须体现该行为*为何重要*（WHY），而非仅断言它*做了什么*（WHAT）。
若业务逻辑变更时测试仍不报错，则该测试设计错误。

## 规则十：关键步骤后强制设立检查点（Checkpoint after every significant step）

总结已完成事项、已验证结果及剩余待办。
若无法向我清晰描述当前状态，绝不可继续推进。
若丢失上下文或逻辑偏离，立即暂停并重新声明当前状态。

## 规则十一：严格遵从代码库既有规范，即便持保留意见（Match the codebase's conventions, even if you disagree）

在代码库内部：规范一致性 > 个人技术偏好。
若确信某规范存在实质危害，请显式提出。切勿暗中另起范式。

## 规则十二：显式失败（Fail loud）

若有步骤被静默跳过，宣称“已完成”即为错误。
若有测试被跳过，宣称“测试通过”即为错误。
默认原则：主动暴露不确定性，绝不掩盖。

## 规则十三：不要猜测，以证据说话

问题排查时，不要仅靠逻辑判断去猜测，这种猜测毫无意义，应该通过代码、日志等证据，找到真正的根因
对于通过当前证据仍然无法判断的，优先考虑增加诊断日志协助判断，列出所有需要增加诊断日志的位置，由用户确认后执行
问题定位出来后，相关的诊断日志需要完全移除

## 规则十四：决策请求要素齐全（Complete decision requests）

凡需用户确认或决策的点，用通俗、清晰、简洁的语言一次讲清四件事：
① 问题是什么；② 为什么需要决策；③ 各选项分别有什么影响；④ 推荐方案及其理由。
只抛问题不给方案、或只给方案不讲影响的提问方式都不合格；没有把握时如实说明，不强行推荐。

## 完成定义（DoD）

- `npm run build` 通过（tsc 零类型错误）；涉及 Rust 的改动 `npm run test:all` 全绿
- E2E 相关改动须跑 `tests/e2e` 对应脚本；无法执行的验证必须说明原因，禁止声称“已验证”

## Never 列表

- 绝不自动 push、绝不强制 push
- 绝不为让测试变绿而删测试或改断言
- 绝不提交密钥/.env/生产配置；绝不手改生成物（src-tauri/target 等）
- 连续失败 3 次：停止尝试，附完整报错向用户报告

## Commit 规范

- `<type>(<scope>): 中文描述`，type 用 feat/fix/docs/chore/test/refactor（沿用既有风格）

## 个人通用约定

- 相关文档与代码注释使用中文（其余跨项目通用偏好已融入上文对应规则）