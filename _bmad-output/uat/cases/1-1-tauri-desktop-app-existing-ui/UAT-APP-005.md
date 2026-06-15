---
用例编号: UAT-APP-005
测试模块: 桌面应用启动异常处理
story_key: 1-1-tauri-desktop-app-existing-ui
version_anchor: 5268797
exec_mode: semi
destructive: write-isolated
优先级: 中
data_contract:
  entities:
    - type: 占用端口的进程
      ref: port-5173-occupier
      state: { port: 5173, occupied: true }
      auto_generatable: true
      requirement:
    - type: 源码仓库
      ref: egosync-repo
      state: { branch: main }
      auto_generatable: true
      requirement:
  isolation: write-isolated
  notes: 通过脚本临时占用 5173 端口模拟冲突；用例结束后释放端口。不改动仓库数据。
---

# UAT-APP-005 端口被占用时启动给出明确报错而非静默白屏

## 业务场景
开发者机器上若已有其他程序占用了开发端口（5173），启动桌面应用时系统应明确告知端口冲突，而不是弹出一个空白窗口让人困惑。这是常见的开发期异常场景。

## 前置条件
- 已用任意程序占用本机 5173 端口。
- 仓库可正常构建。

## 测试步骤
1. 先启动一个占用 5173 端口的进程。
2. 在 `egosync-app/` 目录执行启动桌面应用的开发命令。
3. 观察终端输出与窗口表现。

## 预期结果
- 终端给出明确的端口被占用/strictPort 报错信息，启动中止或提示冲突。
- 不出现"窗口已弹出但内容空白且无任何提示"的情况。

## 实际结果

## 测试结论
