---
用例编号: UAT-LLM-006
测试模块: LLM Provider 连接测试
story_key: 1-6-llm-provider-connection-test
version_anchor: 5268797
exec_mode: semi
destructive: write-isolated
优先级: 高
data_contract:
  entities:
    - type: 有效 LLM 配置
      ref: valid_config
      state: { is_default: true }
      auto_generatable: false
      requirement: 需一组真实有效配置（含有效 API Key），以验证重启后仍可用。
  isolation: write-isolated
  notes: 涉及重启应用；测试后删除配置与 keyring 条目。
---

# UAT-LLM-006 重启应用后已保存配置仍然存在且可用

## 业务场景
用户辛苦配置好了 LLM 并测试通过。关掉应用第二天再打开，配置应原样保留，用户不必每次重配。同时 Key 始终只存在系统安全区，数据库里只留引用标识。

## 前置条件
- 已保存至少一个有效 LLM 配置并设为默认

## 测试步骤
1. 记录当前已保存的配置（名称、服务地址、模型）
2. 完全关闭应用
3. 重新启动应用，进入「LLM」分页
4. 核对配置是否与关闭前一致
5. 对该配置再次执行「测试连接」

## 预期结果
- 重启后配置列表与关闭前完全一致（名称/地址/模型/默认标记保留）
- 再次测试连接仍成功（Key 从系统安全区正确取回）
- 界面中 Key 仍以掩码显示

## 实际结果

## 测试结论
