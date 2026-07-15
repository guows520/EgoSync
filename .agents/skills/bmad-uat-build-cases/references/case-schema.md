# UAT 用例 Schema

UAT 用例资产的完整字段定义 = 10 个业务字段 + 4 个机器治理字段。

## 业务字段（业务验收主体）

| 字段 | 说明 |
|---|---|
| 用例编号 | 唯一标识，格式 `UAT-<MODULE>-<NNN>`，如 UAT-LOGIN-001 |
| 测试模块 | 所属业务模块/功能区域，如 订单管理 |
| 用例标题 | 简明描述测试场景，如 使用有效账号密码成功登录 |
| 业务场景 | 该用例对应的实际业务价值/用户故事（UAT 特有字段） |
| 优先级 | 高（核心阻断性流程）/ 中（常规功能）/ 低（边缘场景） |
| 前置条件 | 执行前必须满足的系统环境、账号权限、数据状态（业务语言描述） |
| 测试步骤 | 用户具体操作，按序号排列，业务语言（"点击 XX 按钮"而非"调用 XX 接口"） |
| 预期结果 | 业务层面期望的系统表现、页面跳转、数据变化 |
| 实际结果 | 执行时填写，初始为空 |
| 测试结论 | Pass / Fail / Blocked，执行时填写 |

## 机器治理字段（系统编排用）

| 字段 | 取值 | 作用 |
|---|---|---|
| data_contract（数据契约） | 结构化对象（见下） | 机器可读的前置条件，驱动 bmad-uat-prepare-data |
| story_key | 如 1-2-order | 关联 BMAD story |
| version_anchor（版本锚点） | baseline_commit / git SHA | 过期检测（bmad-uat-impact-scan 比对） |
| exec_mode（执行模式） | auto / manual / semi | auto=机器可客观判定；manual=需人主观验收；semi=机器跑流程+人确认 |
| destructive（破坏性） | read-only / write-isolated | read-only=共享基线数据；write-isolated=改状态，需专属实体或独立快照 |

## data_contract 数据契约 Schema

机器可读的前置条件，声明该用例需要什么数据、什么状态、能否自动生成。建议 YAML 结构：

```yaml
data_contract:
  entities:
    - type: account            # 业务实体类型
      ref: buyer_a              # 用例内引用名
      state: { balance: 0 }    # 需要的状态（如余额为0测"余额不足"）
      auto_generatable: true   # 能否通过 API/脚本自动构造
    - type: product
      ref: sku_a
      state: { stock: 1 }
      auto_generatable: true
    - type: coupon
      ref: coupon_expired
      state: { expired: true } # 已过期优惠券，测"优惠券失效"
      auto_generatable: true
    - type: id_photo           # 需外部真实物料的示例
      ref: kyc_photo
      auto_generatable: false  # 无法程序化生成 → 列表格请用户提供
      requirement: "一张清晰的真实证件照，jpg/png，<2MB"
  isolation: write-isolated    # 与用例 destructive 字段一致
  notes: "buyer_a 与 sku_a 为本用例专属实体，跑前重置、跑后清理"
```

### 自动生成边界（P4）

- **auto_generatable: true** — 能通过现有 API/脚本构造的（测试商品、测试账号、过期优惠券、库存归零等业务状态）→ bmad-uat-prepare-data 自动生成。
- **auto_generatable: false** — 需外部真实物料的（真实证件照、特定第三方真实账号、人工审核物料）→ 列入数据需求表格，请用户填路径。

### 数据隔离分层（P2）

- **read-only** 用例：共享 UAT 环境基线数据，无需重置。
- **write-isolated** 用例：使用 `data_contract.entities` 中声明的专属实体，跑前重置到声明状态、跑后清理，保证用例间零污染。
