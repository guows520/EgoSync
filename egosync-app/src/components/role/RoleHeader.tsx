import { useState, useRef, useEffect } from 'react';
import { ListTodo, BrainCircuit, Sliders, MoreHorizontal, Archive, Trash2, X } from 'lucide-react';
import { cn } from '../../lib/utils';
import { getRoleIconComponent, normalizeColorHex } from '../../lib/roleIcons';
import { Modal } from '../layout/Modal';
import type { Role } from '../../types/role';

export type RoleViewTab = 'tasks' | 'memory' | 'settings' | null;

interface RoleHeaderProps {
  role: Role;
  openTab: RoleViewTab;
  onToggleTab: (tab: Exclude<RoleViewTab, null>) => void;
  /** Story 16.4：移动端「⋯」菜单——切换角色（回角色列表根） */
  onSwitchRole?: () => void;
  /** Story 16.4：移动端「⋯」菜单——新建角色直达 */
  onAddRole?: () => void;
  /** Story 16.4 D2 收口（2026-09-25 人类指令）：移动端归档/删除入口——桌面
   *  侧栏右键菜单在 <768px 随侧栏退场消失，能力对等缺口由本组补齐 */
  onArchiveRole?: (id: string) => Promise<void> | void;
  onDeleteRole?: (id: string) => Promise<void> | void;
}

/**
 * 角色视图头部 — 从 RoleView 抽取出来的独立组件 (Story 2.2 AC-5)。
 *
 * 设计原则：
 * - icon/color 一律走 `lib/roleIcons.ts` 白名单，与 Sidebar 共用一套真相
 *   （Story 1.9/2.1 经验：mock 字段 `role.text`/Tailwind class 已废弃，不再兼容）
 * - active tab 的着色用 inline style 注入 `role.color`，不引入 `role.text` 这种
 *   只在 mock 模式下有效的 Tailwind class
 */
export function RoleHeader({ role, openTab, onToggleTab, onSwitchRole, onAddRole, onArchiveRole, onDeleteRole }: RoleHeaderProps) {
  const RoleIcon = getRoleIconComponent(role.icon);
  const roleColor = normalizeColorHex(role.color);
  // Story 16.4：移动端「⋯」菜单状态（切换角色/新建角色直达——线框屏 2b）
  const [showMoreMenu, setShowMoreMenu] = useState(false);
  const moreMenuRef = useRef<HTMLDivElement>(null);
  // Story 16.4 D2 收口：归档/删除确认态——语义照搬桌面 Sidebar ConfirmDialog
  // （删除需输入角色名；错误文案含「至少保留一个角色」归一化）
  const [confirmAction, setConfirmAction] = useState<'archive' | 'delete' | null>(null);
  const [deleteConfirmName, setDeleteConfirmName] = useState('');
  const [isConfirming, setIsConfirming] = useState(false);
  const [confirmError, setConfirmError] = useState('');

  useEffect(() => {
    if (!showMoreMenu) return;
    const handleClick = (e: MouseEvent) => {
      if (moreMenuRef.current && !moreMenuRef.current.contains(e.target as Node)) {
        setShowMoreMenu(false);
      }
    };
    // 键盘退出（Escape）——与 ButlerSettingsContent/GlobalSettingsModal
    // 的 Escape 范式一致；role=menu 必须可键盘关闭（a11y）
    const handleKeyDown = (e: KeyboardEvent) => {
      if (e.key === 'Escape') {
        e.stopPropagation();
        setShowMoreMenu(false);
      }
    };
    document.addEventListener('mousedown', handleClick);
    document.addEventListener('keydown', handleKeyDown);
    return () => {
      document.removeEventListener('mousedown', handleClick);
      document.removeEventListener('keydown', handleKeyDown);
    };
  }, [showMoreMenu]);

  // D2 收口：确认执行（与桌面 Sidebar.handleConfirm 同语义：删除校验输入名、
  // 「至少保留一个角色」错误归一化；成功后由 App 处理器负责刷新与视图回退）
  const handleConfirm = async () => {
    if (!confirmAction) return;
    if (confirmAction === 'delete' && deleteConfirmName.trim() !== role.name) return;
    setIsConfirming(true);
    setConfirmError('');
    try {
      if (confirmAction === 'archive') await onArchiveRole?.(role.id);
      else await onDeleteRole?.(role.id);
      setConfirmAction(null);
      setDeleteConfirmName('');
    } catch (e: any) {
      const msg = typeof e === 'string' ? e : (e?.message || JSON.stringify(e));
      setConfirmError(msg.includes('至少保留一个角色') ? '至少保留一个角色' : '操作失败，请稍后重试');
    } finally {
      setIsConfirming(false);
    }
  };

  const tabButton = (tab: Exclude<RoleViewTab, null>, Icon: typeof ListTodo, label: string) => {
    const isActive = openTab === tab;
    return (
      <button
        onClick={() => { setShowMoreMenu(false); onToggleTab(tab); }}
        className={cn(
          // Story 16.4 评审修复：小屏触控 ≥44px（AC2 红线——px-2.5 py-2 仅
          // 约 36px）；桌面零变化
          'flex items-center gap-1.5 px-2.5 md:px-3 py-2 max-md:min-h-[44px] rounded-lg text-[13px] font-medium transition-all',
          isActive ? 'bg-white dark:bg-slate-700 shadow-sm' : 'text-slate-500 dark:text-slate-400 hover:bg-white/50 dark:hover:bg-slate-700/60',
        )}
        style={isActive ? { color: roleColor } : undefined}
      >
        <Icon size={16} /> {label}
      </button>
    );
  };

  return (
    // Story 16.2 响应式基线：小屏 px-4 收缩边距、目标文案与能量条收缩、
    // tab 按钮收窄（375px 不溢出；断点方案归 UX 勘注定稿）；≥md 维持桌面原样。
    // Story 16.4：relative 供移动端「⋯」菜单绝对定位（桌面零影响）；
    // max-md:z-20——backdrop-blur 令 header 成为层叠上下文（z-auto 且
    // 先于聊天区挂载 ⇒ 下拉菜单被后挂载的聊天区盖住，WebDriver 实测
    // click intercepted）。小屏抬层即可，桌面无重叠面零影响。
    <header className="relative max-md:z-20 h-auto md:h-[76px] py-2 md:py-0 border-b border-slate-200/60 dark:border-slate-700/60 bg-white/60 dark:bg-slate-800/60 backdrop-blur-md flex flex-wrap items-center px-4 md:px-8 gap-2 md:gap-4 shrink-0 shadow-[0_1px_2px_rgba(0,0,0,0.02)]">
      <div
        className="w-[46px] h-[46px] rounded-xl flex items-center justify-center text-white shadow-sm shrink-0"
        style={{ backgroundColor: roleColor }}
      >
        <RoleIcon size={24} strokeWidth={2} />
      </div>
      <div className="min-w-0 max-w-full">
        <h2 className="font-semibold text-lg leading-tight text-slate-800 dark:text-slate-100 truncate">
          {role.name}
        </h2>
        <div className="flex items-center gap-2 md:gap-3 mt-1.5">
          {role.goal.trim() && (
            <span className="hidden min-[480px]:inline max-w-[140px] md:max-w-[260px] truncate text-[12px] text-slate-500 dark:text-slate-400 font-medium">
              {role.goal}
            </span>
          )}
          <div className="w-[70px] min-[480px]:w-[120px] h-[6px] bg-slate-200/80 dark:bg-slate-700 rounded-full overflow-hidden shrink-0">
            <div
              className="h-full bg-emerald-500 rounded-full transition-all duration-1000"
              style={{ width: `${role.energy}%` }}
            />
          </div>
          <span className="text-[11px] text-slate-500 dark:text-slate-400 font-medium shrink-0">{role.energy}% 能量</span>
        </div>
      </div>

      <div className="ml-auto relative flex items-center gap-1.5 md:gap-2" ref={moreMenuRef}>
        {tabButton('tasks', ListTodo, '任务')}
        {tabButton('memory', BrainCircuit, '记忆')}
        {tabButton('settings', Sliders, '设置')}
        {/* 移动端去重（2026-09-26 人类指令，spec-web-mobile-tab-dedup）：面板
            自带 tab 条小屏退场后，关闭入口上移头部——仅 <768px 显（md:hidden）、
            仅 tab 打开时显；再点当前 tab 即关闭（既有 toggle 语义），不新增
            props；触控 44×44。桌面无此按钮、零变化。
            点击先收「⋯」菜单：菜单挂在本簇内（moreMenuRef），外部点击关闭逻辑
            对簇内点击不生效——不先收菜单会出现「tab 关了菜单还悬浮」的断裂。 */}
        {openTab && (
          <button
            onClick={() => { setShowMoreMenu(false); onToggleTab(openTab); }}
            aria-label="关闭"
            data-testid="role-close-tab"
            className="md:hidden w-11 h-11 flex items-center justify-center rounded-lg text-slate-400 dark:text-slate-500 hover:bg-white/50 dark:hover:bg-slate-700/60 transition-colors"
          >
            <X size={20} />
          </button>
        )}
        {/* Story 16.4：「⋯」= 切换角色/新建角色直达（线框屏 2b）——md:hidden
            桌面无此控件；触控 ≥44×44。2026-09-26 人类指令：整块移到三个 tab 按钮之后（「设置」右边）。D2 收口（2026-09-25 人类指令）追加
            归档/删除两项——桌面侧栏右键菜单的移动对等入口。 */}
        {onSwitchRole && (
          <>
            <button
              onClick={() => setShowMoreMenu(v => !v)}
              aria-label="角色菜单"
              aria-expanded={showMoreMenu}
              data-testid="role-more-menu"
              className="md:hidden w-11 h-11 flex items-center justify-center rounded-lg text-slate-400 dark:text-slate-500 hover:bg-white/50 dark:hover:bg-slate-700/60 transition-colors"
            >
              <MoreHorizontal size={20} />
            </button>
            {showMoreMenu && (
              <div
                role="menu"
                aria-label="角色操作菜单"
                className="md:hidden absolute right-2 top-[calc(100%-2px)] z-30 w-36 rounded-xl border border-slate-200 dark:border-slate-700 bg-white dark:bg-slate-800 shadow-lg py-1 animate-in fade-in zoom-in-95 duration-150"
              >
                <button
                  role="menuitem"
                  onClick={() => { setShowMoreMenu(false); onSwitchRole(); }}
                  data-testid="role-menu-switch"
                  className="w-full text-left px-4 py-2.5 max-md:min-h-[44px] max-md:flex max-md:items-center text-[13px] font-medium text-slate-700 dark:text-slate-200 hover:bg-slate-50 dark:hover:bg-slate-700/60"
                >
                  切换角色
                </button>
                <button
                  role="menuitem"
                  onClick={() => { setShowMoreMenu(false); onAddRole?.(); }}
                  data-testid="role-menu-add"
                  className="w-full text-left px-4 py-2.5 max-md:min-h-[44px] max-md:flex max-md:items-center text-[13px] font-medium text-slate-700 dark:text-slate-200 hover:bg-slate-50 dark:hover:bg-slate-700/60"
                >
                  新建角色
                </button>
                {/* D2 收口：归档/删除——桌面右键菜单的移动对等入口（仅 <768px
                    菜单内可见；删除为破坏性操作，确认弹窗要求输入角色名） */}
                {(onArchiveRole || onDeleteRole) && (
                  <>
                    <div className="my-1 border-t border-slate-100 dark:border-slate-700" role="separator" />
                    {onArchiveRole && (
                      <button
                        role="menuitem"
                        onClick={() => { setShowMoreMenu(false); setConfirmAction('archive'); }}
                        data-testid="role-menu-archive"
                        className="w-full text-left px-4 py-2.5 max-md:min-h-[44px] max-md:flex max-md:items-center gap-2 text-[13px] font-medium text-slate-700 dark:text-slate-200 hover:bg-slate-50 dark:hover:bg-slate-700/60"
                      >
                        <Archive size={14} /> 归档
                      </button>
                    )}
                    {onDeleteRole && (
                      <button
                        role="menuitem"
                        onClick={() => { setShowMoreMenu(false); setConfirmAction('delete'); }}
                        data-testid="role-menu-delete"
                        className="w-full text-left px-4 py-2.5 max-md:min-h-[44px] max-md:flex max-md:items-center gap-2 text-[13px] font-medium text-red-600 dark:text-red-400 hover:bg-red-50 dark:hover:bg-red-900/20"
                      >
                        <Trash2 size={14} /> 删除
                      </button>
                    )}
                  </>
                )}
              </div>
            )}
            {confirmAction && (
              <Modal
                ariaLabel={confirmAction === 'archive' ? '确认归档' : '确认删除'}
                onClose={() => { setConfirmAction(null); setDeleteConfirmName(''); setConfirmError(''); }}
                width="w-[360px]"
              >
                <div className="p-6">
                  <h3 className="text-[16px] font-semibold text-slate-800 dark:text-slate-100 mb-3">
                    {confirmAction === 'archive' ? '确认归档' : '确认删除'}
                  </h3>
                  <p className="text-[14px] text-slate-600 dark:text-slate-300 leading-relaxed mb-4">
                    {confirmAction === 'archive'
                      ? `归档「${role.name}」？归档后角色从列表消失，可在管家→设置中恢复。`
                      : `删除「${role.name}」？该角色及其对话、任务、记忆将一并删除，且不可恢复。`}
                  </p>
                  {confirmAction === 'delete' && (
                    <input
                      value={deleteConfirmName}
                      onChange={e => setDeleteConfirmName(e.target.value)}
                      placeholder={`请输入角色名「${role.name}」以确认`}
                      aria-label="输入角色名以确认删除"
                      data-testid="role-delete-confirm-input"
                      className="w-full mb-3 px-3 py-2.5 rounded-lg border border-slate-200 dark:border-slate-600 bg-white dark:bg-slate-900 text-[14px] text-slate-800 dark:text-slate-100 outline-none focus:border-indigo-400"
                    />
                  )}
                  {confirmError && (
                    <p className="mb-3 text-[13px] text-red-600 dark:text-red-400">{confirmError}</p>
                  )}
                  <div className="flex gap-2 max-md:gap-3">
                    <button
                      onClick={() => { setConfirmAction(null); setDeleteConfirmName(''); setConfirmError(''); }}
                      data-testid="role-confirm-cancel"
                      className="flex-1 min-h-[44px] rounded-lg text-[14px] font-medium text-slate-600 dark:text-slate-300 bg-slate-100 dark:bg-slate-700 hover:bg-slate-200 dark:hover:bg-slate-600 transition-colors"
                    >
                      取消
                    </button>
                    <button
                      onClick={handleConfirm}
                      disabled={isConfirming || (confirmAction === 'delete' && deleteConfirmName.trim() !== role.name)}
                      data-testid={confirmAction === 'archive' ? 'role-confirm-archive' : 'role-confirm-delete'}
                      className={cn(
                        'flex-1 min-h-[44px] rounded-lg text-[14px] font-medium transition-colors disabled:opacity-50',
                        confirmAction === 'archive'
                          ? 'text-white bg-indigo-600 hover:bg-indigo-700'
                          : 'text-white bg-red-600 hover:bg-red-700',
                      )}
                    >
                      {isConfirming ? '处理中…' : (confirmAction === 'archive' ? '确认归档' : '确认删除')}
                    </button>
                  </div>
                </div>
              </Modal>
            )}
          </>
        )}
      </div>
    </header>
  );
}