package com.egosync.companion.ui

import com.egosync.companion.sync.BigRockResult
import com.egosync.companion.sync.BriefingActionPoint
import com.egosync.companion.sync.BriefingSection
import com.egosync.companion.sync.MorningBriefing
import com.egosync.companion.sync.NoticeItem
import com.egosync.companion.sync.NoticeLevel
import com.egosync.companion.sync.RoleCard
import com.egosync.companion.sync.RoleScore
import com.egosync.companion.sync.TaskItem
import com.egosync.companion.sync.TaskOwner
import com.egosync.companion.sync.TaskProtectionStatus
import com.egosync.companion.sync.WeeklyReview
import com.egosync.companion.ui.theme.Quadrant
import com.egosync.companion.ui.theme.RoleDomain

/**
 * Preview / sample() 专用样例数据（Story 13.2 从 SnapshotStore mock 迁出）：
 * **不参与快照通道**，仅供 Compose Preview 与 UiState.sample() 呈现真实布局。
 * 内容对齐原 mock（视觉零回归基准，AC6）。
 */
internal val previewRoles: List<RoleCard> = listOf(
    RoleCard(
        id = "role-pm",
        name = "产品经理",
        icon = "target",
        domain = RoleDomain.WORK,
        energy = 82,
        pendingCount = 3,
        lastActive = "10 分钟前",
        taskCount = 14,
        memoryCount = 38,
        sessionCount = 26,
    ),
    RoleCard(
        id = "role-father",
        name = "父亲",
        icon = "home",
        domain = RoleDomain.FAMILY,
        energy = 56,
        pendingCount = 1,
        lastActive = "昨天",
        taskCount = 6,
        memoryCount = 17,
        sessionCount = 12,
    ),
    RoleCard(
        id = "role-learner",
        name = "学习者",
        icon = "book-open",
        domain = RoleDomain.LEARN,
        energy = 33,
        pendingCount = 0,
        lastActive = "4 天前",
        taskCount = 9,
        memoryCount = 21,
        sessionCount = 8,
    ),
)

internal val previewTasks: List<TaskItem> = listOf(
    TaskItem(
        id = "t-1",
        title = "产品评审会议材料",
        quadrant = Quadrant.Q1,
        roleName = "产品经理",
        ownerType = TaskOwner.ROLE,
        roleId = "role-pm",
        due = "今天 14:00",
        bigRock = false,
    ),
    TaskItem(
        id = "t-2",
        title = "回复供应商询价邮件",
        quadrant = Quadrant.Q1,
        roleName = "产品经理",
        ownerType = TaskOwner.ROLE,
        roleId = "role-pm",
        due = "今天 17:00 前",
        bigRock = false,
    ),
    TaskItem(
        id = "t-3",
        title = "读完《深度工作》第 3 章并写笔记",
        quadrant = Quadrant.Q2,
        roleName = "学习者",
        ownerType = TaskOwner.ROLE,
        roleId = "role-learner",
        due = "本周日",
        bigRock = true,
        // FR-24：学习者角色 4 天未活跃（≥ AT_RISK_DAYS=3），Q2 大石头被持续挤压
        protectionStatus = TaskProtectionStatus.AT_RISK,
    ),
    TaskItem(
        id = "t-4",
        title = "预约女儿的钢琴课时间",
        quadrant = Quadrant.Q2,
        roleName = "父亲",
        ownerType = TaskOwner.ROLE,
        roleId = "role-father",
        due = null,
        bigRock = true,
    ),
    TaskItem(
        id = "t-5",
        title = "整理季度 OKR 草稿",
        quadrant = Quadrant.Q2,
        roleName = "产品经理",
        ownerType = TaskOwner.ROLE,
        roleId = "role-pm",
        due = "周五",
        bigRock = false,
    ),
    TaskItem(
        id = "t-6",
        title = "给母亲回电话",
        quadrant = Quadrant.Q3,
        roleName = "父亲",
        ownerType = TaskOwner.ROLE,
        roleId = "role-father",
        due = "今晚",
        bigRock = false,
    ),
    TaskItem(
        id = "t-7",
        title = "取干洗衣物",
        quadrant = Quadrant.Q3,
        roleName = "管家",
        // 管家归属任务：ownerType=BUTLER + roleId=null（桌面 ownerKey 恒为 "butler"）
        ownerType = TaskOwner.BUTLER,
        roleId = null,
        due = "今天",
        bigRock = false,
    ),
    TaskItem(
        id = "t-8",
        title = "刷 20 分钟行业资讯",
        quadrant = Quadrant.Q4,
        roleName = "产品经理",
        ownerType = TaskOwner.ROLE,
        roleId = "role-pm",
        due = null,
        bigRock = false,
    ),
)

internal val previewNotices: List<NoticeItem> = listOf(
    NoticeItem(
        id = "n-knock-1",
        level = NoticeLevel.KNOCK,
        text = "boss，产品评审改到了今天下午 2 点，和接孩子的钢琴课时间撞了——需要你决定一下。",
        fromRole = "管家",
        time = "10 分钟前",
        actionable = true,
    ),
    NoticeItem(
        id = "n-knock-2",
        level = NoticeLevel.KNOCK,
        text = "供应商的报价明天到期，要不要让产品经理先把比价表整理出来？",
        fromRole = "产品经理",
        time = "2 小时前",
        actionable = true,
        read = true,
    ),
    // FR-24 Q2 保护提醒（母本 q2_protection_reminder.rs:122-135：轻触级通知、文案逐字镜像）
    NoticeItem(
        id = "n-tap-3",
        level = NoticeLevel.TAP,
        text = "你的'读完《深度工作》第 3 章并写笔记'任务已经 3 天没动了，要不要今天安排一下？",
        fromRole = "学习者",
        time = "1 小时前",
    ),
    NoticeItem(
        id = "n-tap-1",
        level = NoticeLevel.TAP,
        text = "顺便说一句，父亲角色提醒这周五是女儿钢琴课。",
        fromRole = "父亲",
        time = "3 小时前",
    ),
    NoticeItem(
        id = "n-tap-2",
        level = NoticeLevel.TAP,
        text = "学习者昨晚沉淀了 2 条读书笔记，已归档。",
        fromRole = "学习者",
        time = "昨天 23:40",
        read = true,
    ),
    NoticeItem(
        id = "n-whisper-1",
        level = NoticeLevel.WHISPER,
        text = "产品经理整理了 3 条竞品对比要点。",
        fromRole = "产品经理",
        time = "昨天",
    ),
    NoticeItem(
        id = "n-whisper-2",
        level = NoticeLevel.WHISPER,
        text = "父亲角色更新了家庭日历（女儿钢琴课改期）。",
        fromRole = "父亲",
        time = "昨天",
        read = true,
    ),
)

internal val previewBriefing: MorningBriefing = MorningBriefing(
    date = "8 月 25 日 · 周二",
    greeting = "早上好 boss。今天你的产品经理角色有个 14:00 的评审会议，材料概要已经备好；父亲角色提醒今天 16:30 是女儿的钢琴课；学习者那边昨晚沉淀了 2 条读书笔记，不着急处理。",
    sections = listOf(
        BriefingSection(
            title = "今日日程",
            body = "09:30 晨间规划 · 14:00 产品评审（材料已备） · 16:30 女儿钢琴课 · 20:00 家庭时间",
        ),
        BriefingSection(
            title = "昨夜沉淀",
            body = "学习者新增 2 条《深度工作》读书笔记；产品经理整理了评审用的竞品对比要点，已归档。",
        ),
        BriefingSection(
            title = "角色状态",
            body = "产品经理能量 82%，状态很好；父亲 56%，钢琴课相关事项需要留意；学习者 33%，有几天没读书了——今天最重要的一件事：评审前把竞品对比页过一遍。",
        ),
    ),
    actionPoints = listOf(
        BriefingActionPoint(
            id = "ap-1",
            text = "评审前 30 分钟提醒你过一遍竞品对比页",
            fromRole = "产品经理",
        ),
        BriefingActionPoint(
            id = "ap-2",
            text = "钢琴课结束后提醒你问问女儿上课感受",
            fromRole = "父亲",
        ),
    ),
)

internal val previewWeeklyReview: WeeklyReview = WeeklyReview(
    weekLabel = "第 34 周 · 8 月 17 日 — 8 月 23 日",
    narrative = "这周你在工作和家庭两个维度都稳步推进。产品经理完成 5/7 项任务，能量从 68% 升到 82%；父亲角色的大石头「陪女儿看画展」已经完成；学习者的「读完第 3 章」还在推进中，不急。",
    roleScores = listOf(
        RoleScore(roleName = "产品经理", completed = 5, total = 7, energyNow = 82, energyDelta = +14),
        RoleScore(roleName = "父亲", completed = 3, total = 4, energyNow = 56, energyDelta = -6),
        RoleScore(roleName = "学习者", completed = 1, total = 3, energyNow = 33, energyDelta = -9),
    ),
    energyTrend = listOf(62, 58, 66, 71, 74, 70, 76),
    energyTrendDays = listOf("一", "二", "三", "四", "五", "六", "日"),
    bigRockResults = listOf(
        BigRockResult(text = "陪女儿看画展", completed = true),
        BigRockResult(text = "读完《深度工作》第 3 章", completed = false),
        BigRockResult(text = "完成季度路线图初稿", completed = true),
    ),
    closingQuestion = "下周想先关注哪个方面？我可以帮你把大石头先排进日历。",
)
