import { useEffect, useState } from 'react';
import { AlertCircle, Check, ChevronDown, ChevronUp, Loader2, Pencil, Plus, Trash2, Upload, X } from 'lucide-react';
import { getRoleIconComponent, normalizeColorHex } from '../../lib/roleIcons';
import { cn } from '../../lib/utils';
import { appService } from '../../services/appService';
import { scheduleService } from '../../services/scheduleService';
import { missionService } from '../../services/missionService';
import { roleService } from '../../services/roleService';
import { skillService } from '../../services/skillService';
import type { ButlerSkillsConfig, Role } from '../../types/role';
import type { OpencodeSkillCandidate, SkillImportPreview, SkillRegistryEntry } from '../../types/skill';
import type { InferredValues, InferenceEligibility } from '../../types/mission';

type ButlerSkillsState = ButlerSkillsConfig;

type ButlerSkillKey = 'findSkills' | 'skillCreator';

const DEFAULT_SKILLS: ButlerSkillsState = { findSkills: true, skillCreator: false, enabledSkillIds: [] };
const BUTLER_SCOPE_ID = '__butler__';
const MESSAGE_TIMEOUT_MS = 1500;
const SKILL_OPTIONS: Array<{ key: ButlerSkillKey; title: string; source: string; description: string }> = [
  {
    key: 'findSkills',
    title: 'find-skills',
    source: 'Vercel 官方',
    description: '发现并推荐适合当前任务的 Skill。',
  },
  {
    key: 'skillCreator',
    title: 'skill-creator',
    source: 'Anthropic 官方',
    description: '创建或扩展角色需要的新 Skill。',
  },
];

interface ButlerSettingsContentProps {
  activeRoles?: Role[];
  archivedRoles?: Role[];
  onRestoreRole?: (id: string) => Promise<void> | void;
  onUpdateRole?: (role: Role) => void;
}

export function ButlerSettingsContent({ activeRoles = [], archivedRoles = [], onRestoreRole, onUpdateRole }: ButlerSettingsContentProps) {
  const [mission, setMission] = useState('');
  const [missionFormat, setMissionFormat] = useState<'free' | 'structured'>('free');
  const [structuredMission, setStructuredMission] = useState('');
  const [structuredPrinciple, setStructuredPrinciple] = useState('');
  const [structuredRoles, setStructuredRoles] = useState<Array<{ name: string; goal: string }>>([]);
  const [isMissionModalOpen, setIsMissionModalOpen] = useState(false);
  const [isLoadingMission, setIsLoadingMission] = useState(true);
  const [isSavingMission, setIsSavingMission] = useState(false);
  const [missionSavedMessage, setMissionSavedMessage] = useState('');
  const [skills, setSkills] = useState<ButlerSkillsState>(DEFAULT_SKILLS);
  const [isLoadingButlerSkills, setIsLoadingButlerSkills] = useState(true);
  const [registrySkills, setRegistrySkills] = useState<SkillRegistryEntry[]>([]);
  const [skillPreview, setSkillPreview] = useState<SkillImportPreview | null>(null);
  const [opencodeSkills, setOpencodeSkills] = useState<OpencodeSkillCandidate[]>([]);
  const [opencodeSkipped, setOpencodeSkipped] = useState<string[]>([]);
  const [isOpencodeExpanded, setIsOpencodeExpanded] = useState(false);
  const [needsFindSkillsPrompt, setNeedsFindSkillsPrompt] = useState(false);
  const [isDiscoveringOpencode, setIsDiscoveringOpencode] = useState(false);
  const [importingOpencodePath, setImportingOpencodePath] = useState('');
  const [removingOpencodeId, setRemovingOpencodeId] = useState('');
  const [pendingSkillContent, setPendingSkillContent] = useState('');
  const [reuseAllRoles, setReuseAllRoles] = useState(true);
  const [reuseRoleIds, setReuseRoleIds] = useState<string[]>([]);
  const [pendingSkill, setPendingSkill] = useState<string | null>(null);
  const [deleteSkillTarget, setDeleteSkillTarget] = useState<SkillRegistryEntry | null>(null);
  const [isLoadingSkills, setIsLoadingSkills] = useState(false);
  const [isPickingDirectory, setIsPickingDirectory] = useState(false);
  const [isImportingSkill, setIsImportingSkill] = useState(false);
  const [settingsSavedMessage, setSettingsSavedMessage] = useState('');
  const [briefingTime, setBriefingTime] = useState('08:00');
  const [reviewDay, setReviewDay] = useState('7');
  const [reviewTime, setReviewTime] = useState('20:00');
  const [bigrockReminderDay, setBigrockReminderDay] = useState('1');
  const [bigrockReminderTime, setBigrockReminderTime] = useState('09:00');
  const [error, setError] = useState('');
  const [inferredValues, setInferredValues] = useState<InferredValues | null>(null);
  const [isInferring, setIsInferring] = useState(false);
  const [inferenceEligibility, setInferenceEligibility] = useState<InferenceEligibility | null>(null);
  const [isInferenceModalOpen, setIsInferenceModalOpen] = useState(false);
  const [editableSummary, setEditableSummary] = useState('');
  const templates = [
    '我的使命是成为一个以原则为中心的人，在生活的各个维度保持平衡与成长。\n\n生活：保持身心健康，经济上勤勉节俭，为家人提供安稳的生活基础。\n关爱：把家人放在第一位——每周至少两个晚上专属陪伴，在重要决策中先问"这对家庭意味着什么"。对朋友真诚相待，值得信任。\n学习：保持终身学习者的心态，每月至少读完一本书或掌握一项新技能，用成长带动身边的人。\n遗产：通过专业能力创造真实价值，每年至少完成一个有长期影响力的项目，让世界因我的存在而更好一点。',
    '我的使命是以家庭为根基，以事业为翅膀，在两者之间找到动态平衡。\n\n作为伴侣和父母：我是家人可以依靠的人。无论工作多忙，家人的健康与快乐始终是第一优先级。每周保留专属家庭时间，重要家庭事件不因工作让步。\n作为职业人：在工作中追求卓越和影响力，但绝不以牺牲家庭为代价。优先做有长期价值的事，而非短期回报的事。\n作为学习者：每季度审视一次生活平衡状态，及时调整。保持开放心态，从每次挫折中学习。\n作为社区成员：力所能及地回馈社会，每年参与至少一次公益或志愿服务。',
    '我的使命是持续成长自己，并用成长去服务和影响他人。\n\n个人成长：每天保留30分钟独处反思时间，定期审视生活是否偏离本心。每月深入阅读一本书，每年掌握一个全新领域。\n家庭关系：用耐心和倾听经营亲密关系，成为家人成长的支持者，而非评判者。\n专业贡献：用专业能力解决真实问题，主动分享知识与经验，帮助他人少走弯路。\n精神传承：活出自己想要传递给下一代的价值观——诚实、勤勉、善良、勇气。决定权始终在自己手中，但要为每个决定负责。',
    '家庭第一。\n在诚信问题上决不妥协。\n未听取正反双方意见，不妄下断语。\n征求他人意见。\n诚恳但立场坚定。\n每年掌握一种新技能。\n今天计划明天的工作。\n态度积极。\n保持幽默感。\n生活与工作有条不紊。\n别怕犯错，怕的是不能吸取教训。\n协助属下成功。\n多请教别人。\n专注于当前的工作，不为下一次任务或提升瞎操心。',
  ];

  const structuredTemplate = {
    mission: '堂堂正正地生活，并且对他人有所影响，对社会有所贡献。',
    principle: '有慈悲心——亲近人群，不分贵贱，热爱每一个人。\n甘愿牺牲——为人生使命奉献时间、才智和金钱。\n激励他人——以身作则，证明人为万物之长，可以克服一切困难。\n施加影响——用实际行动改善他人的生活。',
    roles: [
      { name: '丈夫/妻子', goal: '妻子/丈夫是我这一生中最重要的人，我们同甘共苦，携手前行。' },
      { name: '父亲/母亲', goal: '我要帮助子女体验乐趣无穷的人生。' },
      { name: '儿子/兄弟', goal: '我不忘父母、手足的亲情，随时对他们施以援手。' },
      { name: '变革者', goal: '我能激发和催化团队成员的优异表现。' },
      { name: '学者', goal: '我每天都学习很多重要的新知识。' },
    ],
  };

  useEffect(() => {
    let cancelled = false;
    setIsLoadingMission(true);
    missionService.get()
      .then(m => {
        if (!cancelled && m) {
          setMissionFormat(m.format);
          if (m.format === 'structured' && m.content) {
            try {
              const parsed = JSON.parse(m.content);
              if (Array.isArray(parsed.roles)) {
                setStructuredMission(parsed.mission ?? '');
                setStructuredPrinciple(parsed.principle ?? '');
                setStructuredRoles(parsed.roles.map((r: { name?: string; goal?: string }) => ({ name: r.name ?? '', goal: r.goal ?? '' })));
              } else {
                setStructuredMission(parsed.mission ?? '');
                setStructuredPrinciple(parsed.principle ?? parsed.value ?? '');
                setStructuredRoles([{ name: parsed.role ?? '', goal: parsed.goal ?? '' }]);
              }
            } catch {
              setMissionFormat('free');
              setMission(m.content);
            }
          } else {
            setMission(m.content ?? '');
          }
        }
        if (!cancelled) {
          missionService.checkInferenceEligibility()
            .then(eligibility => {
              if (!cancelled) setInferenceEligibility(eligibility);
            })
            .catch(() => {
              if (!cancelled) setInferenceEligibility(null);
            });
        }
      })
      .catch(() => {
        if (!cancelled) setError('使命宣言加载失败，请稍后重试');
      })
      .finally(() => {
        if (!cancelled) setIsLoadingMission(false);
      });
    return () => {
      cancelled = true;
    };
  }, []);

  const refreshEligibility = () => {
    missionService.checkInferenceEligibility()
      .then(eligibility => setInferenceEligibility(eligibility))
      .catch(() => setInferenceEligibility(null));
  };

  useEffect(() => {
    scheduleService.getSchedule()
      .then(config => {
        setBriefingTime(config.briefingTime);
        setReviewDay(config.reviewDay);
        setReviewTime(config.reviewTime);
        setBigrockReminderDay(config.bigrockReminderDay);
        setBigrockReminderTime(config.bigrockReminderTime);
      })
      .catch(() => {});
  }, []);

  useEffect(() => {
    let cancelled = false;
    setIsLoadingButlerSkills(true);
    appService.getButlerSkills()
      .then(nextSkills => {
        if (!cancelled) setSkills({ ...DEFAULT_SKILLS, ...nextSkills, enabledSkillIds: nextSkills.enabledSkillIds ?? [] });
      })
      .catch(() => {
        if (!cancelled) setError('Skill 配置加载失败，请稍后重试');
      })
      .finally(() => {
        if (!cancelled) setIsLoadingButlerSkills(false);
      });
    return () => {
      cancelled = true;
    };
  }, []);

  useEffect(() => {
    let cancelled = false;
    setIsLoadingSkills(true);
    skillService.listAllRoleSkills()
      .then(items => {
        if (!cancelled) setRegistrySkills(items);
      })
      .catch(() => {
        if (!cancelled) setError('Skill 列表加载失败，请稍后重试');
      })
      .finally(() => {
        if (!cancelled) setIsLoadingSkills(false);
      });
    return () => {
      cancelled = true;
    };
  }, []);

  const handleTriggerInference = async () => {
    setIsInferring(true);
    setError('');
    setInferredValues(null);
    try {
      const result = await missionService.inferValues();
      if (result) {
        setInferredValues(result);
        setEditableSummary(result.summary);
        setIsInferenceModalOpen(true);
      } else {
        setError('暂无足够数据进行推断');
      }
    } catch {
      setInferredValues(null);
      setError('推断失败，请稍后重试');
    } finally {
      setIsInferring(false);
    }
  };

  const handleAdoptInferred = async () => {
    if (!inferredValues) return;
    setIsSavingMission(true);
    setError('');
    try {
      await missionService.update(editableSummary, 'free');
      setMission(editableSummary);
      setMissionFormat('free');
      setInferredValues(null);
      setIsInferenceModalOpen(false);
      setMissionSavedMessage('已采纳推断的价值观');
      setTimeout(() => setMissionSavedMessage(''), MESSAGE_TIMEOUT_MS);
      refreshEligibility();
    } catch {
      setError('采纳推断价值观失败，请稍后重试');
    } finally {
      setIsSavingMission(false);
    }
  };

  const handleDismissInferred = () => {
    setInferredValues(null);
    setIsInferenceModalOpen(false);
  };

  const saveSkills = async (nextSkills: ButlerSkillsState, pendingKey: string) => {
    const previousSkills = skills;
    setSkills(nextSkills);
    setPendingSkill(pendingKey);
    setSettingsSavedMessage('');
    setError('');
    try {
      const savedSkills = await appService.updateButlerSkills(nextSkills);
      setSkills({ ...DEFAULT_SKILLS, ...savedSkills, enabledSkillIds: savedSkills.enabledSkillIds ?? [] });
      setSettingsSavedMessage('Skill 配置已保存');
      setTimeout(() => setSettingsSavedMessage(''), MESSAGE_TIMEOUT_MS);
    } catch {
      setSkills(previousSkills);
      setError('Skill 配置保存失败，请稍后重试');
    } finally {
      setPendingSkill(null);
    }
  };

  const handleSkillToggle = async (key: ButlerSkillKey) => {
    await saveSkills({ ...skills, [key]: !skills[key] }, key);
  };

  const handleCustomSkillToggle = async (skillId: string) => {
    const enabled = skills.enabledSkillIds.includes(skillId);
    const nextSkillIds = enabled
      ? skills.enabledSkillIds.filter(id => id !== skillId)
      : [...skills.enabledSkillIds, skillId];
    await saveSkills({ ...skills, enabledSkillIds: nextSkillIds }, skillId);
  };

  const handleDiscoverOpencode = async () => {
    setError('');
    setSettingsSavedMessage('');
    setOpencodeSkills([]);
    setOpencodeSkipped([]);
    setNeedsFindSkillsPrompt(false);
    if (!skills.findSkills) {
      setNeedsFindSkillsPrompt(true);
      return;
    }
    setIsDiscoveringOpencode(true);
    try {
      const result = await skillService.discoverOpencode(BUTLER_SCOPE_ID);
      setOpencodeSkills(result.items);
      setIsOpencodeExpanded(result.items.length > 0);
      setOpencodeSkipped(result.skipped.reasons);
    } catch (e) {
      setError(toFriendlyError(e, '发现 opencode Skill 失败，请稍后重试'));
    } finally {
      setIsDiscoveringOpencode(false);
    }
  };

  const handleEnableFindSkills = async () => {
    await saveSkills({ ...skills, findSkills: true }, 'findSkills');
  };

  const handleImportOpencode = async (candidate: OpencodeSkillCandidate) => {
    if (!skills.findSkills) {
      setNeedsFindSkillsPrompt(true);
      return;
    }
    setImportingOpencodePath(candidate.sourcePath);
    setError('');
    setSettingsSavedMessage('');
    try {
      const result = await skillService.importOpencode(BUTLER_SCOPE_ID, {
        sourcePath: candidate.sourcePath,
        roleScope: { allRoles: false, roleIds: [BUTLER_SCOPE_ID] },
        expectedContentHash: candidate.contentHash,
      });
      const items = await skillService.listAllRoleSkills();
      setRegistrySkills(items);
      if (result.entry) {
        setSkills(prev => ({
          ...prev,
          enabledSkillIds: prev.enabledSkillIds.includes(result.entry!.id)
            ? prev.enabledSkillIds
            : [...prev.enabledSkillIds, result.entry!.id],
        }));
      }
      const duplicate = result.entry ? { kind: 'contentHash' as const, existing: result.entry } : candidate.duplicate;
      setOpencodeSkills(prev => prev.map(item => item.sourcePath === candidate.sourcePath ? { ...item, alreadyImported: true, duplicate } : item));
      if (result.status === 'duplicate') {
        setSettingsSavedMessage('Skill 已存在，已更新复用范围');
      } else if (result.synced) {
        setSettingsSavedMessage('opencode Skill 已导入并启用');
      } else {
        setSettingsSavedMessage('opencode Skill 已导入，但同步到 agent 暂时失败，将在下次同步时自动生效');
      }
    } catch (e) {
      setError(toFriendlyError(e, '导入 opencode Skill 失败，请稍后重试'));
    } finally {
      setImportingOpencodePath('');
    }
  };

  const handleRemoveOpencode = async (candidate: OpencodeSkillCandidate) => {
    const skillId = candidate.duplicate?.existing.id;
    if (!skillId) return;
    setRemovingOpencodeId(skillId);
    setError('');
    setSettingsSavedMessage('');
    try {
      await skillService.removeFromRole(skillId, BUTLER_SCOPE_ID);
      const items = await skillService.listAllRoleSkills();
      setRegistrySkills(items);
      setSkills(prev => ({
        ...prev,
        enabledSkillIds: prev.enabledSkillIds.filter(id => id !== skillId),
      }));
      setOpencodeSkills(prev => prev.map(item => item.sourcePath === candidate.sourcePath ? { ...item, alreadyImported: false } : item));
      setSettingsSavedMessage('opencode Skill 已取消导入');
      setTimeout(() => setSettingsSavedMessage(''), MESSAGE_TIMEOUT_MS);
    } catch (e) {
      setError(toFriendlyError(e, '取消导入 opencode Skill 失败，请稍后重试'));
    } finally {
      setRemovingOpencodeId('');
    }
  };

  const handleSkillDirectorySelected = async () => {
    setIsPickingDirectory(true);
    setError('');
    setSettingsSavedMessage('');
    setSkillPreview(null);
    try {
      const picked = await skillService.pickCustomDirectory();
      const preview = await skillService.previewCustom({ content: picked.content });
      setPendingSkillContent(picked.content);
      setSkillPreview(preview);
      setReuseAllRoles(true);
      setReuseRoleIds([BUTLER_SCOPE_ID]);
    } catch (e) {
      const text = typeof e === 'string' ? e : (JSON.stringify(e) ?? String(e));
      if (text.includes('未选择 Skill 文件夹')) return;
      setPendingSkillContent('');
      setError(toFriendlyError(e, '未能从所选 Skill 文件夹读取 SKILL.md'));
    } finally {
      setIsPickingDirectory(false);
    }
  };

  const handleConfirmImport = async (overwriteExisting = false) => {
    if (!pendingSkillContent) return;
    setIsImportingSkill(true);
    setError('');
    setSettingsSavedMessage('');
    try {
      const result = await skillService.importCustom({
        content: pendingSkillContent,
        overwriteExisting,
        roleScope: { allRoles: reuseAllRoles, roleIds: reuseAllRoles ? [] : reuseRoleIds },
      });
      const items = await skillService.listAllRoleSkills();
      setRegistrySkills(items);
      if (result.entry) {
        const entry = result.entry;
        const refreshedRoles = await roleService.list();
        refreshedRoles.forEach(item => onUpdateRole?.(item));
        const shouldEnableButler = reuseAllRoles || reuseRoleIds.includes(BUTLER_SCOPE_ID);
        const nextSkillIds = shouldEnableButler
          ? (skills.enabledSkillIds.includes(entry.id) ? skills.enabledSkillIds : [...skills.enabledSkillIds, entry.id])
          : skills.enabledSkillIds.filter(id => id !== entry.id);
        await saveSkills({ ...skills, enabledSkillIds: nextSkillIds }, entry.id);
      }
      setSkillPreview(result.status === 'duplicate' ? result.preview : null);
      setPendingSkillContent(result.status === 'duplicate' ? pendingSkillContent : '');
      setSettingsSavedMessage(result.status === 'duplicate' ? 'Skill 已存在，已更新复用范围' : '自定义 Skill 已导入并启用');
    } catch (e) {
      setError(toFriendlyError(e, '导入自定义 Skill 失败，请稍后重试'));
    } finally {
      setIsImportingSkill(false);
    }
  };

  const handleSaveMission = async () => {
    setIsSavingMission(true);
    setMissionSavedMessage('');
    setError('');
    try {
      let content: string | null;
      let format: 'free' | 'structured';
      if (missionFormat === 'structured') {
        const allRolesEmpty = structuredRoles.every(r => !r.name.trim() && !r.goal.trim());
        const isEmpty = !structuredMission.trim() && !structuredPrinciple.trim() && allRolesEmpty;
        content = isEmpty ? null : JSON.stringify({
          mission: structuredMission,
          principle: structuredPrinciple,
          roles: structuredRoles.filter(r => r.name.trim() || r.goal.trim()),
        });
        format = 'structured';
      } else {
        content = mission || null;
        format = 'free';
      }
      await missionService.update(content, format);
      setMissionSavedMessage('使命宣言已保存');
      setTimeout(() => setMissionSavedMessage(''), MESSAGE_TIMEOUT_MS);
      if (missionFormat === 'structured') setIsMissionModalOpen(false);
      refreshEligibility();
    } catch {
      setError('使命宣言保存失败，请稍后重试');
    } finally {
      setIsSavingMission(false);
    }
  };

  const handleCustomSkillDelete = async () => {
    if (!deleteSkillTarget) return;
    const skillId = deleteSkillTarget.id;
    setPendingSkill(skillId);
    setError('');
    setSettingsSavedMessage('');
    try {
      const nextSkillIds = skills.enabledSkillIds.filter(id => id !== skillId);
      await saveSkills({ ...skills, enabledSkillIds: nextSkillIds }, skillId);
      setDeleteSkillTarget(null);
      setSettingsSavedMessage('自定义 Skill 已删除');
      setTimeout(() => setSettingsSavedMessage(''), MESSAGE_TIMEOUT_MS);
    } catch {
      setError('删除自定义 Skill 失败，请稍后重试');
    } finally {
      setPendingSkill(null);
    }
  };

  return (
    <div className="space-y-8">
      <div>
        <label className="text-[14px] font-semibold text-slate-800 block mb-3">如何称呼您</label>
        <input type="text" defaultValue="boss" className="w-full bg-white border border-slate-200 rounded-lg px-4 py-2.5 text-[14px] focus:ring-2 focus:ring-indigo-500/20 focus:border-indigo-500 outline-none" />
        <p className="text-[12px] text-slate-400 mt-2 leading-relaxed">管家在对话中称呼您的方式。</p>
      </div>
      <div className="pt-6 border-t border-slate-200/80">
        <label className="text-[14px] font-semibold text-slate-800 block mb-3">您的个人使命宣言</label>
        <div className="flex gap-2 mb-3">
          <button
            type="button"
            onClick={() => setMissionFormat('free')}
            disabled={isLoadingMission}
            className={cn('px-3 py-1.5 rounded-lg text-[12px] font-medium transition-colors disabled:cursor-not-allowed disabled:opacity-60', missionFormat === 'free' ? 'bg-indigo-600 text-white' : 'bg-white border border-slate-200 text-slate-600 hover:bg-slate-50')}
          >
            自由文本
          </button>
          <button
            type="button"
            onClick={() => setMissionFormat('structured')}
            disabled={isLoadingMission}
            className={cn('px-3 py-1.5 rounded-lg text-[12px] font-medium transition-colors disabled:cursor-not-allowed disabled:opacity-60', missionFormat === 'structured' ? 'bg-indigo-600 text-white' : 'bg-white border border-slate-200 text-slate-600 hover:bg-slate-50')}
          >
            结构化模板
          </button>
          <button
            type="button"
            onClick={handleTriggerInference}
            disabled={!inferenceEligibility?.eligible}
            title={inferenceEligibility && !inferenceEligibility.eligible ? inferenceEligibility.reason : ''}
            className={cn(
              'ml-auto px-3 py-1.5 text-[12px] font-medium transition-colors',
              isInferring
                ? 'pointer-events-none text-slate-400'
                : 'text-slate-500 hover:text-indigo-600 disabled:cursor-not-allowed disabled:opacity-40 disabled:hover:text-slate-500',
            )}
          >
            {isInferring ? (
              <span className="inline-flex items-center gap-1">
                <Loader2 size={12} className="animate-loading-spin text-indigo-500" />
                推断中...
              </span>
            ) : '推断使命宣言'}
          </button>
        </div>
        {isLoadingMission ? (
          <div className="rounded-lg border border-slate-200 bg-white px-4 py-3 text-[13px] text-slate-500">正在加载使命宣言...</div>
        ) : missionFormat === 'free' ? (
          <>
            <textarea
              value={mission}
              onChange={e => setMission(e.target.value)}
              placeholder="个人使命宣言是你人生各角色间的优先级指南。当角色之间发生时间或资源冲突时，管家会依据它来做仲裁建议。"
              rows={6}
              className="w-full bg-white border border-slate-200 rounded-lg px-4 py-2.5 text-[14px] focus:ring-2 focus:ring-indigo-500/20 focus:border-indigo-500 outline-none resize-none placeholder:text-slate-300"
            />
            <div className="mt-3 space-y-2">
              <p className="text-[12px] text-slate-400 mb-1.5">参考模板（点击填入）：</p>
              {templates.map((t, i) => (
                <button key={i} onClick={() => setMission(t)} className="block w-full text-left px-3 py-2 rounded-lg border border-slate-200 text-[13px] text-slate-600 hover:bg-indigo-50 hover:border-indigo-200 hover:text-indigo-700 transition-colors">
                  {t.split('\n')[0]}
                </button>
              ))}
            </div>
          </>
        ) : (
          <>
            <div className="rounded-lg border border-slate-200 bg-white p-4 space-y-3">
              <div>
                <span className="text-[12px] text-slate-400">使命</span>
                <p className={cn('text-[14px] mt-0.5', structuredMission ? 'text-slate-700' : 'text-slate-300')}>
                  {structuredMission || '未设定'}
                </p>
              </div>
              <div>
                <span className="text-[12px] text-slate-400">原则</span>
                <p className={cn('text-[14px] mt-0.5 whitespace-pre-line', structuredPrinciple ? 'text-slate-700' : 'text-slate-300')}>
                  {structuredPrinciple || '未设定'}
                </p>
              </div>
              <div>
                <span className="text-[12px] text-slate-400">角色</span>
                <p className={cn('text-[14px] mt-0.5', structuredRoles.some(r => r.name.trim()) ? 'text-slate-700' : 'text-slate-300')}>
                  {structuredRoles.filter(r => r.name.trim()).map(r => r.name).join('、') || '未设定'}
                </p>
              </div>
            </div>
            <button
              type="button"
              onClick={() => setIsMissionModalOpen(true)}
              className="mt-4 inline-flex items-center gap-1.5 rounded-lg border border-slate-200 px-4 py-2 text-[13px] font-medium text-slate-600 hover:bg-slate-50 transition-colors"
            >
              <Pencil size={14} /> 编辑使命宣言
            </button>
          </>
        )}
        <div className="mt-3 flex items-center gap-3">
          {missionFormat === 'free' && (
            <button
              type="button"
              onClick={handleSaveMission}
              disabled={isSavingMission || isLoadingMission}
              className="inline-flex items-center gap-2 rounded-lg bg-indigo-600 px-4 py-2 text-[13px] font-medium text-white hover:bg-indigo-700 disabled:cursor-not-allowed disabled:opacity-60"
            >
              {isSavingMission && <Loader2 size={14} className="animate-spin" />}
              {isSavingMission ? '保存中...' : '保存使命宣言'}
            </button>
          )}
          {missionSavedMessage && (
            <span className="text-[13px] text-emerald-600 flex items-center gap-1.5">
              <Check size={14} /> {missionSavedMessage}
            </span>
          )}
        </div>
      </div>
      <div className="pt-6 border-t border-slate-200/80">
        <label className="text-[14px] font-semibold text-slate-800 block mb-3">Skill 配置</label>
        {isLoadingButlerSkills ? (
          <div className="rounded-xl border border-slate-200 bg-white px-4 py-3 text-[13px] text-slate-500 shadow-sm">正在加载 Skill 配置...</div>
        ) : (
          <>
        <div className="space-y-3">
          {SKILL_OPTIONS.map(option => {
            const enabled = skills[option.key];
            return (
              <div key={option.key} className="bg-white border border-slate-200 rounded-xl p-4 shadow-sm">
                <div className="flex items-start justify-between gap-4">
                  <div>
                    <div className="flex items-center gap-2.5">
                      <span className={cn('w-2.5 h-2.5 rounded-full', enabled ? 'bg-emerald-500 shadow-[0_0_8px_rgba(16,185,129,0.4)]' : 'bg-slate-300')} />
                      <span className="text-[14.5px] font-medium text-slate-800">{option.title}</span>
                      <span className="text-[11px] font-medium text-slate-500 bg-slate-100 px-2 py-0.5 rounded-full border border-slate-200">{option.source}</span>
                    </div>
                    <p className="mt-2 text-[12.5px] leading-relaxed text-slate-500">{option.description}</p>
                  </div>
                  <button
                    type="button"
                    role="switch"
                    aria-checked={enabled}
                    disabled={pendingSkill !== null}
                    onClick={() => handleSkillToggle(option.key)}
                    className={cn(
                      'relative h-6 w-11 shrink-0 rounded-full transition-colors disabled:cursor-not-allowed disabled:opacity-60',
                      enabled ? 'bg-indigo-600' : 'bg-slate-300',
                    )}
                  >
                    <span className={cn('absolute left-0.5 top-0.5 h-5 w-5 rounded-full bg-white shadow transition-transform', enabled ? 'translate-x-5' : 'translate-x-0')} />
                    <span className="sr-only">{option.title}</span>
                  </button>
                </div>
              </div>
            );
          })}
        </div>

        <div className="mt-5 bg-white border border-slate-200 rounded-xl p-4 shadow-sm">
          <div className="flex flex-col gap-3 sm:flex-row sm:items-center sm:justify-between">
            <div>
              <div className="text-[14.5px] font-medium text-slate-800">opencode 生态 Skill</div>
              <p className="mt-1 text-[12.5px] leading-relaxed text-slate-500">扫描本机 opencode 项目级与全局 Skill 目录，不下载远程内容。</p>
            </div>
            <button
              type="button"
              title="发现 opencode Skill"
              aria-label="发现 opencode Skill"
              onClick={handleDiscoverOpencode}
              disabled={isDiscoveringOpencode}
              className="inline-flex min-w-[3.5rem] shrink-0 items-center justify-center whitespace-nowrap rounded-lg border border-slate-200 bg-white px-3 py-2 text-[12px] font-medium text-slate-700 hover:bg-slate-50 disabled:cursor-not-allowed disabled:opacity-60"
            >
              {isDiscoveringOpencode ? '发现中...' : '发现'}
            </button>
          </div>
          {!skills.findSkills && needsFindSkillsPrompt && (
            <div className="mt-3 rounded-lg border border-amber-100 bg-amber-50 px-3 py-2 text-[12.5px] text-amber-700">
              <div>需要先启用 find-skills 才能发现可用 Skill</div>
              <button
                type="button"
                onClick={handleEnableFindSkills}
                disabled={pendingSkill !== null}
                className="mt-2 rounded-md bg-amber-600 px-3 py-1.5 text-[12px] font-medium text-white hover:bg-amber-700 disabled:cursor-not-allowed disabled:opacity-60"
              >
                启用 find-skills
              </button>
            </div>
          )}
          {opencodeSkipped.length > 0 && (
            <div className="mt-3 rounded-lg border border-amber-100 bg-amber-50 px-3 py-2 text-[12.5px] text-amber-700">
              <div>已跳过 {opencodeSkipped.length} 个无效 Skill：</div>
              <ul className="mt-1 list-disc space-y-0.5 pl-4">
                {opencodeSkipped.slice(0, 5).map((reason, index) => (
                  <li key={`${reason}-${index}`} className="break-words">{reason}</li>
                ))}
              </ul>
              {opencodeSkipped.length > 5 && (
                <div className="mt-1 text-amber-600">…另有 {opencodeSkipped.length - 5} 项已跳过</div>
              )}
            </div>
          )}
          {opencodeSkills.length > 0 && (
            <div className="mt-3 space-y-2">
              <button
                type="button"
                onClick={() => setIsOpencodeExpanded(prev => !prev)}
                className="flex w-full items-center justify-between rounded-lg border border-slate-100 bg-slate-50 px-3 py-2 text-[12.5px] font-medium text-slate-600 hover:bg-slate-100"
              >
                <span>发现 {opencodeSkills.length} 个 opencode Skill</span>
                <span className="inline-flex items-center gap-1">
                  {isOpencodeExpanded ? '收起' : '展开'}
                  {isOpencodeExpanded ? <ChevronUp size={14} /> : <ChevronDown size={14} />}
                </span>
              </button>
              {isOpencodeExpanded && opencodeSkills.map(candidate => {
                const importedSkillId = candidate.duplicate?.existing.id;
                const isRemoving = importedSkillId === removingOpencodeId;
                return (
                  <div key={candidate.sourcePath} className="rounded-lg border border-slate-100 bg-slate-50 px-3 py-3">
                    <div className="flex items-start justify-between gap-3">
                      <div className="min-w-0 flex-1">
                        <div className="flex flex-wrap items-center gap-2">
                          <span className="break-words text-[13.5px] font-medium text-slate-800">{candidate.name}</span>
                          <span className="rounded-full border border-slate-200 bg-white px-2 py-0.5 text-[11px] font-medium text-slate-500">opencode</span>
                          <span className="rounded-full border border-slate-200 bg-white px-2 py-0.5 text-[11px] font-medium text-slate-500">{candidate.sourceLocation}</span>
                          {candidate.alreadyImported && <span className="rounded-full border border-emerald-100 bg-emerald-50 px-2 py-0.5 text-[11px] font-medium text-emerald-700">已导入</span>}
                        </div>
                        <div className="group relative mt-1.5">
                          <p data-testid={`opencode-skill-description-${candidate.contentHash}`} className="line-clamp-2 break-words text-[12px] leading-relaxed text-slate-500">{candidate.description}</p>
                          <div data-testid={`opencode-skill-tooltip-${candidate.contentHash}`} className="pointer-events-none absolute left-0 top-full z-20 mt-1 hidden max-w-[min(28rem,calc(100vw-3rem))] rounded-lg border border-slate-200 bg-white px-3 py-2 text-[12px] leading-relaxed text-slate-600 shadow-xl group-hover:block">
                            {candidate.description}
                          </div>
                        </div>
                      </div>
                      {candidate.alreadyImported ? (
                        <button
                          type="button"
                          onClick={() => handleRemoveOpencode(candidate)}
                          disabled={!importedSkillId || removingOpencodeId !== '' || importingOpencodePath !== ''}
                          className="shrink-0 rounded-lg border border-slate-200 bg-white px-3 py-1.5 text-[12px] font-medium text-slate-600 hover:bg-slate-50 disabled:cursor-not-allowed disabled:opacity-60"
                        >
                          {isRemoving ? '取消中...' : '取消导入'}
                        </button>
                      ) : (
                        <button
                          type="button"
                          onClick={() => handleImportOpencode(candidate)}
                          disabled={importingOpencodePath !== '' || removingOpencodeId !== ''}
                          className="shrink-0 rounded-lg bg-indigo-600 px-3 py-1.5 text-[12px] font-medium text-white hover:bg-indigo-700 disabled:cursor-not-allowed disabled:opacity-60"
                        >
                          {importingOpencodePath === candidate.sourcePath ? '导入中...' : '导入'}
                        </button>
                      )}
                    </div>
                  </div>
                );
              })}
            </div>
          )}
        </div>

        <div className="mt-5 bg-white border border-slate-200 rounded-xl p-4 shadow-sm">
          <div className="mb-4 space-y-3">
            <div className="min-w-0">
              <div className="text-[14.5px] font-medium text-slate-800">自定义 Skill</div>
              <p className="mt-1 text-[12.5px] leading-relaxed text-slate-500">选择包含 SKILL.md 的文件夹后，可按应用范围启用。</p>
            </div>
            <button
              type="button"
              onClick={handleSkillDirectorySelected}
              disabled={isImportingSkill || isPickingDirectory}
              className="inline-flex w-full items-center justify-center gap-2 rounded-lg border border-indigo-200 bg-indigo-50 px-3 py-2 text-[12px] font-medium text-indigo-700 hover:bg-indigo-100 disabled:cursor-not-allowed disabled:opacity-60 sm:w-auto"
            >
              <Upload size={14} /> {isPickingDirectory ? '选择中...' : '选择skill文件夹'}
            </button>
          </div>

          {skillPreview && (
            <div className="mb-3 rounded-lg border border-indigo-100 bg-indigo-50 px-3 py-3 text-[12.5px] text-slate-600">
              <div className="font-semibold text-slate-800">预览：{skillPreview.name}</div>
              <div className="mt-1">{skillPreview.description}</div>
              {skillPreview.duplicate && (
                <div className="mt-2 rounded-md border border-amber-200 bg-amber-50 px-2.5 py-2 text-amber-700">
                  {skillPreview.duplicate.kind === 'contentHash' ? '相同内容的 Skill 已存在。' : '同名 Skill 已存在。'}可取消或覆盖元数据。
                </div>
              )}
              <div className="mt-3 rounded-md border border-indigo-100 bg-white/70 px-3 py-2">
                <div className="mb-2 text-[12px] font-medium text-slate-700">复用到以下角色</div>
                <label className="flex items-center gap-2 text-[12.5px] text-slate-600">
                  <input
                    type="checkbox"
                    checked={reuseAllRoles}
                    onChange={e => {
                      setReuseAllRoles(e.target.checked);
                      setReuseRoleIds([BUTLER_SCOPE_ID]);
                    }}
                    className="h-3.5 w-3.5 rounded border-slate-300 text-indigo-600 focus:ring-indigo-500"
                  />
                  全部角色
                </label>
                {!reuseAllRoles && (
                  <div className="mt-2 space-y-1.5">
                    <label className="flex items-center gap-2 text-[12.5px] text-slate-600">
                      <input
                        type="checkbox"
                        checked={reuseRoleIds.includes(BUTLER_SCOPE_ID)}
                        onChange={e => {
                          setReuseRoleIds(prev => e.target.checked
                            ? [...prev.filter(id => id !== BUTLER_SCOPE_ID), BUTLER_SCOPE_ID]
                            : prev.filter(id => id !== BUTLER_SCOPE_ID));
                        }}
                        className="h-3.5 w-3.5 rounded border-slate-300 text-indigo-600 focus:ring-indigo-500"
                      />
                      管家
                    </label>
                    {activeRoles.map(item => (
                      <label key={item.id} className="flex items-center gap-2 text-[12.5px] text-slate-600">
                        <input
                          type="checkbox"
                          checked={reuseRoleIds.includes(item.id)}
                          onChange={e => {
                            setReuseRoleIds(prev => e.target.checked
                              ? [...prev.filter(id => id !== item.id), item.id]
                              : prev.filter(id => id !== item.id));
                          }}
                          className="h-3.5 w-3.5 rounded border-slate-300 text-indigo-600 focus:ring-indigo-500"
                        />
                        {item.name}
                      </label>
                    ))}
                  </div>
                )}
              </div>
              <div className="mt-3 flex justify-end gap-2">
                <button type="button" onClick={() => { setSkillPreview(null); setPendingSkillContent(''); }} className="rounded-lg border border-slate-200 bg-white px-3 py-1.5 text-[12px] font-medium text-slate-600 hover:bg-slate-50">取消</button>
                <button type="button" onClick={() => handleConfirmImport(false)} disabled={isImportingSkill} className="rounded-lg bg-indigo-600 px-3 py-1.5 text-[12px] font-medium text-white hover:bg-indigo-700 disabled:cursor-not-allowed disabled:opacity-60">确认导入</button>
                {skillPreview.duplicate && (
                  <button type="button" onClick={() => handleConfirmImport(true)} disabled={isImportingSkill} className="rounded-lg bg-amber-600 px-3 py-1.5 text-[12px] font-medium text-white hover:bg-amber-700 disabled:cursor-not-allowed disabled:opacity-60">覆盖元数据</button>
                )}
              </div>
            </div>
          )}

          <div className="space-y-2">
            {isLoadingSkills && <div className="text-[12.5px] text-slate-400">正在加载自定义 Skill...</div>}
            {!isLoadingSkills && registrySkills.length === 0 && <div className="text-[12.5px] text-slate-400">暂无自定义 Skill。</div>}
            {registrySkills.map(item => {
              const enabled = skills.enabledSkillIds.includes(item.id);
              return (
                <div key={item.id} data-testid={`custom-skill-card-${item.id}`} className="rounded-lg border border-slate-100 bg-slate-50 px-3 py-3">
                  <div className="flex items-start justify-between gap-4">
                    <div className="min-w-0 flex-1">
                      <div className="flex min-w-0 flex-wrap items-center gap-2">
                        <span className={cn('w-2.5 h-2.5 shrink-0 rounded-full', enabled ? 'bg-emerald-500' : 'bg-slate-300')} />
                        <span className="min-w-0 break-words text-[13.5px] font-medium text-slate-800">{item.name}</span>
                        <span className="shrink-0 text-[11px] font-medium text-slate-500 bg-white px-2 py-0.5 rounded-full border border-slate-200">自定义</span>
                      </div>
                    </div>
                    <div data-testid={`custom-skill-actions-${item.id}`} className="flex shrink-0 items-center gap-2">
                      <button
                        type="button"
                        aria-label={`删除 ${item.name}`}
                        disabled={pendingSkill !== null}
                        onClick={() => setDeleteSkillTarget(item)}
                        className="inline-flex h-8 w-8 items-center justify-center rounded-lg border border-red-100 bg-red-50 text-red-500 hover:bg-red-100 disabled:cursor-not-allowed disabled:opacity-60"
                      >
                        <Trash2 size={14} />
                      </button>
                      <button
                        type="button"
                        role="switch"
                        aria-checked={enabled}
                        disabled={pendingSkill !== null}
                        onClick={() => handleCustomSkillToggle(item.id)}
                        className={cn(
                          'relative h-6 w-11 shrink-0 rounded-full transition-colors disabled:cursor-not-allowed disabled:opacity-60',
                          enabled ? 'bg-indigo-600' : 'bg-slate-300',
                        )}
                      >
                        <span className={cn('absolute left-0.5 top-0.5 h-5 w-5 rounded-full bg-white shadow transition-transform', enabled ? 'translate-x-5' : 'translate-x-0')} />
                        <span className="sr-only">{item.name}</span>
                      </button>
                    </div>
                  </div>
                  <div className="group relative mt-1.5">
                    <p data-testid={`custom-skill-description-${item.id}`} className="line-clamp-2 break-words text-[12px] leading-relaxed text-slate-500">{item.description}</p>
                    <div data-testid={`custom-skill-tooltip-${item.id}`} className="pointer-events-none absolute left-0 top-full z-20 mt-1 hidden max-w-[min(28rem,calc(100vw-3rem))] rounded-lg border border-slate-200 bg-white px-3 py-2 text-[12px] leading-relaxed text-slate-600 shadow-xl group-hover:block">
                      {item.description}
                    </div>
                  </div>
                </div>
              );
            })}
          </div>
        </div>
          </>
        )}
        {settingsSavedMessage && (
          <div className="mt-3 rounded-lg border border-emerald-100 bg-emerald-50 px-3 py-2 text-[13px] text-emerald-700 flex items-center gap-2">
            <Check size={14} /> {settingsSavedMessage}
          </div>
        )}
        {error && (
          <div className="mt-3 rounded-lg border border-red-100 bg-red-50 px-3 py-2 text-[13px] text-red-600 flex items-center gap-2">
            <AlertCircle size={14} /> {error}
          </div>
        )}
      </div>
      <div className="pt-6 border-t border-slate-200/80">
        <label className="text-[14px] font-semibold text-slate-800 block mb-3">晨间简报时间</label>
        <input
          type="time"
          value={briefingTime}
          onChange={async (e) => {
            const value = e.target.value;
            setBriefingTime(value);
            setSettingsSavedMessage('');
            setError('');
            try {
              await scheduleService.updateSchedule({ briefingTime: value });
              setSettingsSavedMessage('晨间简报时间已保存');
              setTimeout(() => setSettingsSavedMessage(''), MESSAGE_TIMEOUT_MS);
            } catch {
              setError('晨间简报时间保存失败，请稍后重试');
            }
          }}
          className="bg-white border border-slate-200 rounded-lg px-4 py-2.5 text-[14px] focus:ring-2 focus:ring-indigo-500/20 focus:border-indigo-500 outline-none"
        />
        <p className="text-[12px] text-slate-400 mt-2 leading-relaxed">每天推送晨间简报的时间。</p>
      </div>
      <div className="pt-6 border-t border-slate-200/80">
        <label className="text-[14px] font-semibold text-slate-800 block mb-3">周复盘时间</label>
        <div className="flex gap-3">
          <select value={reviewDay} onChange={async (e) => {
            const value = e.target.value;
            setReviewDay(value);
            setSettingsSavedMessage('');
            setError('');
            try {
              await scheduleService.updateSchedule({ reviewDay: value });
              setSettingsSavedMessage('周复盘时间已保存');
              setTimeout(() => setSettingsSavedMessage(''), MESSAGE_TIMEOUT_MS);
            } catch {
              setError('周复盘时间保存失败，请稍后重试');
            }
          }} className="bg-white border border-slate-200 rounded-lg px-3 py-2.5 text-[14px] focus:ring-2 focus:ring-indigo-500/20 outline-none">
            <option value="1">周一</option><option value="2">周二</option><option value="3">周三</option>
            <option value="4">周四</option><option value="5">周五</option><option value="6">周六</option>
            <option value="7">周日</option>
          </select>
          <input type="time" value={reviewTime} onChange={async (e) => {
            const value = e.target.value;
            setReviewTime(value);
            setSettingsSavedMessage('');
            setError('');
            try {
              await scheduleService.updateSchedule({ reviewTime: value });
              setSettingsSavedMessage('周复盘时间已保存');
              setTimeout(() => setSettingsSavedMessage(''), MESSAGE_TIMEOUT_MS);
            } catch {
              setError('周复盘时间保存失败，请稍后重试');
            }
          }} className="bg-white border border-slate-200 rounded-lg px-4 py-2.5 text-[14px] focus:ring-2 focus:ring-indigo-500/20 focus:border-indigo-500 outline-none" />
        </div>
        <p className="text-[12px] text-slate-400 mt-2 leading-relaxed">每周触发周复盘的时间。</p>
      </div>
      <div className="pt-6 border-t border-slate-200/80">
        <label className="text-[14px] font-semibold text-slate-800 block mb-3">大石头规划提醒时间</label>
        <div className="flex gap-3">
          <select value={bigrockReminderDay} onChange={async (e) => {
            const value = e.target.value;
            setBigrockReminderDay(value);
            setSettingsSavedMessage('');
            setError('');
            try {
              await scheduleService.updateSchedule({ bigrockReminderDay: value });
              setSettingsSavedMessage('大石头规划提醒时间已保存');
              setTimeout(() => setSettingsSavedMessage(''), MESSAGE_TIMEOUT_MS);
            } catch {
              setError('大石头规划提醒时间保存失败，请稍后重试');
            }
          }} className="bg-white border border-slate-200 rounded-lg px-3 py-2.5 text-[14px] focus:ring-2 focus:ring-indigo-500/20 outline-none">
            <option value="1">周一</option>
            <option value="2">周二</option>
          </select>
          <input type="time" value={bigrockReminderTime} onChange={async (e) => {
            const value = e.target.value;
            setBigrockReminderTime(value);
            setSettingsSavedMessage('');
            setError('');
            try {
              await scheduleService.updateSchedule({ bigrockReminderTime: value });
              setSettingsSavedMessage('大石头规划提醒时间已保存');
              setTimeout(() => setSettingsSavedMessage(''), MESSAGE_TIMEOUT_MS);
            } catch {
              setError('大石头规划提醒时间保存失败，请稍后重试');
            }
          }} className="bg-white border border-slate-200 rounded-lg px-4 py-2.5 text-[14px] focus:ring-2 focus:ring-indigo-500/20 focus:border-indigo-500 outline-none" />
        </div>
        <p className="text-[12px] text-slate-400 mt-2 leading-relaxed">每周初提醒规划本周大石头的时间。</p>
      </div>

      {archivedRoles.length > 0 && (
        <div className="pt-6 border-t border-slate-200/80">
          <label className="text-[14px] font-semibold text-slate-800 block mb-3">已归档角色</label>
          <div className="space-y-2.5">
            {archivedRoles.map((role: any) => {
              const Icon = getRoleIconComponent(role.icon);
              const roleColor = normalizeColorHex(role.color);
              return (
              <div key={role.id} className="bg-white border border-slate-200 rounded-xl p-4 flex items-center justify-between shadow-sm">
                <div className="flex items-center gap-3">
                  <div className="w-9 h-9 rounded-lg flex items-center justify-center text-white opacity-60" style={{ backgroundColor: roleColor }}>
                    <Icon size={18} />
                  </div>
                  <div>
                    <p className="text-[14px] font-medium text-slate-700">{role.name}</p>
                    <p className="text-[12px] text-slate-400">已归档</p>
                  </div>
                </div>
                <button onClick={() => onRestoreRole?.(role.id)} className="px-4 py-2 rounded-lg text-[13px] font-medium text-indigo-600 hover:bg-indigo-50 border border-indigo-200 transition-colors">
                  重新启用
                </button>
              </div>
              );
            })}
          </div>
        </div>
      )}

      {deleteSkillTarget && (
        <div className="fixed inset-0 z-[220] flex items-center justify-center bg-slate-900/20 backdrop-blur-sm">
          <div role="dialog" aria-modal="true" aria-label="删除自定义 Skill" className="w-[380px] rounded-2xl bg-white p-6 shadow-2xl">
            <h3 className="mb-3 text-[16px] font-semibold text-slate-800">删除自定义 Skill</h3>
            <p className="mb-5 text-[14px] leading-relaxed text-slate-600">
              确认删除「{deleteSkillTarget.name}」吗？
            </p>
            <div className="flex justify-end gap-3">
              <button type="button" onClick={() => setDeleteSkillTarget(null)} disabled={pendingSkill !== null} className="rounded-lg border border-slate-200 px-5 py-2.5 text-[13px] font-medium text-slate-600 hover:bg-slate-50 disabled:cursor-not-allowed disabled:opacity-60">取消</button>
              <button
                type="button"
                onClick={handleCustomSkillDelete}
                disabled={pendingSkill !== null}
                className="rounded-lg bg-red-600 px-5 py-2.5 text-[13px] font-medium text-white hover:bg-red-700 disabled:cursor-not-allowed disabled:opacity-60"
              >
                {pendingSkill === deleteSkillTarget.id ? '删除中...' : '确认删除'}
              </button>
            </div>
          </div>
        </div>
      )}

      {isMissionModalOpen && (
        <div className="fixed inset-0 z-50 flex items-center justify-center bg-black/40" onClick={() => setIsMissionModalOpen(false)}>
          <div
            role="dialog"
            aria-modal="true"
            aria-label="编辑使命宣言"
            onClick={e => e.stopPropagation()}
            className="w-[640px] max-h-[85vh] overflow-y-auto rounded-2xl bg-white p-6 shadow-2xl"
          >
            <div className="flex items-center justify-between mb-4">
              <h3 className="text-[16px] font-semibold text-slate-800">编辑使命宣言</h3>
              <button type="button" onClick={() => setIsMissionModalOpen(false)} className="text-slate-400 hover:text-slate-600 transition-colors">
                <X size={20} />
              </button>
            </div>

            <div className="space-y-4">
              <div>
                <label className="text-[13px] font-medium text-slate-600 block mb-1.5">使命</label>
                <textarea
                  value={structuredMission}
                  onChange={e => setStructuredMission(e.target.value)}
                  placeholder="一句话概括你的人生使命。例如：堂堂正正地生活，并且对他人有所影响，对社会有所贡献。"
                  rows={2}
                  className="w-full bg-white border border-slate-200 rounded-lg px-4 py-2.5 text-[14px] focus:ring-2 focus:ring-indigo-500/20 focus:border-indigo-500 outline-none resize-none placeholder:text-slate-300"
                />
              </div>
              <div>
                <label className="text-[13px] font-medium text-slate-600 block mb-1.5">原则</label>
                <textarea
                  value={structuredPrinciple}
                  onChange={e => setStructuredPrinciple(e.target.value)}
                  placeholder={'你信奉的核心原则和行为准则。例如：\n有慈悲心——亲近人群，不分贵贱\n甘愿牺牲——为人生使命奉献时间、才智和金钱\n诚信——在诚信问题上决不妥协'}
                  rows={5}
                  className="w-full bg-white border border-slate-200 rounded-lg px-4 py-2.5 text-[14px] focus:ring-2 focus:ring-indigo-500/20 focus:border-indigo-500 outline-none resize-none placeholder:text-slate-300"
                />
              </div>
              <div>
                <div className="flex items-center justify-between mb-1.5">
                  <label className="text-[13px] font-medium text-slate-600">角色与目标</label>
                  <span className="text-[11px] text-slate-400">{structuredRoles.length}/7</span>
                </div>
                <div className="space-y-2">
                  {structuredRoles.map((r, i) => (
                    <div key={i} className="flex items-start gap-2">
                      <input
                        type="text"
                        value={r.name}
                        onChange={e => {
                          const next = [...structuredRoles];
                          next[i] = { ...next[i], name: e.target.value };
                          setStructuredRoles(next);
                        }}
                        placeholder="角色名称"
                        className="w-28 shrink-0 bg-white border border-slate-200 rounded-lg px-3 py-2 text-[13px] focus:ring-2 focus:ring-indigo-500/20 focus:border-indigo-500 outline-none placeholder:text-slate-300"
                      />
                      <textarea
                        value={r.goal}
                        onChange={e => {
                          const next = [...structuredRoles];
                          next[i] = { ...next[i], goal: e.target.value };
                          setStructuredRoles(next);
                        }}
                        placeholder="该角色的目标与承诺"
                        rows={2}
                        className="flex-1 bg-white border border-slate-200 rounded-lg px-3 py-2 text-[13px] focus:ring-2 focus:ring-indigo-500/20 focus:border-indigo-500 outline-none resize-none placeholder:text-slate-300"
                      />
                      <button
                        type="button"
                        onClick={() => setStructuredRoles(structuredRoles.filter((_, idx) => idx !== i))}
                        className="shrink-0 mt-1.5 text-slate-400 hover:text-red-500 transition-colors"
                      >
                        <Trash2 size={16} />
                      </button>
                    </div>
                  ))}
                  {structuredRoles.length < 7 && (
                    <button
                      type="button"
                      onClick={() => setStructuredRoles([...structuredRoles, { name: '', goal: '' }])}
                      className="inline-flex items-center gap-1.5 text-[12px] text-indigo-600 hover:text-indigo-700 transition-colors"
                    >
                      <Plus size={14} /> 添加角色
                    </button>
                  )}
                </div>
              </div>

              <div className="pt-2 border-t border-slate-100">
                <p className="text-[12px] text-slate-400 mb-1.5">柯维四段式示例（点击填入）：</p>
                <button
                  type="button"
                  onClick={() => {
                    setStructuredMission(structuredTemplate.mission);
                    setStructuredPrinciple(structuredTemplate.principle);
                    setStructuredRoles(structuredTemplate.roles.map(r => ({ ...r })));
                  }}
                  className="block w-full text-left px-3 py-2 rounded-lg border border-slate-200 text-[13px] text-slate-600 hover:bg-indigo-50 hover:border-indigo-200 hover:text-indigo-700 transition-colors"
                >
                  柯维四段式示例：使命 → 原则 → 角色与目标
                </button>
              </div>
            </div>

            <div className="mt-5 flex items-center justify-end gap-3">
              <button
                type="button"
                onClick={() => setIsMissionModalOpen(false)}
                className="rounded-lg border border-slate-200 px-5 py-2.5 text-[13px] font-medium text-slate-600 hover:bg-slate-50"
              >
                取消
              </button>
              <button
                type="button"
                onClick={handleSaveMission}
                disabled={isSavingMission}
                className="inline-flex items-center gap-2 rounded-lg bg-indigo-600 px-5 py-2.5 text-[13px] font-medium text-white hover:bg-indigo-700 disabled:cursor-not-allowed disabled:opacity-60"
              >
                {isSavingMission && <Loader2 size={14} className="animate-spin" />}
                {isSavingMission ? '保存中...' : '保存'}
              </button>
            </div>
          </div>
        </div>
      )}

      {isInferenceModalOpen && inferredValues && (
        <div className="fixed inset-0 z-50 flex items-center justify-center bg-black/40" onClick={() => setIsInferenceModalOpen(false)}>
          <div
            role="dialog"
            aria-modal="true"
            aria-label="推断的使命宣言"
            onClick={e => e.stopPropagation()}
            className="w-[480px] rounded-2xl bg-white p-6 shadow-2xl"
          >
            <div className="flex items-center justify-between mb-4">
              <h3 className="text-[16px] font-semibold text-slate-800">基于行为推断的使命宣言</h3>
              <button type="button" onClick={() => setIsInferenceModalOpen(false)} className="text-slate-400 hover:text-slate-600 transition-colors">
                <X size={20} />
              </button>
            </div>

            <div className="space-y-3">
              <div>
                <div className="text-[13px] font-medium text-slate-600 mb-1.5">推断的优先级</div>
                <div className="space-y-1.5">
                  {inferredValues.values.map((v, i) => (
                    <div key={i} className="text-[13px] text-slate-700 leading-relaxed">
                      {v}
                    </div>
                  ))}
                </div>
              </div>
              <div>
                <div className="text-[13px] font-medium text-slate-600 mb-1">推断的使命宣言</div>
                <textarea
                  value={editableSummary}
                  onChange={e => setEditableSummary(e.target.value)}
                  rows={4}
                  className="w-full bg-white border border-slate-200 rounded-lg px-3 py-2 text-[14px] text-slate-800 leading-relaxed focus:ring-2 focus:ring-indigo-500/20 focus:border-indigo-500 outline-none resize-none"
                />
                {inferredValues.confidence < 0.7 && (
                  <p className="text-[12px] text-slate-400 mt-1.5">（置信度较低，仅供参考）</p>
                )}
              </div>
            </div>

            <div className="mt-5 flex items-center justify-end gap-3">
              <button
                type="button"
                onClick={handleDismissInferred}
                disabled={isSavingMission}
                className="rounded-lg border border-slate-200 px-5 py-2.5 text-[13px] font-medium text-slate-600 hover:bg-slate-50 disabled:cursor-not-allowed disabled:opacity-60"
              >
                不准确
              </button>
              <button
                type="button"
                onClick={handleAdoptInferred}
                disabled={isSavingMission}
                className="inline-flex items-center gap-2 rounded-lg bg-indigo-600 px-5 py-2.5 text-[13px] font-medium text-white hover:bg-indigo-700 disabled:cursor-not-allowed disabled:opacity-60"
              >
                {isSavingMission && <Loader2 size={14} className="animate-spin" />}
                {isSavingMission ? '保存中...' : '确认采纳'}
              </button>
            </div>
          </div>
        </div>
      )}
    </div>
  );
}

function toFriendlyError(error: unknown, fallback: string) {
  if (error == null) return fallback;
  const text = typeof error === 'string' ? error : (JSON.stringify(error) ?? String(error));
  if (text.includes('frontmatter') || text.includes('SKILL.md')) {
    return 'SKILL.md 解析失败，请检查 frontmatter 中的 name 和 description';
  }
  return fallback;
}
