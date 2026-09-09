# 测试与验收矩阵

> 命令基线（AGENTS DoD）：Android `cd companion-android && ./gradlew :app:assembleDebug && ./gradlew :app:testDebugUnitTest`；桌面改动 `cd egosync-app && npm run test:all`（tsc + vitest + cargo test）。无法执行的验证（真机项）须标注「人工/待设备」并说明原因，禁止声称已验证。
> 分层：U = JVM 单测（时间可控，`runTest` + 假实现缝）；C = 跨端契约（fixture）；D = 桌面（Rust/Vitest）；M = 真机/模拟器人工；R = 回归（既有测试不得删除/弱化断言）。

## 一、11 条产品验收（用户裁决原文编号）

| # | 验收场景 | 层 | 落点与断言要点 | 任务 |
| --- | --- | --- | --- | --- |
| 1 | 健康已配对设备冷启动，20 秒内不出现阻断遮罩 | U + M | U：构造 paired 健康 Store，编排启动后立即（不 advanceTimeBy）断言 TransportStatus=Connecting、无全屏遮罩门派生；20s 虚拟时间后仍无会话 → Degraded。M：强杀重开真机 20s 内缓存可交互、对话输入可录入（待发态）、无遮罩 | T-S3 |
| 2 | 网络不可达：Connecting/Reconnecting → Degraded，不出现重配 CTA | U + M | U：NSD/中继全失败假实现 + 无结构化拒绝 → Degraded 且 PairingHealth=Ok、重配入口派生值为 false；退避循环仍活跃。M：桌面关机复现 | T-S3/T-S5 |
| 3 | 配对或凭据失效：明确显示重新配对原因并进入配对流 | U (+M) | U：SecretsInvalidatedException / pairingRejected(已配对) / 信任锚失败 → 恢复事件 reason 正确 + 原因文案映射断言 + 导航调用（PAIRING + popUpTo(0)）| T-S4/T-S5 |
| 4 | paired 元数据完整但私钥文件缺失：不得静默生成新身份并无限重试 | U | U：SecretsProvider 假实现——已配对上下文 loadExisting 抛 Missing（文件缺失）→ 断言不调用 create、paired=false、发布 CredentialMissing、退避循环终止（不进入重试）；store/密钥清理调用各一次 | T-S4 |
| 5 | 运行中 `SecretsInvalidatedException`：连接、流、快照、通知和导航完整收口 | U | U：会话中抛出 → 原子序列调用断言：commandChannel.shutdown、streamCoordinator.reset、store.clear、secrets.wipe、snapshotStore.clear、notifications.clear、navigate 各一次且有序；待发箱队列原样不动；无"Direct 假在线 + 旧路由"残留态 | T-S4 |
| 6 | 成功配对后手机本地解绑，再扫旧 QR：显示「二维码已使用/需重新生成」，而不是网络双失败 | D + U (+M) | D：早期准入拒绝（窗口已消费）前发送 `pairingRejected{pairingWindowClosed}` Notice（直连与中继两路径）。U：手机配对 probe 收到该 Notice → PairingProgress.Failed(reason) 直接呈现结构化文案，断言不落入 waitDesktopConfirmAndRetry、不出现「未发现桌面设备，且中继连接失败」 | T-S5/T-S6/T-S7 |
| 7 | 桌面有效 QR 可立即重新生成，旧码失效，新码可用 | D + M | D：`generate_qr` 开新窗口覆盖旧窗口（既有行为回归测试）；组件：显示态存在重新生成按钮 + 确认对话框文案含「立即失效」。M：有效期内重新生成 → 旧码扫描被拒（结构化）、新码扫码配对成功；配对成功事件后码显示「已使用」并清空 | T-S6 |
| 8 | `engineAvailable=true` 但 binding 为空的竞速窗口：发送不产生突兀失败（**裁决修订**：队列化取代"控件不可用"字面，意图——无突兀报错——不变） | U | U：模拟"会话结束 unbind + 滞回窗内展示仍 Direct" → 断言 commandReady=false、此时发送入待发箱（不触发"桌面引擎不可达"路径）、bind 完成后待发条目自动发出；bind 完成先于 Direct 发布的时序断言（onDirectEstablished 调用晚于 bind 回调） | T-S2/T-S10 |
| 9 | `phase="answering"` 多个正文 token：逐 token 展示，无空光标 | U + C | C：fixture（answering 多 token、thinking、tool、process、done、多 messageId、多 conversationId、phase=null 兼容）全矩阵断言（streaming-protocol.md §4）；U：中间态逐字断言 + 空光标守卫（无 text 为空 streaming 项） | T-S1 |
| 10 | IME 聚焦：composer 与键盘之间没有底部导航栏高度残留 | M（诊断 A → 修复 B） | A：诊断数据表（root/内外层 content/NavigationBar/composer bounds + 各 inset 值，键盘前后）；B：真机验证表——原输入法 × 手势/三键导航，composer 底部与键盘顶部间距 8–12dp、无 bottomBar 高度残留空区、收起回位；诊断日志 grep 零残留 | T-S8 |
| 11 | 离线待发消息在断网、重配、凭据恢复和进程被杀中不得丢失或被虚假标记已发送 | U (+M) | U：三场景断言待发箱队列原样（断网 Degraded、unpair、恢复事件原子序列）；落盘恢复：进程重建（新容器实例/队列重读）后队列与待发态还原（含本地占位会话）；flush 守门：commandReady=false 时绝不 flush（含"展示 Direct 但 unbind"窗口）；flush 串行（一次一条、等本轮流式 done）；重发复用同一 commandId（条目 id 稳定断言）。M：断网录入 → 强杀重开 → 恢复网络，消息自动送达 | T-S2/T-S4/T-S10 |

## 二、机制级补充断言（超出 11 条但锁定修复意图）

| 断言 | 层 | 说明 |
| --- | --- | --- |
| 正文判定函数唯一性 | U | `foldNew`/`foldInto` 对同一事件序列产生一致文本（共用 `isAnswerText`——防止两处判定再次漂移） |
| tool/process/thinking/done 不误入正文 | U | §4 矩阵的负向断言 |
| PairingProgress.Failed 携带结构化 reason | U | 连接层不拼用户文案（qr-and-unbind-semantics.md §4） |
| 未知 phase 值保守忽略 | U | 未来桌面扩展 fail-safe |
| `unpair()` 后 pairWithQr join wipeJob | R | 既有 P3 语义回归（防止新调用点破坏） |
| 旧桌面不发 Notice 时手机回退现状 | U | 兼容用例：无拒绝 Notice 的关闭按既有网络/超时文案 |
| Debug 六档切换驱动新状态 | U/R | 设置页隐藏入口 + Fake 换装 |
| 桌面 phase 值域锁定（**必做**，A+B 裁决） | D | StreamPayload phase 语义 Rust 测试，锚定 Android fixture |
| 待发箱四路径结果处理 | U | 成功删条目 / 连接失败留队 / 业务错误删条目 / thinking·responding 并发守卫忽略新发送 |
| 离线发送 UI 待发态 | U | !commandReady 时消息项呈待发态、提示文案为"网络不可用，恢复后自动发送"；恢复后状态翻转 |

## 三、回归清单

- `StreamCoordinatorTest` / `ChatViewModelTest`：辅助函数默认值改生产形状后全绿（改造不是删除断言）。
- `RealConnectionClientOrchestrationTest`：既有自愈/退避/滞回用例全绿；新增 §一 #4/#5/#8 用例。
- `CommandChannel` 相关既有用例（TOCTOU/看门狗语义）不受 StateFlow 改造影响。
- 桌面 `cargo test`（companion 模块）+ 既有 vitest 全绿；`npm run build` tsc 零错误。
- 手动回归：冷启动直落配对流（未配对）、配对成功进主界面、设置页 Debug 档、离线输入→恢复自动发送链路。
- `QuickNoteQueue`/`DegradedOverlay` 速记行删除后全仓零引用（编译 + grep 断言）。
