---
用例编号: UAT-KEY-002
测试模块: API Key 安全存储
story_key: 1-5-llm-api-key-keyring-storage
version_anchor: 5268797
exec_mode: manual
destructive: write-isolated
优先级: 中
data_contract:
  entities:
    - type: 已保存 API Key 的应用实例
      ref: app_with_key
      state: { api_key_saved: true }
      auto_generatable: true
      requirement: ""
  isolation: write-isolated
  notes: 涉及卸载重装操作，需在隔离测试机或测试用户环境执行；测试后清理 keyring 条目。
---

# UAT-KEY-002 卸载重装应用后 API Key 仍然有效

## 业务场景
用户的 API Key 存储在操作系统的安全区域，而非应用自身文件中。因此即使用户卸载并重新安装应用，之前保存过的 Key 仍应保留，用户不必重新输入。

## 前置条件
- 已通过设置保存过一个有效的 API Key（系统钥匙串中已有 com.egosync.app 条目）

## 测试步骤
1. 确认应用中已保存 API Key 且配置可用
2. 通过系统正常方式卸载应用（不手动清理系统凭据管理器）
3. 重新安装并启动应用
4. 进入「全局设置」的「LLM」分页查看原有配置
5. 对原配置执行一次「测试连接」

## 预期结果
- 重装后系统钥匙串中 com.egosync.app 的 Key 条目仍然存在
- 应用能复用该 Key 完成连接测试（无需重新输入 Key）

## 实际结果

## 测试结论
