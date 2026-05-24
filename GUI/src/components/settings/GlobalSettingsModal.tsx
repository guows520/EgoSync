import { useState, useEffect, useCallback } from 'react';
import { Plus, X, Download, Trash2, Loader2, Check, AlertCircle, ArchiveRestore } from 'lucide-react';
import { cn } from '../../lib/utils';
import { llmConfigService } from '../../services/llmConfigService';
import { roleService } from '../../services/roleService';
import type { LlmConfig, CreateLlmConfigInput, UpdateLlmConfigInput } from '../../types/settings';
import type { Role } from '../../types/role';
import { getRoleIconComponent } from '../../lib/roleIcons';

type TestStatus = 'idle' | 'testing' | 'success' | 'error';

export function GlobalSettingsModal({ onClose, archivedRoles: initialArchivedRoles = [], onRestoreRole, onRefreshRoles }: any) {
  const [tab, setTab] = useState('llm');
  const [configs, setConfigs] = useState<LlmConfig[]>([]);
  const [isEditing, setIsEditing] = useState(false);
  const [editForm, setEditForm] = useState<any>(null);
  const [editingId, setEditingId] = useState<string | null>(null);
  const [isSaving, setIsSaving] = useState(false);
  const [testStatus, setTestStatus] = useState<TestStatus>('idle');
  const [testError, setTestError] = useState('');
  const [testingConfigId, setTestingConfigId] = useState<string | null>(null);
  const [lastTestedConfigId, setLastTestedConfigId] = useState<string | null>(null);
  const [archivedRoles, setArchivedRoles] = useState<Role[]>(initialArchivedRoles);
  const [archiveError, setArchiveError] = useState('');
  const [restoringRoleId, setRestoringRoleId] = useState<string | null>(null);

  const loadConfigs = useCallback(async () => {
    try {
      const data = await llmConfigService.list();
      setConfigs(data);
    } catch (e) {
      console.error('加载 LLM 配置失败:', e);
    }
  }, []);

  const loadArchivedRoles = useCallback(async () => {
    setArchiveError('');
    try {
      const archived = await roleService.listArchived();
      setArchivedRoles(archived);
    } catch (e) {
      console.error('加载归档角色失败:', e);
      setArchiveError('归档角色加载失败，请稍后重试');
    }
  }, []);

  useEffect(() => {
    loadConfigs();
    loadArchivedRoles();
  }, [loadArchivedRoles, loadConfigs]);

  const handleEdit = (conf: LlmConfig) => {
    setEditForm({ name: conf.name, provider: conf.provider, baseUrl: conf.baseUrl, model: conf.model, apiKey: '' });
    setEditingId(conf.id);
    setIsEditing(true);
    setTestStatus('idle');
    setTestError('');
  };

  const handleNew = () => {
    setEditForm({ name: '新配置', provider: 'openai_compatible', baseUrl: '', apiKey: '', model: '' });
    setEditingId(null);
    setIsEditing(true);
    setTestStatus('idle');
    setTestError('');
  };

  const handleSave = async () => {
    setIsSaving(true);
    try {
      if (editingId) {
        const input: UpdateLlmConfigInput = {
          name: editForm.name,
          provider: editForm.provider,
          baseUrl: editForm.baseUrl,
          model: editForm.model,
        };
        if (editForm.apiKey) input.apiKey = editForm.apiKey;
        await llmConfigService.update(editingId, input);
      } else {
        const input: CreateLlmConfigInput = {
          name: editForm.name,
          provider: editForm.provider,
          baseUrl: editForm.baseUrl,
          model: editForm.model,
          apiKey: editForm.apiKey,
        };
        await llmConfigService.create(input);
      }
      await loadConfigs();
      setIsEditing(false);
    } catch (e: any) {
      console.error('保存失败:', e);
    } finally {
      setIsSaving(false);
    }
  };

  const handleDelete = async (id: string) => {
    try {
      await llmConfigService.delete(id);
      await loadConfigs();
    } catch (e) {
      console.error('删除失败:', e);
    }
  };

  const handleSetDefault = async (id: string) => {
    try {
      await llmConfigService.setDefault(id);
      await loadConfigs();
    } catch (e) {
      console.error('设置默认失败:', e);
    }
  };

  const handleTestConnection = async (id: string) => {
    setTestingConfigId(id);
    setLastTestedConfigId(id);
    setTestStatus('testing');
    setTestError('');
    try {
      await llmConfigService.testConnection(id);
      setTestStatus('success');
    } catch (e: any) {
      setTestStatus('error');
      const errMsg = typeof e === 'string' ? e : (e?.message || JSON.stringify(e));
      setTestError(errMsg);
    } finally {
      setTestingConfigId(null);
    }
  };

  const handleRestoreArchivedRole = async (id: string) => {
    setRestoringRoleId(id);
    setArchiveError('');
    try {
      if (onRestoreRole) {
        await onRestoreRole(id);
      } else {
        await roleService.restore(id);
        await onRefreshRoles?.();
      }
      await loadArchivedRoles();
    } catch (e) {
      console.error('恢复归档角色失败:', e);
      setArchiveError('恢复失败，请稍后重试');
    } finally {
      setRestoringRoleId(null);
    }
  };

  return (
    <div className="fixed top-0 right-0 bottom-0 left-16 z-50 animate-in slide-in-from-right duration-300">
      <div className="h-full bg-white shadow-2xl flex">
        <div className="w-64 bg-slate-50 border-r border-slate-200 p-6 flex flex-col gap-2 shrink-0">
          <div className="flex items-center justify-between mb-8 px-3">
            <h2 className="text-[16px] font-semibold text-slate-800">全局设置</h2>
          </div>
          <button onClick={() => { setTab('llm'); setIsEditing(false); }} className={cn("text-left px-3 py-2 rounded-lg text-[14px] font-medium transition-colors", tab === 'llm' ? "bg-white text-indigo-600 shadow-sm border border-slate-200/60" : "text-slate-600 hover:bg-slate-200/50")}>模型配置 (BYOK)</button>
          <button onClick={() => { setTab('data'); setIsEditing(false); }} className={cn("text-left px-3 py-2 rounded-lg text-[14px] font-medium transition-colors", tab === 'data' ? "bg-white text-indigo-600 shadow-sm border border-slate-200/60" : "text-slate-600 hover:bg-slate-200/50")}>数据与主权</button>
          <button onClick={() => { setTab('mission'); setIsEditing(false); }} className={cn("text-left px-3 py-2 rounded-lg text-[14px] font-medium transition-colors", tab === 'mission' ? "bg-white text-indigo-600 shadow-sm border border-slate-200/60" : "text-slate-600 hover:bg-slate-200/50")}>使命宣言</button>
        </div>
        <div className="flex-1 p-10 overflow-y-auto">
          <div className="flex justify-between items-center mb-8">
            <h3 className="text-[24px] font-semibold text-slate-800">{tab === 'llm' ? 'LLM Provider 配置' : tab === 'data' ? '数据与隐私' : '个人使命宣言'}</h3>
            <button onClick={onClose} className="p-2 text-slate-400 hover:bg-slate-100 rounded-full transition-colors"><X size={24}/></button>
          </div>
          
          {tab === 'llm' && (
            <div className="space-y-6">
              <div className="bg-indigo-50 border border-indigo-100 rounded-xl p-4 text-[13px] text-indigo-800 leading-relaxed">
                EgoSync 采用 BYOK (Bring Your Own Key) 模式，我们不触碰你的数据，也不赚取 API 差价。支持配置多个 Provider，您可以自由切换使用。
              </div>

              {!isEditing ? (
                <div className="space-y-4">
                  {configs.map((conf) => (
                    <div key={conf.id} className={cn("border rounded-xl p-4 transition-all", conf.isDefault ? "bg-indigo-50/30 border-indigo-200 shadow-sm" : "bg-white border-slate-200 hover:border-slate-300")}>
                      <div className="flex items-center justify-between">
                        <div className="flex items-center gap-3">
                          <input type="radio" id={`conf-${conf.id}`} name="activeConfig" checked={conf.isDefault} onChange={() => handleSetDefault(conf.id)} className="w-4 h-4 text-indigo-600 accent-indigo-600" />
                          <label htmlFor={`conf-${conf.id}`} className="font-medium text-[15px] text-slate-800 cursor-pointer">{conf.name}</label>
                          {conf.isDefault && <span className="px-2 py-0.5 rounded text-[11px] font-medium bg-indigo-100 text-indigo-700">当前启用</span>}
                        </div>
                        <div className="flex items-center gap-2">
                          <button onClick={() => handleTestConnection(conf.id)} disabled={testingConfigId === conf.id} className="px-3 py-1.5 text-[13px] font-medium text-slate-600 hover:text-green-600 hover:bg-green-50 rounded-lg transition-colors disabled:opacity-50">
                            {testingConfigId === conf.id ? <Loader2 size={14} className="animate-spin" /> : '测试连接'}
                          </button>
                          <button onClick={() => handleEdit(conf)} className="px-3 py-1.5 text-[13px] font-medium text-slate-600 hover:text-indigo-600 hover:bg-indigo-50 rounded-lg transition-colors">编辑</button>
                          <button onClick={() => handleDelete(conf.id)} className="px-3 py-1.5 text-[13px] font-medium text-slate-600 hover:text-red-600 hover:bg-red-50 rounded-lg transition-colors">删除</button>
                        </div>
                      </div>
                      <div className="mt-3 pl-7 grid grid-cols-2 gap-y-2 text-[13px] text-slate-500">
                        <div><span className="text-slate-400 mr-2">模型:</span>{conf.model || '-'}</div>
                        <div><span className="text-slate-400 mr-2">标准:</span>{conf.provider === 'anthropic' ? 'Anthropic (Claude)' : 'OpenAI 兼容'}</div>
                      </div>
                      {testStatus !== 'idle' && testingConfigId === null && lastTestedConfigId === conf.id && (
                        <div className={cn("mt-3 pl-7 text-[13px] flex items-center gap-1.5", testStatus === 'success' ? 'text-green-600' : testStatus === 'error' ? 'text-red-600' : 'text-slate-500')}>
                          {testStatus === 'success' && <><Check size={14} /> 连接成功</>}
                          {testStatus === 'error' && <><AlertCircle size={14} /> {testError}</>}
                        </div>
                      )}
                    </div>
                  ))}
                  <button onClick={handleNew} className="w-full py-4 border-2 border-dashed border-slate-200 rounded-xl text-[14px] font-medium text-slate-500 hover:border-indigo-300 hover:text-indigo-600 hover:bg-indigo-50/50 transition-all flex items-center justify-center gap-2">
                    <Plus size={16} /> 添加新配置
                  </button>
                </div>
              ) : (
                <div className="bg-slate-50 border border-slate-200 rounded-xl p-6 animate-in slide-in-from-bottom-2">
                  <div className="flex justify-between items-center mb-6">
                    <h4 className="text-[16px] font-medium text-slate-800">{editingId ? '编辑配置' : '新建配置'}</h4>
                  </div>
                  <div className="space-y-4">
                    <div>
                      <label className="block text-[13px] font-medium text-slate-700 mb-1.5">配置名称</label>
                      <input type="text" value={editForm.name} onChange={e => setEditForm({...editForm, name: e.target.value})} className="w-full bg-white border border-slate-200 rounded-lg px-4 py-2.5 text-[14px] focus:ring-2 focus:ring-indigo-500/20 focus:border-indigo-500 outline-none" />
                    </div>
                    <div>
                      <label className="block text-[13px] font-medium text-slate-700 mb-1.5">Provider 标准</label>
                      <select value={editForm.provider} onChange={e => setEditForm({...editForm, provider: e.target.value})} className="w-full bg-white border border-slate-200 rounded-lg px-4 py-2.5 text-[14px] focus:ring-2 focus:ring-indigo-500/20 focus:border-indigo-500 outline-none">
                        <option value="openai_compatible">OpenAI 兼容 (OpenAI, DeepSeek, Ollama...)</option>
                        <option value="anthropic">Anthropic (Claude)</option>
                      </select>
                    </div>
                    <div>
                      <label className="block text-[13px] font-medium text-slate-700 mb-1.5">Base URL</label>
                      <input type="text" value={editForm.baseUrl} onChange={e => setEditForm({...editForm, baseUrl: e.target.value})} placeholder={editForm.provider === 'anthropic' ? 'https://api.anthropic.com' : 'https://api.openai.com/v1'} className="w-full bg-white border border-slate-200 rounded-lg px-4 py-2.5 text-[14px] focus:ring-2 focus:ring-indigo-500/20 focus:border-indigo-500 outline-none font-mono" />
                    </div>
                    <div>
                      <label className="block text-[13px] font-medium text-slate-700 mb-1.5">API Key{editingId ? ' (留空则不修改)' : ''}</label>
                      <input type="password" value={editForm.apiKey} onChange={e => setEditForm({...editForm, apiKey: e.target.value})} placeholder={editingId ? '••••••••' : 'sk-...'} className="w-full bg-white border border-slate-200 rounded-lg px-4 py-2.5 text-[14px] focus:ring-2 focus:ring-indigo-500/20 focus:border-indigo-500 outline-none font-mono" />
                    </div>
                    <div>
                      <label className="block text-[13px] font-medium text-slate-700 mb-1.5">Model Name</label>
                      <input type="text" value={editForm.model} onChange={e => setEditForm({...editForm, model: e.target.value})} placeholder="gpt-4o" className="w-full bg-white border border-slate-200 rounded-lg px-4 py-2.5 text-[14px] focus:ring-2 focus:ring-indigo-500/20 focus:border-indigo-500 outline-none font-mono" />
                    </div>
                  </div>
                  <div className="mt-6 flex justify-end gap-3">
                    <button onClick={() => setIsEditing(false)} className="px-5 py-2.5 border border-slate-300 rounded-lg text-[13px] font-medium text-slate-700 hover:bg-slate-100 transition-colors">取消</button>
                    <button onClick={handleSave} disabled={isSaving} className="px-5 py-2.5 bg-indigo-600 text-white rounded-lg text-[13px] font-medium hover:bg-indigo-700 transition-colors shadow-sm disabled:opacity-50">
                      {isSaving ? <Loader2 size={14} className="animate-spin inline mr-1" /> : null}
                      保存配置
                    </button>
                  </div>
                </div>
              )}
            </div>
          )}

          {tab === 'data' && (
            <div className="space-y-8">
              <div>
                <h4 className="text-[15px] font-medium text-slate-800 mb-2">导出完整数据</h4>
                <p className="text-[13px] text-slate-500 mb-4">将所有角色的记忆、任务和对话记录导出为标准的 JSON/Markdown 格式。</p>
                <button className="flex items-center gap-2 px-5 py-2.5 border border-slate-300 rounded-lg text-[14px] font-medium text-slate-700 hover:bg-slate-50 transition-colors">
                  <Download size={16}/> 导出存档
                </button>
              </div>
              <div>
                <h4 className="text-[15px] font-medium text-slate-800 mb-2">归档角色</h4>
                <p className="text-[13px] text-slate-500 mb-4">恢复后角色会重新出现在侧边栏，历史数据保持不变。</p>
                {archiveError && (
                  <div className="mb-3 rounded-lg border border-red-100 bg-red-50 px-3 py-2 text-[13px] text-red-600 flex items-center gap-2">
                    <AlertCircle size={14} /> {archiveError}
                  </div>
                )}
                <div className="space-y-2.5">
                  {archivedRoles.length === 0 ? (
                    <div className="rounded-xl border border-dashed border-slate-200 bg-slate-50 px-4 py-6 text-center text-[13px] text-slate-400">暂无归档角色</div>
                  ) : archivedRoles.map(role => {
                    const Icon = getRoleIconComponent(role.icon);
                    return (
                      <div key={role.id} className="bg-white border border-slate-200 rounded-xl p-4 flex items-center justify-between shadow-sm">
                        <div className="flex items-center gap-3 min-w-0">
                          <div className="w-10 h-10 rounded-lg flex items-center justify-center text-white shrink-0" style={{ backgroundColor: role.color }}>
                            <Icon size={18} strokeWidth={2} />
                          </div>
                          <div className="min-w-0">
                            <p className="text-[14px] font-medium text-slate-700 truncate">{role.name}</p>
                            <p className="text-[12px] text-slate-400 truncate">{role.goal || '无目标描述'}</p>
                            <p className="text-[11px] text-slate-400 mt-0.5">归档时间：{formatDateTime(role.archivedAt)}</p>
                          </div>
                        </div>
                        <button
                          onClick={() => handleRestoreArchivedRole(role.id)}
                          disabled={restoringRoleId === role.id}
                          className="px-4 py-2 rounded-lg text-[13px] font-medium text-indigo-600 hover:bg-indigo-50 border border-indigo-200 transition-colors disabled:opacity-60 flex items-center gap-1.5"
                        >
                          <ArchiveRestore size={14} /> {restoringRoleId === role.id ? '恢复中...' : '恢复'}
                        </button>
                      </div>
                    );
                  })}
                </div>
              </div>

              <div className="pt-6 border-t border-slate-200">
                <h4 className="text-[15px] font-medium text-red-600 mb-2 flex items-center gap-2">危险区域</h4>
                <p className="text-[13px] text-slate-500 mb-4">永久销毁本地数据库中的所有数据。此操作不可逆！</p>
                <button className="flex items-center gap-2 px-5 py-2.5 bg-red-50 border border-red-200 rounded-lg text-[14px] font-medium text-red-600 hover:bg-red-100 transition-colors">
                  <Trash2 size={16}/> 销毁所有数据
                </button>
              </div>
            </div>
          )}
        </div>
      </div>
    </div>
  );
}

function formatDateTime(value: string | null) {
  if (!value) return '未知';
  const date = new Date(value);
  if (Number.isNaN(date.getTime())) return value;
  return date.toLocaleString('zh-CN', {
    year: 'numeric',
    month: '2-digit',
    day: '2-digit',
    hour: '2-digit',
    minute: '2-digit',
  });
}
