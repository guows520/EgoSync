# Run UAT — Validation Checklist

## 前置
- [ ] mode 已显式指定（dev / regression），未做模糊推断
- [ ] scope 已确认
- [ ] 目标确认为专用 UAT 环境（与生产隔离，外部依赖走沙箱）
- [ ] 所需数据已就绪（快照存在），否则已引导 prepare-data

## 执行
- [ ] 复用项目已有测试框架驱动 auto 类用例，未擅自引入新框架
- [ ] read-only 用例共享基线；write-isolated 用例跑前重置、跑后清理
- [ ] dev 模式：先自动跑 auto，再人工逐条验收 manual/semi
- [ ] regression 模式：自动为主，仅没把握的升级给人
- [ ] 已填写每条用例的实际结果与测试结论

## 报告与状态
- [ ] 已捕获每个 Fail 的五要素证据 DNA
- [ ] 未做代码 path:line 归因（留给 bmad-investigate）
- [ ] 已自动产出业务报告 + 技术诊断报告，按用例编号关联
- [ ] 已更新 uat-status.yaml，未写回 sprint-status.yaml
- [ ] 已对失败项给出交接 bmad-investigate 的提示
