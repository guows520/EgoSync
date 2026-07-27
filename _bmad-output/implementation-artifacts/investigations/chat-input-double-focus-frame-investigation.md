# Investigation: chat-input-double-focus-frame

## Hand-off Brief

1. **What happened.** 管家和角色界面的输入框来自同一个 `ChatInput`，聚焦时同时保留静态灰色 `border`，再叠加组件自身的 `focus:ring`；键盘可见焦点还会叠加全局 `outline`。
2. **Where the case stands.** 根因已达到 High confidence 的源码确认级别；已按确认方案修改共享输入组件并通过定向测试和生产构建。问题不是两个界面各自重复渲染，而是同一输入组件叠加了多套焦点视觉规则。
3. **What's needed next.** 选择并实现“单一焦点视觉拥有者”方案，优先让 `ChatInput` 保留一个 ring/highlight、将原灰色 border 设为透明或改为同色焦点边框，并显式处理全局 `outline` 的优先级。

## Case Info

| Field            | Value |
| ---------------- | ----- |
| Ticket           | N/A |
| Date opened      | 2026-07-25 |
| Status           | Concluded (implemented and verified) |
| System           | Windows workspace; React 18.2 + TypeScript 5.2 + TailwindCSS 3.3.5 |
| Evidence sources | `ChatInput.tsx`, `ChatStream.tsx`, `ButlerView.tsx`, `RoleView.tsx`, `index.css`, git history/blame |

## Problem Statement

用户观察到：管家和角色界面的对话录入框选中后会高亮，但高亮框与原来的灰色框同时存在，视觉上显得重复、不协调。本调查只分析原因和方案，不执行修复。

## Evidence Inventory

| Source   | Status | Notes |
| -------- | ------ | ----- |
| Shared input source | Available | `egosync-app/src/components/chat/ChatInput.tsx:155-169` |
| Butler mount path | Available | `egosync-app/src/components/butler/ButlerView.tsx:132-149` passes `role={null}` to `ChatStream` |
| Role mount path | Available | `egosync-app/src/components/role/RoleView.tsx:92-94` passes the role to `ChatStream` |
| Shared `ChatInput` mount | Available | `egosync-app/src/components/chat/ChatStream.tsx:1243-1255` |
| Global focus CSS | Available | `egosync-app/src/index.css:66-73` |
| Version-control history | Available | `fbbc751` (2026-06-27), `2daf3cd` (2026-06-29), `3b925650` (2026-07-07) |
| Browser screenshot / live repro | Missing | Not needed to identify the CSS overlap, but needed for final visual tuning |
| Automated focus-style assertion | Missing | Existing tests do not assert the exact visual ownership/selector combination |

## Investigation Backlog

| # | Path to Explore | Priority | Status | Notes |
| - | --------------- | -------- | ------ | ----- |
| 1 | Choose one focus-indicator strategy for `ChatInput` | High | Done | Diagnosis identifies the conflict; product styling preference remains to be selected |
| 2 | Verify mouse focus and keyboard focus in Butler and Role views | Medium | Open | Required after implementation, especially because `:focus-visible` is involved |
| 3 | Add a regression assertion for the chosen class contract | Low | Open | Prevents reintroducing simultaneous border/ring/outline ownership |

## Timeline of Events

| Time | Event | Source | Confidence |
| --- | ----- | ----- |
| 2026-05-24 | Original `ChatInput` established with a static border and focus styles | git blame on `ChatInput.tsx:169` | Confirmed |
| 2026-06-27 | Fix for black focus border removed `focus:border-slate-500` but retained static gray border and `focus:ring-2` | commit `fbbc751`, `ChatInput.tsx` diff | Confirmed |
| 2026-06-29 | Accessibility change split global focus behavior into `:focus:not(:focus-visible)` and `:focus-visible` | commit `2daf3cd`, `index.css:66-73` | Confirmed |
| 2026-07-07 | Global keyboard-focus outline changed to a 1px role-accent outline with 1px offset | commit `3b925650`, `index.css:70-72` | Confirmed |
| 2026-07-25 | Current source contains all three relevant layers | current source | Confirmed |

## Confirmed Findings

### Finding 1: Both affected screens use the same input implementation

**Evidence:** `egosync-app/src/components/butler/ButlerView.tsx:132-149`, `egosync-app/src/components/role/RoleView.tsx:92-94`, `egosync-app/src/components/chat/ChatStream.tsx:1243-1255`

**Detail:** Butler and Role views both render `ChatStream`; `ChatStream` renders one shared `ChatInput`. The role-specific difference is only the `role` prop and placeholder/accent behavior. There is no evidence of two independent input frames being rendered in either screen.

### Finding 2: The input keeps a gray border while adding a focus ring

**Evidence:** `egosync-app/src/components/chat/ChatInput.tsx:169`

**Detail:** The same class list contains `border border-slate-200 dark:border-slate-600` and `focus:ring-2 focus:ring-slate-500/20`. Tailwind rings are rendered as an additional box-shadow outside the element border; they do not replace the existing border. Therefore the gray border remains visible whenever the focus ring appears.

### Finding 3: A global focus-visible outline can add a third visible frame

**Evidence:** `egosync-app/src/index.css:66-73`

**Detail:** For `:focus-visible`, the global rule applies `outline: 1px ... !important` and `outline-offset: 1px !important`. The input's local `focus:outline-none` is not marked important, so it does not reliably override this global rule. Text inputs commonly match `:focus-visible` for keyboard interaction, and the global rule is intentionally designed to show a visible focus indicator. Thus the observed extra highlight can be the local ring plus the global outline, in addition to the static border.

### Finding 4: The current overlap was introduced by separate fixes with conflicting visual ownership

**Evidence:** commit `fbbc751` removed `focus:border-slate-500` while retaining the static border and focus ring; commit `2daf3cd`/`3b925650` later introduced the global visible outline.

**Detail:** Before `fbbc751`, the focus border changed color. After that commit, focus no longer changes the border, so the original gray border is guaranteed to remain. The later accessibility rule added another focus indicator without making `ChatInput` opt out of its local ring. This is a style-contract conflict, not a layout bug.

## Deduced Conclusions

### Deduction 1: The defect is shared and can be fixed in one place

**Based on:** Findings 1-2.

**Reasoning:** Both screens converge on the same `ChatInput`, and the conflicting classes are local to that component.

**Conclusion:** A single `ChatInput` style change should affect both Butler and Role views consistently; separate page-level fixes would duplicate logic and risk visual divergence.

### Deduction 2: Removing only the global outline is insufficient

**Based on:** Findings 2-3.

**Reasoning:** Even without the global outline, `border-slate-200` and `focus:ring-2` remain simultaneously visible.

**Conclusion:** The fix must also decide whether the focus ring replaces or complements the base border. Merely changing global accessibility CSS would leave the reported gray-plus-highlight appearance unresolved.

## Hypothesized Paths

### Hypothesis 1: The main reported artifact is the static gray border plus local ring

**Status:** Confirmed

**Theory:** The user sees the base gray border and the Tailwind focus ring as two frames.

**Supporting indicators:** `ChatInput.tsx:169` contains both the static border and `focus:ring-2`; the earlier fix removed `focus:border-slate-500`.

**Would confirm:** Inspect computed styles while focused and observe `border-color` unchanged plus a non-zero ring shadow.

**Would refute:** Computed styles show the border is transparent/absent while the visual duplicate persists.

**Resolution:** Source-level class inspection and the `fbbc751` diff confirm both rules remain active.

### Hypothesis 2: Keyboard focus adds an additional global outline

**Status:** Confirmed at source level; visual contribution unmeasured

**Theory:** When the input matches `:focus-visible`, `index.css` adds an outline outside the local ring.

**Supporting indicators:** `index.css:70-72` uses `!important`; `ChatInput.tsx:169` only has non-important `focus:outline-none`; commits `2daf3cd` and `3b925650` introduced the rule.

**Would confirm:** Browser inspection of `:focus-visible` shows the computed outline and local ring simultaneously.

**Would refute:** The runtime/browser never matches `:focus-visible` for the affected interaction.

**Resolution:** Not visually measured in this analysis; source evidence confirms the rule is eligible to apply.

### Hypothesis 3: Butler and Role layouts render duplicate input DOM nodes

**Status:** Refuted

**Theory:** The two visible frames come from two separate input elements.

**Supporting indicators:** User reports the issue in two screens.

**Would confirm:** DOM inspection finds two `ChatInput`/`input` instances in one screen.

**Would refute:** Both screens converge on one `ChatInput` through `ChatStream`.

**Resolution:** Call-site/source trace shows one shared input component per chat stream; no duplicate-render evidence found.

## Missing Evidence

| Gap | Impact | How to Obtain |
| --- | --- | --- |
| Computed styles for mouse focus vs keyboard focus | Quantifies whether the screenshot shows 2 or 3 layers | Inspect the input in both views with DevTools, or run the existing app and capture `border`, `box-shadow`, `outline` |
| Product preference for focus appearance | Determines whether the final indicator should be a ring, a border, or role-colored combination | Confirm desired visual treatment during implementation review |
| Regression test for focus class contract | Current tests may not catch a future reintroduction | Add a focused component/style contract test after choosing the fix |

## Source Code Trace

| Element | Detail |
| --- | --- |
| Error origin | `egosync-app/src/components/chat/ChatInput.tsx:169`, class list combining static border and local focus ring |
| Trigger | User clicks or tabs into the text input rendered by `ChatStream` |
| Condition | Input is enabled and matches `:focus`; for keyboard navigation it may also match `:focus-visible` |
| Related files | `egosync-app/src/components/chat/ChatStream.tsx:1243-1255`; `egosync-app/src/components/butler/ButlerView.tsx:132-149`; `egosync-app/src/components/role/RoleView.tsx:92-94`; `egosync-app/src/index.css:66-73` |

## Conclusion

**Confidence:** High for the root cause; Medium for the exact number of visible layers in a given interaction mode.

The root cause is a conflicting focus-style contract in the shared `ChatInput`. Its base gray border remains unchanged, while `focus:ring-2` adds an outer highlight. The global `*:focus-visible` rule can add another outline and has `!important`, so the component's ordinary `focus:outline-none` is not a reliable opt-out. The issue is therefore shared by Butler and Role views and should be fixed once in `ChatInput`, not by changing either page layout.

## Recommended Next Steps

### Fix direction

**Recommended minimal direction: make `ChatInput` the single visual owner of focus.** Keep one intentional focus indicator and eliminate the extra frame:

1. Preserve border geometry with a transparent focus border (`focus:border-transparent`) while retaining the existing ring, or replace the ring with one explicit focus border if that is the desired visual language.
2. Explicitly opt this input out of the global `:focus-visible` outline using an important Tailwind utility/selector compatible with the project's utility-only CSS convention, so the local ring is not doubled.
3. Keep the change local to `ChatInput.tsx`; do not add separate Butler/Role overrides.
4. Verify both light/dark themes and both mouse/keyboard focus paths. Ensure disabled inputs do not receive the focus treatment.

**Alternative:** Remove the local ring and rely on a redesigned global focus system. This is broader and not recommended for this ticket because it changes focus visuals across unrelated controls and still requires handling the static border.

### Diagnostic

No extra diagnostic logging is needed. The failure is deterministic CSS composition. Before implementation, use DevTools to record `border-color`, `box-shadow`, and `outline` for mouse focus and keyboard focus; after implementation, confirm exactly one visible focus indicator remains.

## Reproduction Plan

1. Open the Butler view and focus the conversation input by click and by Tab.
2. Repeat in any Role view.
3. Observe the input's base gray border, local ring, and (for `:focus-visible`) global outline.
4. Expected after the fix: one coherent focus indicator, no separate gray frame plus highlight frame.

## Side Findings

- The prior fix commit `fbbc751` was aimed at removing a black focus border, but removing `focus:border-slate-500` without replacing the base border ownership created the current gray-border-plus-ring state.
- The global focus rule is accessibility-motivated and should not be removed globally as a shortcut; the scope should be narrowed to the component that already owns a custom focus indicator.
