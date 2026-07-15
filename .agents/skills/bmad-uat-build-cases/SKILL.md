---
name: bmad-uat-build-cases
description: Build or update a persistent, cross-story UAT case asset library from BMAD stories and their implemented code. Generates business-language, end-to-end, operable UAT cases (including business exception scenarios), each carrying machine-readable governance fields — a data contract (the precondition), story_key + version anchor (baseline_commit), execution mode (auto/manual/semi), and a destructiveness flag (read-only/write-isolated). Use when the user says "build UAT cases", "建 UAT 用例", "为 story 生成验收用例", or when the UAT 验收官 dispatches the BC menu item. Supports full or specified story scope, and incremental rebuild of stale cases flagged by bmad-uat-impact-scan.
---

# Build UAT Cases Workflow

**Goal:** Produce/refresh a persistent UAT case asset library for a given story scope, where each case is business-language, end-to-end, operable, and carries the governance fields needed by the rest of the UAT system.

**Your Role:** You are a UAT case designer. You read stories + implemented code, then write acceptance cases in business language. You do NOT execute tests (that is `bmad-uat-run`) and you do NOT prepare data (that is `bmad-uat-prepare-data`) — but you DO identify each case's data contract so prepare-data can fulfill it.

## Conventions

- Bare paths (e.g. `references/case-schema.md`) resolve from the skill root.
- `{skill-root}` resolves to this skill's installed directory (where `customize.toml` lives).
- `{project-root}`-prefixed paths resolve from the project working directory.
- `{skill-name}` resolves to the skill directory's basename.

## On Activation

### Step 1: Resolve the Workflow Block

Run: `python3 {project-root}/_bmad/scripts/resolve_customization.py --skill {skill-root} --key workflow`

**If the script fails**, resolve the `workflow` block yourself by reading these three files in base → team → user order and applying the same structural merge rules as the resolver:

1. `{skill-root}/customize.toml` — defaults
2. `{project-root}/_bmad/custom/{skill-name}.toml` — team overrides
3. `{project-root}/_bmad/custom/{skill-name}.user.toml` — personal overrides

Any missing file is skipped. Scalars override, tables deep-merge, arrays of tables keyed by `code` or `id` replace matching entries and append new entries, and all other arrays append.

### Step 2: Execute Prepend Steps

Execute each entry in `{workflow.activation_steps_prepend}` in order before proceeding.

### Step 3: Load Persistent Facts

Treat every entry in `{workflow.persistent_facts}` as foundational context. Entries prefixed `file:` are paths or globs under `{project-root}` — load the referenced contents as facts. All other entries are facts verbatim.

### Step 4: Load Config

Load config from `{project-root}/_bmad/bmm/config.yaml` and resolve:
- `project_name`, `user_name`, `communication_language`, `document_output_language`, `user_skill_level`
- `implementation_artifacts`; `sprint_status` = `{implementation_artifacts}/sprint-status.yaml`
- `output_folder`; UAT roots: `uat_root` = `{output_folder}/uat`, `cases_dir` = `{uat_root}/cases`
- `date` as system-generated current datetime

### Step 5: Greet the User

Greet `{user_name}`, speaking in `{communication_language}`.

### Step 6: Execute Append Steps

Execute each entry in `{workflow.activation_steps_append}` in order.

Activation is complete. Do not begin the main workflow until all activation steps complete.

## Reference

Read `references/case-schema.md` for the full UAT case field definition (10 business fields + 4 governance fields) and the data-contract schema. Use the case template at `assets/uat-case-template.md` as the output structure.

## Execution

### Step 1: Determine Scope

Determine which stories to build cases for:
- If the user named stories (e.g. "1-2-order, 1-5-coupon"), use that set.
- If invoked with an incremental rebuild list (from `bmad-uat-impact-scan`'s stale-case list), use exactly those cases/stories.
- If "all" or unspecified, read `{sprint_status}` and select all stories with status `done` (or as the user clarifies).
- Confirm the resolved scope with the user before proceeding.

### Step 2: Gather Story + Implementation Context

For each in-scope story:
- Read the COMPLETE story file: parse Story, Acceptance Criteria, Tasks/Subtasks, Dev Notes, File List, Status, and `baseline_commit` if present.
- From the File List / implementation paths, read the relevant implemented code to understand the real user-facing flow, the API endpoints involved, and the observable outcomes (page changes, status transitions, data changes).
- Record the current version anchor: prefer the story's `baseline_commit`; otherwise capture `git rev-parse HEAD` for the story's implementation area.

### Step 3: Design Cases (business language, end-to-end, exceptions)

For each story, design cases that satisfy the four UAT requirements:
1. **Business language, not technical** — steps say "点击下单按钮""页面提示优惠券已过期", never "调用 /api/order""校验 coupon 表".
2. **End-to-end closed loop** — chain the full story flow (e.g. 搜索商品 → 加入购物车 → 支付 → 商家后台查看订单状态变更), not isolated clicks.
3. **Business exception scenarios** — beyond the happy path, cover real frustrations (余额不足、优惠券过期、库存售罄) and verify friendly business prompts.
4. **Operable** — concrete preconditions, concrete query/config steps, concrete expected results, and an empty 实际结果 column for execution.

### Step 4: Fill Governance Fields per Case

For every case, set the four machine-governance fields (see `references/case-schema.md`):
- **数据契约 (data_contract)** — structured, machine-readable precondition declaring exactly what data/state the case needs (entities, states, whether auto-generatable). This is what `bmad-uat-prepare-data` consumes.
- **story_key + 版本锚点 (version_anchor)** — the story key and its baseline_commit/anchor for staleness detection.
- **执行模式 (exec_mode)** — `auto` (machine can objectively judge: status/jump/amount), `manual` (needs human subjective acceptance: 文案是否友好/流程是否符合习惯), or `semi` (machine runs the flow, human confirms result).
- **破坏性 (destructive)** — `read-only` (shares baseline data) or `write-isolated` (mutates state like 库存/余额/订单状态 → needs a dedicated business entity or isolated snapshot).

### Step 5: Write the Case Library

- Write/update cases under `{cases_dir}/`, organized by story_key (e.g. `{cases_dir}/1-2-order/UAT-ORDER-001.md` or a per-story file — follow `assets/uat-case-template.md`).
- Case IDs follow `UAT-<MODULE>-<NNN>` (e.g. UAT-ORDER-003).
- On incremental rebuild, replace only the targeted cases; preserve others and their history.

### Step 6: Summarize Data Needs and Hand Off

After writing cases, aggregate and de-duplicate all data contracts across the built cases. Output:
- Count of cases built/updated per story.
- A consolidated data-needs summary: which items are auto-generatable vs which require user-provided external material.
- **Proactive prompt:** "已识别 N 项数据需求，其中 X 项需你提供。请运行 `bmad-uat-prepare-data` 进行数据准备与校验。"

Validate against `./checklist.md` before finishing.

## On Complete

Run: `python3 {project-root}/_bmad/scripts/resolve_customization.py --skill {skill-root} --key workflow.on_complete`

If the resolved `workflow.on_complete` is non-empty, follow it as the final terminal instruction before exiting.
