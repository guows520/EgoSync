---
用例编号: UAT-KEY-001
测试模块: API Key 安全存储
story_key: 1-5-llm-api-key-keyring-storage
version_anchor: 5268797
exec_mode: semi
destructive: write-isolated
优先级: 高
data_contract:
  entities:
    - type: 全新数据目录
      ref: clean_app_data
      state: { egosync_db: "不存在", logs: "空" }
      auto_generatable: true
      requirement: ""
    - type: 测试用 API Key
      ref: test_api_key
      state: { value: "sk-uattest1234567890abcdef" }
      auto_generatable: true
      requirement: ""
  isolation: write-isolated
  notes: 使用独立的测试用户数据目录，测试结束后清理 keyring 中以 com.egosync.app 服务名写入的测试条目及数据目录。
---

# UAT-KEY-001 用户保存 API Key 后明文不落数据库与日志

## 业务场景
用户在设置中填入自己的 LLM API Key 并保存。出于隐私安全考虑，这把"钥匙"绝不能以明文出现在应用的数据库文件或任何日志文件中，否则一旦文件被他人获取就会泄露用户的付费账号。

## 前置条件
- 应用已安装并能正常启动
- 当前使用全新的测试数据目录（无历史配置）
- 准备好一个可识别的测试 Key 字符串 `sk-uattest1234567890abcdef`

## 测试步骤
1. 启动应用，进入「全局设置」的「LLM」分页
2. 新增一个 LLM 配置，在 API Key 输入框中填入测试 Key `sk-uattest1234567890abcdef`
3. 点击保存，等待提示保存成功
4. 关闭应用
5. 打开应用数据目录，查找主数据库文件与全部日志文件，搜索是否出现 `sk-` 开头的明文 Key
6. 打开操作系统的凭据管理器（Windows 凭据管理器 / macOS 钥匙串 / Linux Secret Service），查找服务名为 com.egosync.app 的条目

## 预期结果
- 数据库文件与日志文件中均搜索不到测试 Key 明文（无 `sk-uattest` 匹配）
- 操作系统凭据管理器中存在 com.egosync.app 对应的条目，Key 真实存储于系统安全区域
- 应用界面 Key 字段以掩码（如 ••••••••）显示，不回显明文

## 实际结果

## 测试结论
