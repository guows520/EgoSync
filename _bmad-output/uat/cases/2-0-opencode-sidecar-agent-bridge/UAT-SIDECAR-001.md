---
用例编号: UAT-SIDECAR-001
测试模块: 对话引擎可用性
story_key: 2-0-opencode-sidecar-agent-bridge
version_anchor: 5268797
exec_mode: semi
destructive: write-isolated
优先级: 高
data_contract:
  entities:
    - type: 应用实例
      ref: 全新启动的 EgoSync 桌面应用
      state: { 已安装: true, 首次启动: false }
      auto_generatable: true
      requirement: false
    - type: 对话
      ref: 一个用于发送测试消息的管家对话
      state: { 内容: 空 }
      auto_generatable: true
      requirement: false
  isolation: write-isolated
  notes: 启动会在后台拉起独立的对话引擎进程并占用本机端口；验证后关闭应用即可回收，不影响用户数据。
---

# UAT-SIDECAR-001 打开应用后无需任何设置即可立即开始对话

## 业务场景
用户打开 EgoSync 后，希望像打开微信一样——窗口出来就能直接和管家说话，不需要先去配置什么"引擎""服务"。背后负责思考的对话能力应当在应用启动时就自动准备好。

## 前置条件
- EgoSync 已正常安装在本机
- 应用此前处于完全关闭状态（任务管理器中没有遗留的 EgoSync 相关进程）

## 测试步骤
1. 双击打开 EgoSync 应用，等待主界面出现
2. 不做任何额外设置，直接在管家对话框输入一句话（如"你好，介绍一下你能帮我做什么"）并发送
3. 观察管家是否能正常给出流式回复

## 预期结果
- 主界面打开后短时间内即进入可对话状态，无需用户手动开启任何后台服务
- 发送的消息能得到管家正常回复，回复以打字机式逐字流出
- 全程没有出现"引擎未就绪""无法连接"之类的阻断性报错

## 实际结果
<留空>

## 测试结论
<留空>
