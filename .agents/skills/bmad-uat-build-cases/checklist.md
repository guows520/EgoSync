# Build UAT Cases — Validation Checklist

## 用例质量
- [ ] 每条用例使用业务语言，无"调用接口/校验数据库表"等技术词汇
- [ ] 覆盖端到端闭环（完整故事流），而非孤立单点操作
- [ ] 包含业务异常场景（余额不足/优惠券过期/库存售罄等），并验证友好提示
- [ ] 每条用例可操作：具体前置条件、具体步骤、具体预期结果、含实际结果列
- [ ] 用例编号唯一，格式 UAT-<MODULE>-<NNN>

## 治理字段完整
- [ ] 每条用例有 data_contract（机器可读前置条件）
- [ ] 每条用例有 story_key + version_anchor（版本锚点）
- [ ] 每条用例标注 exec_mode（auto/manual/semi）
- [ ] 每条用例标注 destructive（read-only/write-isolated）
- [ ] write-isolated 用例已在 data_contract 声明专属实体

## 输出与交接
- [ ] 用例写入 {output_folder}/uat/cases/ 按 story_key 组织
- [ ] 增量重建只替换目标用例，保留其余
- [ ] 已输出数据需求汇总（自动生成项 vs 需用户提供项）
- [ ] 已提示用户运行 bmad-uat-prepare-data
