---
用例编号: UAT-LLM-005
测试模块: LLM Provider 连接测试
story_key: 1-6-llm-provider-connection-test
version_anchor: 5268797
exec_mode: auto
destructive: write-isolated
优先级: 中
data_contract:
  entities:
    - type: 多个 LLM 配置
      ref: multi_configs
      state: { count: 3 }
      auto_generatable: true
      requirement: 可用任意占位地址创建多个配置，本用例只验证默认唯一性，无需真实连通。
  isolation: write-isolated
  notes: 创建多条配置；测试后全部删除。
---

# UAT-LLM-005 多个配置间切换默认项时默认唯一

## 业务场景
用户保存了多个 LLM 配置（如一个云端、一个本地）。用户可把其中一个标记为"默认"，应用后续对话会用默认配置。任意时刻只能有一个默认，标记新默认时旧默认自动取消。

## 前置条件
- 已保存至少 3 个 LLM 配置

## 测试步骤
1. 进入「LLM」分页查看配置列表
2. 将配置 A 标记为默认，观察列表标记
3. 再将配置 B 标记为默认，观察列表标记
4. 重复将配置 C 标记为默认

## 预期结果
- 每次标记后，列表中有且仅有一个配置显示为默认
- 标记新默认后旧默认标记自动消失
- 不存在两个配置同时为默认的情况

## 实际结果

## 测试结论
