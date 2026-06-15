/**
 * UAT 数据预置脚本（幂等）
 *
 * 隔离策略：真实库 + 专用标记
 *   - 所有测试实体 id 以 `uat-` 前缀；角色名以 `[UAT]` 前缀
 *   - 复用现有默认 LLM Provider（无问苍穹 glm-5）与现有 MCP/Skill，不新建/不改动它们
 *   - 仅插入额外的「测试专用实体」和「异常场景物料」
 *   - 跑前先删所有 uat- 前缀实体（幂等），再插入
 *
 * 用法：
 *   node seed.mjs <egosync.db路径>            # 预置
 *   node seed.mjs <egosync.db路径> --cleanup  # 仅清理 uat- 实体
 *
 * 真实数据保护：只 DELETE id LIKE 'uat-%' 的行，绝不碰真实角色/记忆/skill/mcp/llm_config。
 */
import { DatabaseSync } from 'node:sqlite';

const dbPath = process.argv[2];
const cleanupOnly = process.argv.includes('--cleanup');
if (!dbPath) {
  console.error('用法: node seed.mjs <egosync.db路径> [--cleanup]');
  process.exit(1);
}

const db = new DatabaseSync(dbPath);
db.exec('PRAGMA foreign_keys = ON;');

const now = new Date().toISOString().replace(/\.\d+Z$/, 'Z');

// ---------- 清理（幂等核心：只删 uat- 前缀实体）----------
function cleanup() {
  // 顺序：先删依赖（绑定/记忆），再删主体
  db.exec(`DELETE FROM skill_role_bindings WHERE skill_id LIKE 'uat-%' OR role_id LIKE 'uat-%';`);
  db.exec(`DELETE FROM role_mcp_server_bindings WHERE server_id LIKE 'uat-%' OR role_id LIKE 'uat-%';`);
  db.exec(`DELETE FROM memories WHERE id LIKE 'uat-%' OR source_conversation_id LIKE 'uat-%';`);
  db.exec(`DELETE FROM forgotten_memory_sources WHERE id LIKE 'uat-%' OR source_conversation_id LIKE 'uat-%';`);
  db.exec(`DELETE FROM skills WHERE id LIKE 'uat-%';`);
  db.exec(`DELETE FROM mcp_servers WHERE id LIKE 'uat-%';`);
  db.exec(`DELETE FROM llm_configs WHERE id LIKE 'uat-%';`);
  db.exec(`DELETE FROM roles WHERE id LIKE 'uat-%';`);
  console.log('✅ 已清理所有 uat- 前缀测试实体');
}

cleanup();
if (cleanupOnly) { db.close(); process.exit(0); }

// ---------- 角色实体 ----------
// 覆盖：CRUD/视图切换/语气/委派/侧栏/Skill/MCP/记忆等用例
const roles = [
  // ref, name, icon, color, goal, personality, status, energy, proactivity, skills_config
  ['uat-role-pm',     '[UAT]产品教练', '🧭', '#3B82F6', '把模糊想法拆成可执行计划', '目标导向，先给优先级和计划', 'active', 100, 'moderate', '{"find-skills":true,"skill-creator":true}'],
  ['uat-role-health', '[UAT]健康教练', '🏃', '#22C55E', '帮助制定健康习惯和训练计划', '关注身体感受，循序渐进', 'active', 90, 'proactive', '{"find-skills":true,"skill-creator":false}'],
  ['uat-role-family',  '[UAT]家庭助理', '🏠', '#F97316', '帮助安排家庭事务', '体察家庭成员感受，重协调', 'active', 80, 'moderate', '{}'],
  ['uat-role-writer',  '[UAT]写作顾问', '✍️', '#A855F7', '帮助写作和总结', '', 'active', 70, 'passive', '{}'],            // 空个性，测 TONE-005
  ['uat-role-learner', '[UAT]学习者',   '📚', '#EAB308', '陪伴学习与复盘', '鼓励式，多提问', 'archived', 60, 'moderate', '{}'], // 归档态，测 ROLE-003 恢复
  ['uat-role-solo',    '[UAT]唯一角色', '⭐', '#EF4444', '用于唯一角色保护场景', '', 'active', 100, 'moderate', '{}'],          // 测 ROLE-005（配合临时归档其他）
];
const insRole = db.prepare(`INSERT INTO roles
  (id,name,icon,color,goal,personality_prompt,status,energy,skills_config,proactivity_level,archived_at,created_at,updated_at)
  VALUES (?,?,?,?,?,?,?,?,?,?,?,?,?)`);
for (const [id,name,icon,color,goal,pers,status,energy,proact,skills] of roles) {
  insRole.run(id,name,icon,color,goal,pers,status,energy,skills,proact, status==='archived'?now:null, now, now);
}
console.log(`✅ 角色: ${roles.length} 个`);

// ---------- 记忆实体 ----------
// 覆盖：记忆面板/分类筛选/来源追溯/遗忘/推理依据/空态
// category ∈ preference,task_status,cognition_update,fact
const memories = [
  // id, role_id(null=全局), category, content, source_conv, source_msg_ids
  ['uat-mem-001', null,            'preference',       '用户偏好早上处理深度工作，下午开会', 'uat-conv-001', '["uat-msg-001"]'],
  ['uat-mem-002', null,            'fact',             '用户季度目标是提升用户留存',         'uat-conv-001', '["uat-msg-002"]'],
  ['uat-mem-003', 'uat-role-pm',    'task_status',      '产品复盘已排到本周五',               'uat-conv-002', '["uat-msg-003"]'],
  ['uat-mem-004', 'uat-role-pm',    'cognition_update', '用户认为留存比拉新更重要',           'uat-conv-002', '["uat-msg-004"]'],
  ['uat-mem-005', 'uat-role-health','preference',       '用户每周最多跑步三次',               'uat-conv-003', '["uat-msg-005"]'],
  // 来源缺失记忆：source_conversation_id 指向不存在的对话（测 MEMPANEL-006 来源不可用）
  ['uat-mem-006', 'uat-role-pm',    'fact',             '来源已删除的测试记忆',               'uat-conv-deleted', '["uat-msg-x"]'],
];
const insMem = db.prepare(`INSERT INTO memories
  (id,role_id,category,content,source_conversation_id,source_message_ids,created_at)
  VALUES (?,?,?,?,?,?,?)`);
for (const [id,rid,cat,content,conv,msgs] of memories) {
  insMem.run(id,rid,cat,content,conv,msgs,now);
}
console.log(`✅ 记忆: ${memories.length} 条`);

// 已遗忘记忆来源（测 FORGET-005 / REASON-005/006：已遗忘记忆不回流、引用降级）
db.prepare(`INSERT INTO forgotten_memory_sources
  (id,role_id,category,content,normalized_content,source_conversation_id,source_message_ids,forgotten_at)
  VALUES (?,?,?,?,?,?,?,?)`).run(
  'uat-forgotten-001','uat-role-pm','preference','已遗忘：用户不想被安排晨会','已遗忘：用户不想被安排晨会',
  'uat-conv-forgotten','["uat-msg-f1"]', now);
console.log('✅ 已遗忘记忆来源: 1 条');

// ---------- Skill 实体（测试专用，不碰真实 4 个）----------
// 覆盖：导入/重复导入/绑定/启用禁用
const skills = [
  ['uat-skill-mae', 'meeting-action-extractor', '将会议纪要整理为行动项', 'custom',   'uat-skills/meeting-action-extractor/SKILL.md', 'uat-hash-mae'],
  ['uat-skill-oc',  'uat-opencode-demo',         '生态发现测试 Skill',     'opencode', 'uat-opencode/skills/demo/SKILL.md',            'uat-hash-oc'],
];
const insSkill = db.prepare(`INSERT INTO skills
  (id,name,description,source_type,managed_path,content_hash,created_at,updated_at)
  VALUES (?,?,?,?,?,?,?,?)`);
for (const [id,name,desc,src,path,hash] of skills) {
  insSkill.run(id,name,desc,src,path,hash,now,now);
}
// 绑定 meeting-action-extractor 到产品教练（测 IMPORT-004/005 启用禁用、切换不丢绑定）
db.prepare(`INSERT INTO skill_role_bindings (skill_id,role_id,created_at) VALUES (?,?,?)`)
  .run('uat-skill-mae','uat-role-pm',now);
console.log(`✅ Skill: ${skills.length} 个 + 1 绑定`);

// ---------- MCP 故障 server（测 MCP-003 连接失败降级，不碰真实 2 个）----------
// server_type ∈ sse,streamable_http,stdio；env_refs JSON
const insMcp = db.prepare(`INSERT INTO mcp_servers
  (id,name,server_type,command_or_url,env_refs,description,enabled,created_at,updated_at)
  VALUES (?,?,?,?,?,?,?,?,?)`);
insMcp.run('uat-mcp-broken','[UAT]故障MCP','streamable_http','https://127.0.0.1:9/mcp','{}','故意不可达，测连接失败降级',1,now,now);
console.log('✅ 故障 MCP server: 1 个');

// ---------- 无效 LLM Provider（测 LLM-002/CHAT-003 无效Key/失败提示）----------
// 注意：api_key_ref 仅引用，实际无效 key 由 keyring 或运行时注入 invalid 值
db.prepare(`INSERT INTO llm_configs
  (id,name,provider,base_url,model,api_key_ref,is_default,created_at,updated_at)
  VALUES (?,?,?,?,?,?,?,?,?)`).run(
  'uat-llm-invalid','[UAT]无效Provider','openai_compatible','https://api.invalid-uat.example/v1','glm-5',
  'uat_invalid_api_key', 0, now, now);
console.log('✅ 无效 LLM Provider: 1 个（非默认，不影响真实默认 Provider）');

db.close();
console.log('\n========== UAT 数据预置完成 ==========');
console.log('所有测试实体均为 uat- 前缀 / [UAT] 标记，可用 --cleanup 一键清理。');
