---
用例编号: UAT-CLASSIFY-005
测试模块: 任务自动分类
story_key: 3-3-auto-quadrant-classification
version_anchor: 0871096146e95899b732ed7902c8c7706896c066
exec_mode: semi
destructive: write-isolated
优先级: 中
data_contract:
  entities:
    - type: 应用实例
      ref: 已完成首次设置但默认 LLM 未配置或不可用的 EgoSync
      state: { 已安装: true, 默认LLM可用: false }
      auto_generatable: true
      requirement: false
    - type: 角色任务
      ref: 新建任务
      state: { 标题: "任意任务" }
      auto_generatable: true
      requirement: false
  isolation: write-isolated
  notes: 验证 LLM 失败时的降级行为。
---

# UAT-CLASSIFY-005 LLM 不可用时任务降级为 Q2 且不报错

## 业务场景
用户的 LLM 配置暂时不可用（API Key 过期、网络异常、未配置），希望创建任务时不被卡住，系统默认把任务放到 Q2（重要不紧急），不让用户看到一堆技术错误。

## 前置条件
- 默认大模型未配置或连接失败

## 测试步骤
1. 创建一条新任务，不手动选择象限
2. 保存并观察任务状态
3. 检查应用是否报错或卡死

## 预期结果
- 任务正常创建，默认分配 Q2
- 写入合理置信度与失败原因
- 应用不报错、不卡死，用户看不到 API key 或底层敏感信息
- 失败原因仅在日志中记录（tracing::warn）

## 实际结果
<留空>

## 测试结论
<留空>
