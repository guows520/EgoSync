---
用例编号: UAT-APP-004
测试模块: 桌面应用视觉一致性
story_key: 1-1-tauri-desktop-app-existing-ui
version_anchor: 5268797
exec_mode: manual
destructive: read-only
优先级: 中
data_contract:
  entities:
    - type: 运行中的开发服务
      ref: vite-dev-server
      state: { port: 5173, running: true }
      auto_generatable: true
      requirement:
    - type: 浏览器基线视图
      ref: browser-baseline
      state: { opened_url: "http://localhost:5173" }
      auto_generatable: true
      requirement:
  isolation: read-only
  notes: 同时打开浏览器与桌面窗口对比，均为只读浏览，不改动数据。
---

# UAT-APP-004 桌面窗口与浏览器版界面像素级一致

## 业务场景
产品要求把现有 Web 原型零改动地搬进桌面壳，用户无论在桌面窗口还是浏览器中看到的界面都应完全一致，保证视觉品质不因打包而退化。

## 前置条件
- 开发服务已在 5173 端口运行。
- 已同时启动桌面窗口版与浏览器版（同一 dev server）。

## 测试步骤
1. 在浏览器中打开 `http://localhost:5173`，截取首屏（管家视角）。
2. 在桌面窗口中打开同一首屏，截取相同区域。
3. 逐项对比字体、布局、配色、间距与动效。
4. 切换若干视图（如点击角色、打开设置）再对比一次。

## 预期结果
- 两端界面在字体、布局、颜色、动效上完全一致，无错位、无字体降级、无配色差异。
- 视图切换行为两端表现相同。

## 实际结果

## 测试结论
