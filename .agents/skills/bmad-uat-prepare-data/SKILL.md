---
name: bmad-uat-prepare-data
description: Prepare and validate real (non-mock) business test data for UAT cases by fulfilling the data contracts that bmad-uat-build-cases produced. Aggregates and de-duplicates all case data contracts, auto-generates data that can be built via existing APIs/scripts, and for external real materials (real ID photos, specific third-party real accounts, manual-review materials) emits a fill-in table requesting the user to provide file paths under a central data directory. After the user fills it, validates the data for completeness and usability. Use when the user says "prepare UAT data", "准备 UAT 数据", "备数据", or when the UAT 验收官 dispatches the PD menu item. Runs independently and can be re-run for regression (cases unchanged but environment data needs re-seeding).
---

# Prepare UAT Data Workflow

**Goal:** Turn the data contracts declared by UAT cases into real, validated test data sitting in a dedicated UAT environment / central data directory — auto-generating what is possible and collaborating with the user for the rest.

**Your Role:** You are a UAT data steward. You fulfill and validate data; you do NOT write cases (that is `bmad-uat-build-cases`) and you do NOT execute tests (that is `bmad-uat-run`).

## Conventions

- Bare paths resolve from the skill root. `{skill-root}` is this skill's directory; `{project-root}` is the project working directory; `{skill-name}` is the directory basename.

## On Activation

### Step 1: Resolve the Workflow Block

Run: `python3 {project-root}/_bmad/scripts/resolve_customization.py --skill {skill-root} --key workflow`

**If the script fails**, resolve the `workflow` block yourself by reading `{skill-root}/customize.toml`, then `{project-root}/_bmad/custom/{skill-name}.toml`, then `{project-root}/_bmad/custom/{skill-name}.user.toml` (base → team → user), applying BMad merge rules (scalars override, arrays-of-tables keyed by code/id replace+append, other arrays append).

### Step 2: Execute Prepend Steps
Execute each entry in `{workflow.activation_steps_prepend}` in order.

### Step 3: Load Persistent Facts
Treat each `{workflow.persistent_facts}` entry as foundational context; `file:` entries are globs under `{project-root}` to load.

### Step 4: Load Config
Load `{project-root}/_bmad/bmm/config.yaml` and resolve `user_name`, `communication_language`, `document_output_language`, `output_folder`. UAT roots: `uat_root` = `{output_folder}/uat`, `cases_dir` = `{uat_root}/cases`, `data_dir` = `{uat_root}/data` (overridable via `{workflow}` config), `data_requests_dir` = `{data_dir}/data-requests`, `snapshots_dir` = `{uat_root}/snapshots`.

### Step 5: Greet the User
Greet `{user_name}` in `{communication_language}`.

### Step 6: Execute Append Steps
Execute each entry in `{workflow.activation_steps_append}` in order. Activation complete.

## Execution

### Step 1: Collect and Aggregate Data Contracts
- Determine scope (story set or case set; default = all cases in `{cases_dir}`, or the scope passed by the caller).
- Read each in-scope case's `data_contract` (frontmatter). Aggregate ALL entity requirements across cases and **de-duplicate** by (type, ref, state). Note which cases share an entity.

### Step 2: Classify Each Data Item (auto vs user-provided) — P4
- `auto_generatable: true` → mark for automatic generation (test products/accounts, expired coupons, zero-stock states, etc.).
- `auto_generatable: false` → external real material (real ID photo, specific third-party real account, manual-review material) → must be user-provided.

### Step 3: Auto-Generate What's Possible
- For auto items, construct data via the project's existing APIs/scripts/seed tooling against the **dedicated UAT environment** (production-isolated; external deps point at sandboxes — see persistent facts).
- For `write-isolated` entities, create dedicated business entities (their own SKU/account/coupon) so cases do not collide; record their identifiers.
- Record each generated entity as a snapshot record under `{snapshots_dir}/` (entity id, type, state, owning case ids, reset/cleanup notes) so `bmad-uat-run` can reset/cleanup per case.

### Step 4: Request User-Provided Data via Fill-In Table
- For non-auto items, generate a fill-in table at `{data_requests_dir}/data-request-{{date}}.md` using `assets/data-request-template.md`. The table lists, per item: 需求项、数据要求规格、归属用例、用户填写路径列。
- Tell the user the central directory (`{data_dir}`) to place files, then **HALT** and wait for the user to fill paths.

### Step 5: Validate Completeness and Usability
Once the user reports done (or after auto-generation), validate:
- **完整性**：每个 data_contract 项都有对应数据（自动生成成功 / 用户填了存在的路径）。缺失项明确列出。
- **可用性**：用户提供的文件确实存在、可读、符合规格（格式/大小/必要字段）；自动生成的实体确实处于声明状态（如优惠券确为已过期、库存确为0）。
- **一致性（数据-story 语义）**：引用的实体相互自洽（如订单引用的商品真实存在、状态合法），不存在脏/矛盾数据。
- 输出校验结果：✅ 通过项 / ⚠️ 需修正项（附原因与修正建议）。若有未通过项，HALT 请用户修正后重跑本步。

### Step 6: Summarize and Hand Off
Output a data-readiness summary (按用例：数据是否就绪）。When all green: "数据已就绪。请运行 `bmad-uat-run --mode=dev`（开发阶段验收）或 `--mode=regression`（回归）。"

Validate against `./checklist.md` before finishing.

## On Complete
Run: `python3 {project-root}/_bmad/scripts/resolve_customization.py --skill {skill-root} --key workflow.on_complete`. If non-empty, follow it as the final terminal instruction.
