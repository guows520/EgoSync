---
用例编号: UAT-APP-001
测试模块: 桌面应用启动
story_key: 1-1-tauri-desktop-app-existing-ui
version_anchor: 5268797
exec_mode: semi
destructive: read-only
优先级: 高
data_contract:
  entities:
    - type: 源码仓库
      ref: egosync-repo
      state: { branch: main, gui_dir_present: true, node_installed: true, rust_installed: true }
      auto_generatable: true
      requirement:
  isolation: read-only
  notes: 仅读取仓库源码并启动开发服务，不修改任何持久化数据；用例结束后关闭窗口与 dev server 即可。
---

# UAT-APP-001 开发者本地启动桌面应用并看到完整界面

## 业务场景
新加入的开发者克隆 EgoSync 仓库后，希望用一条命令把应用以独立桌面窗口跑起来，从而无需打开浏览器即可看到产品原型界面，确认开发环境已就绪。

## 前置条件
- 机器已安装 Node.js ≥ 18 与 Rust ≥ 1.77.2，以及对应平台构建工具（Windows: VS Build Tools + WebView2）。
- 已克隆仓库到本地，`egosync-app/` 目录完整。
- 端口 5173 未被其他程序占用。

## 测试步骤
1. 进入 `egosync-app/` 目录，安装依赖（首次运行）。
2. 执行启动桌面应用的命令（开发模式）。
3. 等待编译完成，观察是否弹出独立的应用窗口。
4. 查看窗口标题、默认窗口尺寸与界面内容。

## 预期结果
- 弹出一个标题为 "EgoSync" 的独立桌面窗口（默认 1200×800，居中显示）。
- 窗口内显示产品原型界面（左侧角色导航栏、主内容区），无白屏、无报错弹窗。
- 界面内容与在浏览器中打开的开发版本视觉一致。

## 实际结果

## 测试结论
