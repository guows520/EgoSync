---
用例编号: UAT-APP-002
测试模块: 桌面应用打包分发
story_key: 1-1-tauri-desktop-app-existing-ui
version_anchor: 5268797
exec_mode: manual
destructive: write-isolated
优先级: 高
data_contract:
  entities:
    - type: 源码仓库
      ref: egosync-repo
      state: { branch: main, build_toolchain_ready: true }
      auto_generatable: true
      requirement:
    - type: 干净测试机
      ref: clean-install-host
      state: { os: Windows10_or_11, webview2: installed }
      auto_generatable: false
      requirement: 需要一台未安装过 EgoSync 的干净 Windows 机器（或全新虚拟机快照）用于验证安装包可独立双击启动。
  isolation: write-isolated
  notes: 构建产物写入 target/release/bundle/；安装动作在干净测试机/独立快照上进行，验证后卸载并回滚快照，避免污染基线环境。
---

# UAT-APP-002 构建安装包并在干净机器上双击启动

## 业务场景
团队准备向真实用户分发桌面应用，用户期望下载一个安装包后双击即可安装并打开应用，无需任何命令行操作。

## 前置条件
- 构建机已具备目标平台的完整构建工具链。
- 准备一台干净的 Windows 机器（或全新虚拟机快照），未安装过 EgoSync。

## 测试步骤
1. 在构建机执行生产构建命令产出安装包。
2. 确认在产物目录生成了 Windows 安装包文件（.msi / setup.exe）。
3. 将安装包拷贝到干净测试机。
4. 双击安装包完成安装。
5. 从开始菜单或桌面图标启动 EgoSync。

## 预期结果
- 构建成功并在产物目录生成形如 `EgoSync_0.1.0_x64_en-US.msi` 与 `EgoSync_0.1.0_x64-setup.exe` 的安装包。
- 在干净机器上能正常完成安装。
- 双击应用图标后弹出 EgoSync 窗口并显示完整界面，无缺组件、无依赖缺失报错。

## 实际结果
[Blocked] 缺少前置物料：未提供「未安装过 EgoSync 的干净 Windows 机器/全新 VM 快照」（数据需求表 B-1），无法验证安装包在干净环境双击安装启动。构建产物侧未在本轮触发 `tauri build` 打包（无干净机承接安装验证，单跑打包无意义）。

## 测试结论
Blocked（阻塞原因：环境物料缺失 B-1 干净测试机；解除后转人工执行）
