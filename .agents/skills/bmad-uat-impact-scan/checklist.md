# UAT Impact Scan — Validation Checklist

- [ ] 变更范围已确定（默认 last-UAT..HEAD 或用户指定）
- [ ] 改动文件已通过 git diff 列出
- [ ] 改动文件已映射到受影响 story（经 File List/实现路径）
- [ ] 每条用例的 version_anchor 已与当前 baseline 比对
- [ ] 语义性变更才判定过期；非语义改动（重构/格式）归入重验而非过期
- [ ] 已产出 impact-scan-report，含清单①重验范围 + 清单②过期用例 + 汇总建议
- [ ] 已呈现带选项的下一步菜单并等待用户选择
- [ ] 过期用例仅在用户逐条确认后才触发 build-cases 重建，绝不自动重建
- [ ] 重验范围正确传给 prepare-data → run
