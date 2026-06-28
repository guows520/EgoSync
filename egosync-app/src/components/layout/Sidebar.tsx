import { useState, useRef } from 'react';
import { Home, Plus, Moon, Sun, Settings as SettingsIcon, Pencil, Archive, Trash2, Bell } from 'lucide-react';
import { cn } from '../../lib/utils';
import { RoleSidebarIcon } from './RoleSidebarIcon';

export function Sidebar({ roles, currentView, onViewChange, isSettingsOpen, onOpenSettings, onCloseSettings, onAddRole, onArchiveRole, onDeleteRole, theme, onToggleTheme, onEditRole, isNotifOpen, onToggleNotif, unreadCount = 0, whisperUnread = 0 }: any) {
  const handleNav = (view: string) => {
    onViewChange(view);
    onCloseSettings();
  };
  const [contextMenu, setContextMenu] = useState<{roleId: string, x: number, y: number} | null>(null);
  const [confirmAction, setConfirmAction] = useState<{type: 'archive' | 'delete', roleId: string, roleName: string} | null>(null);
  const [deleteConfirmName, setDeleteConfirmName] = useState('');
  const [confirmError, setConfirmError] = useState('');
  const [isConfirming, setIsConfirming] = useState(false);
  const roleListRef = useRef<HTMLDivElement>(null);

  const handleContextMenu = (e: React.MouseEvent, roleId: string) => {
    e.preventDefault();
    setContextMenu({ roleId, x: e.clientX, y: e.clientY });
  };

  const handleConfirm = async () => {
    if (!confirmAction) return;
    if (confirmAction.type === 'delete' && deleteConfirmName.trim() !== confirmAction.roleName) return;
    setIsConfirming(true);
    setConfirmError('');
    try {
      if (confirmAction.type === 'archive') await onArchiveRole(confirmAction.roleId);
      else await onDeleteRole(confirmAction.roleId);
      setConfirmAction(null);
      setDeleteConfirmName('');
    } catch (e: any) {
      const msg = typeof e === 'string' ? e : (e?.message || JSON.stringify(e));
      setConfirmError(msg.includes('至少保留一个角色') ? '至少保留一个角色' : '操作失败，请稍后重试');
    } finally {
      setIsConfirming(false);
    }
  };

  const handleRoleListKeyDown = (e: React.KeyboardEvent) => {
    if (e.key !== 'ArrowUp' && e.key !== 'ArrowDown') return;
    e.preventDefault();
    const container = roleListRef.current;
    if (!container) return;
    const buttons = Array.from(container.querySelectorAll<HTMLButtonElement>('[data-role-icon] button'));
    if (buttons.length === 0) return;
    const currentIndex = buttons.findIndex(btn => btn === document.activeElement);
    let nextIndex: number;
    if (e.key === 'ArrowDown') {
      nextIndex = currentIndex < buttons.length - 1 ? currentIndex + 1 : 0;
    } else {
      nextIndex = currentIndex > 0 ? currentIndex - 1 : buttons.length - 1;
    }
    buttons[nextIndex]?.focus();
  };

  return (
    <aside
      className="w-16 flex flex-col items-center pt-6 pb-6 z-10 shrink-0 relative transition-colors duration-300 bg-[#F1F3F5] dark:bg-slate-800"
      role="navigation"
      aria-label="角色导航"
    >
      <div className="flex-1 flex flex-col items-center gap-4 w-full min-h-0">
        <button
          onClick={() => handleNav('butler')}
          title="管家"
          className={cn("w-11 h-11 rounded-xl flex items-center justify-center transition-all duration-200 shrink-0", !isSettingsOpen && currentView === 'butler' ? "bg-indigo-600 text-white shadow-lg shadow-indigo-200 scale-105" : "text-slate-500 hover:bg-slate-200")}
        >
          <Home size={22} strokeWidth={2.5} />
        </button>
        <div className="w-6 border-b border-slate-300 my-2 shrink-0"></div>
        <div ref={roleListRef} onKeyDown={handleRoleListKeyDown} className="flex flex-col items-center gap-4 flex-[0_1_auto] overflow-y-auto min-h-0 w-full">
          {roles.map((role: any) => (
            <div key={role.id} data-role-icon>
              <RoleSidebarIcon
                role={role}
                isActive={!isSettingsOpen && currentView === role.id}
                onClick={() => handleNav(role.id)}
                onContextMenu={(e) => handleContextMenu(e, role.id)}
              />
            </div>
          ))}
        </div>
        <button
          onClick={onAddRole}
          title="添加角色"
          className="w-11 h-11 rounded-xl flex items-center justify-center text-slate-400 border-2 border-dashed border-slate-300 dark:border-slate-600 hover:border-indigo-400 hover:text-indigo-500 hover:bg-indigo-50/50 dark:hover:bg-indigo-900/30 transition-all duration-200 shrink-0"
        >
          <Plus size={20} strokeWidth={2.5} />
        </button>
      </div>

      <div className="flex flex-col items-center gap-3 mt-auto pt-4 pb-2">
        <button onClick={onToggleNotif} title="通知" aria-label={unreadCount > 0 ? '有新通知' : (whisperUnread > 0 ? '有耳语通知' : '通知')} className={cn("relative w-11 h-11 rounded-xl flex items-center justify-center transition-colors", isNotifOpen ? "bg-indigo-600 text-white shadow-lg shadow-indigo-200" : "text-slate-400 hover:bg-slate-200 dark:hover:bg-slate-700 hover:text-slate-700 dark:hover:text-slate-200")}>
          <Bell size={20} />
          {unreadCount > 0 ? (
            <span className="absolute top-1.5 right-1.5 w-2.5 h-2.5 bg-red-500 rounded-full border-2 border-[#F1F3F5] dark:border-slate-800"></span>
          ) : whisperUnread > 0 ? (
            <span className="absolute top-1.5 right-1.5 w-2.5 h-2.5 bg-emerald-500 rounded-full border-2 border-[#F1F3F5] dark:border-slate-800"></span>
          ) : null}
        </button>
        <button onClick={onToggleTheme} title={theme === 'light' ? '切换深色' : '切换浅色'} className="w-11 h-11 rounded-xl flex items-center justify-center text-slate-400 hover:bg-slate-200 dark:hover:bg-slate-700 hover:text-slate-700 dark:hover:text-slate-200 transition-colors">
          {theme === 'light' ? <Moon size={20} /> : <Sun size={20} />}
        </button>
        <button onClick={onOpenSettings} title="设置" className={cn("w-11 h-11 rounded-xl flex items-center justify-center transition-colors", isSettingsOpen ? "bg-indigo-600 text-white shadow-lg shadow-indigo-200" : "text-slate-400 hover:bg-slate-200 dark:hover:bg-slate-700 hover:text-slate-700 dark:hover:text-slate-200")}>
          <SettingsIcon size={22} />
        </button>
      </div>

      {/* Context Menu */}
      {contextMenu && (
        <>
          <div className="fixed inset-0 z-[100]" onClick={() => setContextMenu(null)} />
          <div className="fixed z-[101] bg-white dark:bg-slate-800 rounded-xl shadow-xl border border-slate-200 dark:border-slate-600 py-2 w-36 animate-in zoom-in-95 duration-150" style={{ left: contextMenu.x, top: contextMenu.y }}>
            <button onClick={() => { onEditRole(contextMenu.roleId); setContextMenu(null); }} className="w-full px-4 py-2 text-left text-[13px] text-slate-700 dark:text-slate-200 hover:bg-slate-50 dark:hover:bg-slate-700 flex items-center gap-2.5">
              <Pencil size={14} /> 编辑
            </button>
            <button onClick={() => { const r = roles.find((x: any) => x.id === contextMenu.roleId); setConfirmAction({type: 'archive', roleId: contextMenu.roleId, roleName: r?.name || ''}); setContextMenu(null); }} className="w-full px-4 py-2 text-left text-[13px] text-slate-700 dark:text-slate-200 hover:bg-slate-50 dark:hover:bg-slate-700 flex items-center gap-2.5">
              <Archive size={14} /> 归档
            </button>
            <button onClick={() => { const r = roles.find((x: any) => x.id === contextMenu.roleId); setConfirmAction({type: 'delete', roleId: contextMenu.roleId, roleName: r?.name || ''}); setContextMenu(null); }} className="w-full px-4 py-2 text-left text-[13px] text-red-600 dark:text-red-400 hover:bg-red-50 dark:hover:bg-red-900/20 flex items-center gap-2.5">
              <Trash2 size={14} /> 删除
            </button>
          </div>
        </>
      )}

      {/* Confirm Dialog */}
      {confirmAction && (
        <div className="fixed inset-0 z-[200] flex items-center justify-center bg-slate-900/20 backdrop-blur-sm animate-in fade-in duration-200">
          <div className="bg-white dark:bg-slate-800 rounded-2xl shadow-2xl p-6 w-[360px] animate-in zoom-in-95 duration-200">
            <h3 className="text-[16px] font-semibold text-slate-800 dark:text-slate-100 mb-3">
              {confirmAction.type === 'archive' ? '确认归档' : '确认删除'}
            </h3>
            <p className="text-[14px] text-slate-600 dark:text-slate-300 leading-relaxed mb-4">
              {confirmAction.type === 'archive' 
                ? `归档「${confirmAction.roleName}」后，该角色将暂停工作，但保留所有记忆和任务。你可以随时重新启用。` 
                : `删除「${confirmAction.roleName}」后，该角色的所有数据将被永久移除，且不可恢复。请输入角色名确认。`}
            </p>
            {confirmAction.type === 'delete' && (
              <input
                value={deleteConfirmName}
                onChange={e => setDeleteConfirmName(e.target.value)}
                placeholder={confirmAction.roleName}
                className="mb-4 w-full rounded-lg border border-slate-200 dark:border-slate-600 bg-white dark:bg-slate-700 px-4 py-2.5 text-[14px] text-slate-800 dark:text-slate-100 outline-none focus:border-red-400 focus:ring-2 focus:ring-red-500/10"
              />
            )}
            {confirmError && (
              <p className="mb-3 rounded-lg border border-red-100 bg-red-50 px-3 py-2 text-[13px] text-red-600">{confirmError}</p>
            )}
            <div className="flex justify-end gap-3">
              <button onClick={() => { setConfirmAction(null); setConfirmError(''); }} className="px-5 py-2.5 border border-slate-200 dark:border-slate-600 text-slate-600 dark:text-slate-300 rounded-lg text-[13px] font-medium hover:bg-slate-50 dark:hover:bg-slate-700">取消</button>
              <button onClick={handleConfirm} disabled={isConfirming || (confirmAction.type === 'delete' && deleteConfirmName.trim() !== confirmAction.roleName)} className={cn("px-5 py-2.5 rounded-lg text-[13px] font-medium text-white disabled:cursor-not-allowed disabled:opacity-50", confirmAction.type === 'archive' ? "bg-amber-600 hover:bg-amber-700" : "bg-red-600 hover:bg-red-700")}>
                {isConfirming ? '处理中...' : confirmAction.type === 'archive' ? '确认归档' : '确认删除'}
              </button>
            </div>
          </div>
        </div>
      )}
    </aside>
  );
}
