---
用例编号: UAT-KEY-004
测试模块: API Key 安全存储
story_key: 1-5-llm-api-key-keyring-storage
version_anchor: 5268797
exec_mode: semi
destructive: write-isolated
优先级: 中
data_contract:
  entities:
    - type: 测试用 API Key
      ref: test_api_key
      state: { value: "sk-uatlog9876543210zyxw" }
      auto_generatable: true
      requirement: ""
  isolation: write-isolated
  notes: 使用独立测试数据目录与日志；测试后清理 keyring 测试条目与日志。
---

# UAT-KEY-004 保存 Key 过程中日志不泄露 Key 真实值

## 业务场景
应用运行时会写入诊断日志。用户保存或加载 API Key 的过程被记录到日志，但日志中绝不能出现 Key 的真实值——只能记录"哪个配置的 Key 被操作了"，真实值需被脱敏。

## 前置条件
- 应用可启动并能写日志
- 准备测试 Key `sk-uatlog9876543210zyxw`

## 测试步骤
1. 启动应用并开启日志记录
2. 进入「全局设置」的「LLM」分页，新增配置并保存测试 Key
3. 触发一次「测试连接」（会触发 Key 的加载）
4. 打开应用日志文件，搜索 `sk-uatlog` 是否出现

## 预期结果
- 日志文件中找不到 Key 真实值（无 `sk-uatlog` 匹配）
- 日志中关于密钥操作的记录仅含配置标识/Key 名称，真实值被脱敏处理

## 实际结果

## 测试结论
