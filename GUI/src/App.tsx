import { useState } from 'react';
import { cn } from './lib/utils';
import { DEFAULT_ROLES } from './constants/mockData';
import { Sidebar } from './components/layout/Sidebar';
import { ButlerView } from './components/butler/ButlerView';
import { RoleView } from './components/role/RoleView';
import { OnboardingView } from './components/onboarding/OnboardingView';
import { GlobalSettingsModal } from './components/settings/GlobalSettingsModal';
import { ArbitrationModal } from './components/modals/ArbitrationModal';
import { WeeklyReviewModal } from './components/modals/WeeklyReviewModal';
import { TaskModal } from './components/modals/TaskModal';
import { AddRoleModal } from './components/modals/AddRoleModal';
import { NotificationPanel } from './components/notifications/NotificationPanel';

export default function App() {
  const [currentView, setCurrentView] = useState('butler');
  const [isSettingsOpen, setIsSettingsOpen] = useState(false);
  const [isArbOpen, setIsArbOpen] = useState(false);
  const [isReviewOpen, setIsReviewOpen] = useState(false);
  const [isTaskModalOpen, setIsTaskModalOpen] = useState(false);
  const [isAddRoleOpen, setIsAddRoleOpen] = useState(false);
  const [roles, setRoles] = useState(DEFAULT_ROLES);
  const [archivedRoles, setArchivedRoles] = useState<typeof DEFAULT_ROLES>([]);
  const [theme, setTheme] = useState<'light' | 'dark'>(() => {
    const stored = localStorage.getItem('egosync-theme');
    if (stored === 'light' || stored === 'dark') return stored;
    return window.matchMedia('(prefers-color-scheme: dark)').matches ? 'dark' : 'light';
  });
  const [roleInitialTab, setRoleInitialTab] = useState<string | null>(null);
  const [isNotifOpen, setIsNotifOpen] = useState(false);

  const handleArchiveRole = (id: string) => {
    const role = roles.find(r => r.id === id);
    if (role) setArchivedRoles(prev => [...prev, role]);
    setRoles(roles.filter(r => r.id !== id));
    if (currentView === id) setCurrentView('butler');
  };
  const handleRestoreRole = (id: string) => {
    const role = archivedRoles.find(r => r.id === id);
    if (role) setRoles(prev => [...prev, role]);
    setArchivedRoles(archivedRoles.filter(r => r.id !== id));
  };
  const handleDeleteRole = (id: string) => {
    setRoles(roles.filter(r => r.id !== id));
    if (currentView === id) setCurrentView('butler');
  };

  return (
    <div className={cn("flex flex-col h-screen font-sans transition-colors duration-300 bg-[#F8F9FA] text-slate-900 selection:bg-indigo-100 dark:bg-slate-900 dark:text-slate-100 dark:selection:bg-indigo-900", theme === 'dark' && "dark")}>
      <div className="flex-1 flex overflow-hidden relative">
        <Sidebar roles={roles} currentView={currentView} onViewChange={setCurrentView} isSettingsOpen={isSettingsOpen} onOpenSettings={() => setIsSettingsOpen(true)} onCloseSettings={() => setIsSettingsOpen(false)} onAddRole={() => setIsAddRoleOpen(true)} onArchiveRole={handleArchiveRole} onDeleteRole={handleDeleteRole} theme={theme} onToggleTheme={() => setTheme(t => { const next = t === 'light' ? 'dark' : 'light'; localStorage.setItem('egosync-theme', next); document.documentElement.classList.toggle('dark', next === 'dark'); return next; })} onEditRole={(id: string) => { setCurrentView(id); setRoleInitialTab('settings'); setIsSettingsOpen(false); }} isNotifOpen={isNotifOpen} onToggleNotif={() => setIsNotifOpen(v => !v)} />
        
        <main className="flex-1 relative overflow-hidden bg-white/30 dark:bg-slate-900/50 backdrop-blur-3xl shadow-[inset_1px_0_10px_rgba(0,0,0,0.02)]">
          {currentView === 'onboard' && <OnboardingView onComplete={() => setCurrentView('butler')} />}
          {currentView === 'butler' && <ButlerView roles={roles} onViewChange={setCurrentView} archivedRoles={archivedRoles} onRestoreRole={handleRestoreRole} />}
          {roles.map(r => r.id === currentView && <RoleView key={r.id} role={r} onOpenTask={() => setIsTaskModalOpen(true)} initialTab={roleInitialTab} onTabConsumed={() => setRoleInitialTab(null)} onUpdateRole={(updated: any) => setRoles(roles.map(x => x.id === updated.id ? {...x, ...updated} : x))} />)}
        </main>
      </div>

      {/* OVERLAYS */}
      {isSettingsOpen && <GlobalSettingsModal onClose={() => setIsSettingsOpen(false)} />}
      {isArbOpen && <ArbitrationModal onClose={() => setIsArbOpen(false)} />}
      {isReviewOpen && <WeeklyReviewModal roles={roles} onClose={() => setIsReviewOpen(false)} />}
      {isTaskModalOpen && <TaskModal onClose={() => setIsTaskModalOpen(false)} />}
      {isAddRoleOpen && <AddRoleModal onClose={() => setIsAddRoleOpen(false)} onAdd={(role: any) => { setRoles([...roles, role]); setIsAddRoleOpen(false); }} />}
      {isNotifOpen && <NotificationPanel onClose={() => setIsNotifOpen(false)} />}
    </div>
  );
}
