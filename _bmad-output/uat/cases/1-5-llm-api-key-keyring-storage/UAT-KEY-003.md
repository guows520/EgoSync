---
用例编号: UAT-KEY-003
测试模块: API Key 安全存储
story_key: 1-5-llm-api-key-keyring-storage
version_anchor: 5268797
exec_mode: semi
destructive: read-only
优先级: 高
data_contract:
  entities:
    - type: 钥匙串不可用环境
      ref: no_keyring_env
      state: { keyring_available: false }
      auto_generatable: false
      requirement: 需提供一个系统密钥服务不可用的环境（如无桌面会话/无 D-Bus 的 Linux 环境，或人为停用凭据服务），用于验证降级行为。
  isolation: read-only
  notes: 仅验证报错行为，不写入持久数据；环境为只读验证。
---

# UAT-KEY-003 系统钥匙串不可用时明确报错且不降级明文

## 业务场景
某些环境（如无桌面会话的系统）系统密钥服务不可用。此时应用必须给出明确的错误提示，而绝不能为了"能用"而偷偷把 Key 以明文形式存到普通文件，否则会造成安全隐患。

## 前置条件
- 处于系统密钥服务不可用的环境
- 应用可启动

## 测试步骤
1. 在钥匙串不可用的环境中启动应用
2. 进入「全局设置」的「LLM」分页，尝试新增配置并保存 API Key
3. 观察界面返回的错误信息
4. 检查应用数据目录中是否出现包含 Key 明文的降级存储文件

## 预期结果
- 应用未崩溃，能正常运行
- 保存 Key 操作返回明确的"钥匙串错误"提示（说明密钥服务不可用），而非含糊的失败
- 应用数据目录中不存在任何明文 Key 文件——确认没有降级到明文存储

## 实际结果

## 测试结论
