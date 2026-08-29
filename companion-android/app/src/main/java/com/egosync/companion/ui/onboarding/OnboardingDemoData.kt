package com.egosync.companion.ui.onboarding

import com.egosync.companion.sync.RoleProposal

/**
 * 引导演示数据（Story 13.2 从 SnapshotStore mock 迁出）：开场/步骤文案/回复轮换/
 * 涌现提案属**生成性内容**（LLM 驱动，指令通道 13.3 接入），不进快照通道。
 */

/** 引导开场问候（FR-21，镜像桌面 startOnboarding 失败 fallback 文案）。 */
val onboardingGreeting: String =
    "你好，我是你的数字分身管家。很高兴为你服务！聊聊你最近在忙什么？"

/** 五步访谈输入 placeholder（FR-21，镜像桌面 OnboardingView placeholders 逐字）。 */
val onboardingPlaceholders: List<String> = listOf(
    "输入你的名字或昵称...",
    "比如：我是一个产品经理，最近在忙新产品上线...",
    "聊聊你最关注的方面...",
    "确认，或者告诉我你想调整...",
    "好的，创建吧！",
)

/** 五步访谈管家回复（mock；桌面由 LLM 流式生成）。 */
val onboardingReplies: List<String> = listOf(
    "很高兴认识你。我平时会帮你把工作、家庭、学习这些方面都照顾到——先从你最常忙的事说起？",
    "听起来这是个重要的方向。要不要我帮你盯着这类事？到了合适的时机我会主动提醒你。",
    "明白了。看来这方面值得专门立一个角色来跟进，稍后我可能会有个提议。",
    "好的，都记下了。我会把这些偏好放进你的个人画像，之后的服务都会以此为基准。",
    "那就这么定了。角色建好之后，任务、记忆和简报都会围绕它展开。有任何事随时叫我。",
)

/** 引导涌现提案种子（Preview/VM 复用 ChatDemoData.roleProposal 亦可，此处就近引用）。 */
val onboardingRoleProposal: RoleProposal = RoleProposal(
    name = "策划师",
    icon = "lightbulb",
    color = "#8B5CF6",
    goal = "把零散的想法收敛成可执行的方案",
)
