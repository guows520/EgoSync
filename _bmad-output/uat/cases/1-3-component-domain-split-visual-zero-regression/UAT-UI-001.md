---
用例编号: UAT-UI-001
测试模块: 界面视觉零回归
story_key: 1-3-component-domain-split-visual-zero-regression
version_anchor: 5268797
exec_mode: manual
destructive: read-only
优先级: 高
data_contract:
  entities:
    - type: 拆分前基线截图集
      ref: baseline-screenshots
      state: { dir: "_bmad-output/screenshots/1-3-baseline/", views: ["管家视角","角色视图","全局设置","仲裁弹窗","周复盘弹窗","通知面板"] }
      auto_generatable: false
      requirement: 需要组件拆分前采集的 6 张基线截图（管家视角/角色视图/全局设置/仲裁/周复盘/通知面板）作为像素对比基准。
    - type: 运行中的应用
      ref: app-after-split
      state: { running: true }
      auto_generatable: true
      requirement:
  isolation: read-only
  notes: 仅做视觉浏览对比，不改动数据。允许 ≤2px 容差。
---

# UAT-UI-001 组件拆分后界面与拆分前像素级一致

## 业务场景
代码做了大规模组件域拆分，但用户体验不应有任何变化。用户期望拆分后看到的界面与之前完全相同，重构对其"无感"。

## 前置条件
- 已有拆分前采集的 6 个关键页面基线截图。
- 拆分后的应用可正常启动。

## 测试步骤
1. 启动应用，依次进入：管家视角、角色视图、全局设置面板、仲裁弹窗、周复盘弹窗、通知面板。
2. 对每个页面截取与基线相同区域的截图。
3. 将拆分后截图与基线逐一对比（字体、布局、颜色、间距）。

## 预期结果
- 6 个页面均与基线像素级一致（允许 ≤ 2px 容差）。
- 唯一允许的差异：顶部黑色 Pitch Mode bar 消失、主内容上移。

## 实际结果
[Blocked] 缺少前置物料：未提供「组件拆分前的 6 张基线截图（管家/角色/全局设置/仲裁/周复盘/通知面板）」（数据需求表 B-3）。无拆分前基线无法做像素级零回归对比。

## 测试结论
Blocked（阻塞原因：基线物料缺失 B-3 拆分前 6 张截图；解除后转人工对比执行）
