import { useState, useEffect, useCallback } from 'react';
import { Plus, X, Download, Trash2, Loader2, Check, AlertCircle, ArchiveRestore, Clock, Bell } from 'lucide-react';
import { cn } from '../../lib/utils';
import { llmConfigService } from '../../services/llmConfigService';
import { roleService } from '../../services/roleService';
import { mcpService } from '../../services/mcpService';
import { schedulerService } from '../../services/schedulerService';
import { appService } from '../../services/appService';
import type { LlmConfig, CreateLlmConfigInput, UpdateLlmConfigInput } from '../../types/settings';
import type { McpServer, McpServerType } from '../../types/mcp';
import type { Role } from '../../types/role';
import { getRoleIconComponent } from '../../lib/roleIcons';

type TestStatus = 'idle' | 'testing' | 'success' | 'error';
type McpForm = {
  name: string;
  serverType: McpServerType;
  commandOrUrl: string;
  envRefs: string;
  description: string;
  enabled: boolean;
};

const EMPTY_MCP_FORM: McpForm = {
  name: '新 MCP server',
  serverType: 'sse',
  commandOrUrl: '',
  envRefs: '{}',
  description: '',
  enabled: true,
};

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
  const [mcpServers, setMcpServers] = useState<McpServer[]>([]);
  const [mcpForm, setMcpForm] = useState<McpForm>(EMPTY_MCP_FORM);
  const [editingMcpId, setEditingMcpId] = useState<string | null>(null);
  const [isEditingMcp, setIsEditingMcp] = useState(false);
  const [isSavingMcp, setIsSavingMcp] = useState(false);
  const [mcpError, setMcpError] = useState('');
  const [testingMcpId, setTestingMcpId] = useState<string | null>(null);
  const [mcpTestResult, setMcpTestResult] = useState<{ id: string; status: 'success' | 'error'; message: string } | null>(null);
  const [pendingDeleteMcp, setPendingDeleteMcp] = useState<McpServer | null>(null);
  const [mcpImportJson, setMcpImportJson] = useState('');
  const [schedulerTimes, setSchedulerTimes] = useState<{ moderate: string[]; proactive: string[] }>({ moderate: [], proactive: [] });
  const [isSavingScheduler, setIsSavingScheduler] = useState(false);
  const [schedulerError, setSchedulerError] = useState('');
  const [schedulerSaved, setSchedulerSaved] = useState(false);
  const [knockSoundEnabled, setKnockSoundEnabled] = useState(false);
  const [isSavingSound, setIsSavingSound] = useState(false);

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

  const loadMcpServers = useCallback(async () => {
    setMcpError('');
    try {
      const servers = await mcpService.list();
      setMcpServers(servers);
    } catch (e) {
      console.error('加载 MCP server 失败:', e);
      setMcpError('MCP server 加载失败，请稍后重试');
    }
  }, []);

  const loadSchedulerTimes = useCallback(async () => {
    setSchedulerError('');
    try {
      const times = await schedulerService.getTimes();
      setSchedulerTimes(times);
    } catch (e) {
      console.error('加载调度时间失败:', e);
      setSchedulerError(typeof e === 'string' ? e : '调度时间加载失败，请稍后重试');
    }
  }, []);

  const loadKnockSound = useCallback(async () => {
    try {
      const value = await appService.getSetting('notification.knock_sound');
      setKnockSoundEnabled(value === 'true');
    } catch (e) {
      console.error('加载声音设置失败:', e);
    }
  }, []);

  const handleToggleKnockSound = async () => {
    const next = !knockSoundEnabled;
    setKnockSoundEnabled(next);
    setIsSavingSound(true);
    try {
      await appService.setSetting('notification.knock_sound', String(next));
    } catch (e) {
      console.error('保存声音设置失败:', e);
      setKnockSoundEnabled(!next);
    } finally {
      setIsSavingSound(false);
    }
  };

  useEffect(() => {
    loadConfigs();
    loadArchivedRoles();
    loadMcpServers();
    loadSchedulerTimes();
    loadKnockSound();
  }, [loadArchivedRoles, loadConfigs, loadMcpServers, loadSchedulerTimes, loadKnockSound]);

  const handleAddSchedulerTime = (level: 'moderate' | 'proactive') => {
    setSchedulerSaved(false);
    setSchedulerTimes(prev => ({
      ...prev,
      [level]: [...prev[level], '12:00'].sort(),
    }));
  };

  const handleRemoveSchedulerTime = (level: 'moderate' | 'proactive', index: number) => {
    setSchedulerSaved(false);
    setSchedulerTimes(prev => ({
      ...prev,
      [level]: prev[level].filter((_, i) => i !== index),
    }));
  };

  const handleSchedulerTimeChange = (level: 'moderate' | 'proactive', index: number, value: string) => {
    setSchedulerSaved(false);
    setSchedulerTimes(prev => {
      const next = [...prev[level]];
      next[index] = value;
      return { ...prev, [level]: next };
    });
  };

  const handleSaveSchedulerTimes = async () => {
    setIsSavingScheduler(true);
    setSchedulerError('');
    setSchedulerSaved(false);
    try {
      const saved = await schedulerService.setTimes(schedulerTimes.moderate, schedulerTimes.proactive);
      setSchedulerTimes(saved);
      setSchedulerSaved(true);
      setTimeout(() => setSchedulerSaved(false), 3000);
    } catch (e) {
      setSchedulerError(typeof e === 'string' ? e : '调度时间保存失败，请检查输入');
    } finally {
      setIsSavingScheduler(false);
    }
  };

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

  const handleNewMcp = () => {
    setMcpForm(EMPTY_MCP_FORM);
    setEditingMcpId(null);
    setIsEditingMcp(true);
    setMcpError('');
    setMcpTestResult(null);
    setMcpImportJson('');
  };

  const handleEditMcp = (server: McpServer) => {
    setMcpForm({
      name: server.name,
      serverType: normalizeMcpServerType(server.serverType),
      commandOrUrl: server.commandOrUrl,
      envRefs: server.envRefs || '{}',
      description: server.description,
      enabled: server.enabled,
    });
    setEditingMcpId(server.id);
    setIsEditingMcp(true);
    setMcpError('');
    setMcpTestResult(null);
    setMcpImportJson('');
  };

  const handleSaveMcp = async () => {
    setIsSavingMcp(true);
    setMcpError('');
    try {
      const input = {
        name: mcpForm.name,
        serverType: mcpForm.serverType,
        commandOrUrl: mcpForm.commandOrUrl,
        envRefs: mcpForm.envRefs,
        description: mcpForm.description,
        enabled: mcpForm.enabled,
      };
      if (editingMcpId) {
        await mcpService.update(editingMcpId, input);
      } else {
        await mcpService.create(input);
      }
      await loadMcpServers();
      setIsEditingMcp(false);
      setEditingMcpId(null);
      setMcpForm(EMPTY_MCP_FORM);
    } catch (e) {
      setMcpError(toFriendlyMcpError(e, 'MCP server 保存失败，请稍后重试'));
    } finally {
      setIsSavingMcp(false);
    }
  };

  const handleDeleteMcp = async (id: string) => {
    setMcpError('');
    try {
      await mcpService.delete(id);
      setPendingDeleteMcp(null);
      await loadMcpServers();
    } catch (e) {
      setMcpError(toFriendlyMcpError(e, 'MCP server 删除失败，请稍后重试'));
    }
  };

  const handleToggleMcp = async (server: McpServer) => {
    setMcpError('');
    try {
      await mcpService.update(server.id, { enabled: !server.enabled });
      await loadMcpServers();
    } catch (e) {
      setMcpError(toFriendlyMcpError(e, 'MCP server 状态更新失败，请稍后重试'));
    }
  };

  const handleImportMcpJson = () => {
    setMcpError('');
    try {
      const imported = JSON.parse(mcpImportJson);
      if (!imported || typeof imported !== 'object' || Array.isArray(imported)) {
        setMcpError('MCP JSON 必须是对象');
        return;
      }
      const importedServer = extractImportedMcpServer(imported as Record<string, unknown>);
      if (!importedServer) {
        setMcpError('MCP JSON 未包含可导入的 MCP server');
        return;
      }
      const object = importedServer.config;
      const commandOrUrl = stringValue(object.url) || stringValue(object.command) || stringValue(object.commandOrUrl);
      const env = object.env ?? object.environment ?? object.headers ?? object.envRefs;
      setMcpForm({
        name: stringValue(object.name) || importedServer.name || EMPTY_MCP_FORM.name,
        serverType: normalizeMcpServerType(stringValue(object.type) || stringValue(object.serverType)),
        commandOrUrl,
        envRefs: formatImportedEnvRefs(env),
        description: stringValue(object.description),
        enabled: typeof object.enabled === 'boolean' ? object.enabled : true,
      });
    } catch {
      setMcpError('MCP JSON 格式无效');
    }
  };

  const handleTestMcp = async (id: string) => {
    setTestingMcpId(id);
    setMcpTestResult(null);
    setMcpError('');
    try {
      await mcpService.test(id);
      setMcpTestResult({ id, status: 'success', message: '连接成功' });
    } catch (e) {
      setMcpTestResult({ id, status: 'error', message: toFriendlyMcpError(e, 'MCP server 无法连接，请检查配置') });
    } finally {
      setTestingMcpId(null);
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
          <button onClick={() => { setTab('mcp'); setIsEditing(false); setIsEditingMcp(false); }} className={cn("text-left px-3 py-2 rounded-lg text-[14px] font-medium transition-colors", tab === 'mcp' ? "bg-white text-indigo-600 shadow-sm border border-slate-200/60" : "text-slate-600 hover:bg-slate-200/50")}>MCP 工具</button>
          <button onClick={() => { setTab('scheduler'); setIsEditing(false); setIsEditingMcp(false); }} className={cn("text-left px-3 py-2 rounded-lg text-[14px] font-medium transition-colors", tab === 'scheduler' ? "bg-white text-indigo-600 shadow-sm border border-slate-200/60" : "text-slate-600 hover:bg-slate-200/50")}>调度时间</button>
          <button onClick={() => { setTab('data'); setIsEditing(false); setIsEditingMcp(false); }} className={cn("text-left px-3 py-2 rounded-lg text-[14px] font-medium transition-colors", tab === 'data' ? "bg-white text-indigo-600 shadow-sm border border-slate-200/60" : "text-slate-600 hover:bg-slate-200/50")}>数据与主权</button>
          <button onClick={() => { setTab('mission'); setIsEditing(false); setIsEditingMcp(false); }} className={cn("text-left px-3 py-2 rounded-lg text-[14px] font-medium transition-colors", tab === 'mission' ? "bg-white text-indigo-600 shadow-sm border border-slate-200/60" : "text-slate-600 hover:bg-slate-200/50")}>使命宣言</button>
        </div>
        <div className="flex-1 p-10 overflow-y-auto">
          <div className="flex justify-between items-center mb-8">
            <h3 className="text-[24px] font-semibold text-slate-800">{tab === 'llm' ? 'LLM Provider 配置' : tab === 'mcp' ? 'MCP 工具配置' : tab === 'scheduler' ? '调度时间配置' : tab === 'data' ? '数据与隐私' : '个人使命宣言'}</h3>
            <button
              onClick={() => {
                if (tab === 'mcp' && isEditingMcp) {
                  setIsEditingMcp(false);
                  setEditingMcpId(null);
                  setMcpForm(EMPTY_MCP_FORM);
                  setMcpImportJson('');
                  setMcpError('');
                  return;
                }
                onClose();
              }}
              aria-label={tab === 'mcp' && isEditingMcp ? '返回 MCP 工具列表' : '关闭全局设置'}
              className="p-2 text-slate-400 hover:bg-slate-100 rounded-full transition-colors"
            ><X size={24}/></button>
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
                            {testingConfigId === conf.id ? <Loader2 size={14} className="animate-loading-spin" /> : '测试连接'}
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
                      {isSaving ? <Loader2 size={14} className="animate-loading-spin inline mr-1" /> : null}
                      保存配置
                    </button>
                  </div>
                </div>
              )}
            </div>
          )}

          {tab === 'mcp' && (
            <div className="space-y-6">
              <div className="bg-amber-50 border border-amber-100 rounded-xl p-4 text-[13px] text-amber-800 leading-relaxed">
                这是外部工具接入，不是 EgoSync 内部 create_role/delegate 工具。secret 只能填写 env: 引用，不会明文写入数据库、opencode.json 或日志。
              </div>

              {mcpError && (
                <div className="rounded-lg border border-red-100 bg-red-50 px-3 py-2 text-[13px] text-red-600 flex items-center gap-2">
                  <AlertCircle size={14} /> {mcpError}
                </div>
              )}

              {!isEditingMcp ? (
                <div className="space-y-4">
                  {mcpServers.length === 0 ? (
                    <div className="rounded-xl border border-dashed border-slate-200 bg-slate-50 px-4 py-8 text-center text-[13px] text-slate-400">暂无 MCP server</div>
                  ) : mcpServers.map(server => (
                    <div key={server.id} className="bg-white border border-slate-200 rounded-xl p-4 shadow-sm">
                      <div className="flex items-start justify-between gap-4">
                        <div className="min-w-0">
                          <div className="flex flex-wrap items-center gap-2">
                            <span className={cn('w-2.5 h-2.5 rounded-full', server.enabled ? 'bg-emerald-500' : 'bg-slate-300')} />
                            <span className="text-[15px] font-medium text-slate-800 break-words">{server.name}</span>
                            <span className="rounded-full border border-slate-200 bg-slate-50 px-2 py-0.5 text-[11px] font-medium text-slate-500">{mcpServerTypeLabel(server.serverType)}</span>
                            {!server.enabled && <span className="rounded-full border border-slate-200 bg-slate-50 px-2 py-0.5 text-[11px] font-medium text-slate-500">已停用</span>}
                          </div>
                          <p className="mt-2 break-words font-mono text-[12px] text-slate-500">{server.commandOrUrl}</p>
                          <p className={cn('mt-1.5 break-words text-[12.5px] leading-relaxed', server.description ? 'text-slate-500' : 'text-slate-400')}>描述：{server.description || '暂无描述'}</p>
                          {mcpTestResult?.id === server.id && (
                            <div className={cn('mt-2 flex items-center gap-1.5 text-[12.5px]', mcpTestResult.status === 'success' ? 'text-emerald-600' : 'text-red-600')}>
                              {mcpTestResult.status === 'success' ? <Check size={14} /> : <AlertCircle size={14} />}
                              {mcpTestResult.message}
                            </div>
                          )}
                        </div>
                        <div className="flex shrink-0 items-center gap-2">
                          <button
                            type="button"
                            role="switch"
                            aria-checked={server.enabled}
                            aria-label={`${server.enabled ? '停用' : '启用'} ${server.name}`}
                            onClick={() => handleToggleMcp(server)}
                            className={cn(
                              'relative h-6 w-11 rounded-full transition-colors focus:outline-none focus:ring-2 focus:ring-indigo-500/30',
                              server.enabled ? 'bg-indigo-600' : 'bg-slate-300'
                            )}
                          >
                            <span
                              className={cn(
                                'absolute left-0.5 top-0.5 h-5 w-5 rounded-full bg-white shadow-sm transition-transform',
                                server.enabled ? 'translate-x-5' : 'translate-x-0'
                              )}
                            />
                          </button>
                          <button onClick={() => handleTestMcp(server.id)} disabled={testingMcpId === server.id} className="px-3 py-1.5 text-[13px] font-medium text-slate-600 hover:text-green-600 hover:bg-green-50 rounded-lg transition-colors disabled:opacity-50">
                            {testingMcpId === server.id ? <Loader2 size={14} className="animate-loading-spin" /> : '测试连接'}
                          </button>
                          <button onClick={() => handleEditMcp(server)} className="px-3 py-1.5 text-[13px] font-medium text-slate-600 hover:text-indigo-600 hover:bg-indigo-50 rounded-lg transition-colors">编辑</button>
                          <button onClick={() => setPendingDeleteMcp(server)} className="px-3 py-1.5 text-[13px] font-medium text-slate-600 hover:text-red-600 hover:bg-red-50 rounded-lg transition-colors">删除</button>
                        </div>
                      </div>
                    </div>
                  ))}
                  <button onClick={handleNewMcp} className="w-full py-4 border-2 border-dashed border-slate-200 rounded-xl text-[14px] font-medium text-slate-500 hover:border-indigo-300 hover:text-indigo-600 hover:bg-indigo-50/50 transition-all flex items-center justify-center gap-2">
                    <Plus size={16} /> 添加 MCP server
                  </button>
                </div>
              ) : (
                <div className="bg-slate-50 border border-slate-200 rounded-xl p-6 animate-in slide-in-from-bottom-2">
                  <h4 className="text-[16px] font-medium text-slate-800 mb-6">{editingMcpId ? '编辑 MCP server' : '新增 MCP server'}</h4>
                  {!editingMcpId && (
                    <div className="mb-6 rounded-xl border border-dashed border-slate-200 bg-white p-4">
                      <label htmlFor="mcp-json-import" className="block text-[13px] font-medium text-slate-700 mb-1.5">MCP JSON</label>
                      <textarea
                        id="mcp-json-import"
                        aria-label="MCP JSON"
                        value={mcpImportJson}
                        onChange={e => setMcpImportJson(e.target.value)}
                        rows={4}
                        placeholder='{ "name": "天气查询", "type": "sse", "url": "https://example.com/mcp" }'
                        className="w-full bg-white border border-slate-200 rounded-lg px-4 py-2.5 text-[14px] focus:ring-2 focus:ring-indigo-500/20 focus:border-indigo-500 outline-none font-mono resize-none"
                      />
                      <div className="mt-3 flex justify-end">
                        <button onClick={handleImportMcpJson} className="px-4 py-2 rounded-lg text-[13px] font-medium text-indigo-600 hover:bg-indigo-50 border border-indigo-200 transition-colors">导入 JSON</button>
                      </div>
                    </div>
                  )}
                  <div className="space-y-4">
                    <div>
                      <label className="block text-[13px] font-medium text-slate-700 mb-1.5">名称</label>
                      <input type="text" value={mcpForm.name} onChange={e => setMcpForm({ ...mcpForm, name: e.target.value })} className="w-full bg-white border border-slate-200 rounded-lg px-4 py-2.5 text-[14px] focus:ring-2 focus:ring-indigo-500/20 focus:border-indigo-500 outline-none" />
                    </div>
                    <div>
                      <label className="block text-[13px] font-medium text-slate-700 mb-1.5">类型</label>
                      <select value={mcpForm.serverType} onChange={e => setMcpForm({ ...mcpForm, serverType: e.target.value as McpServerType })} className="w-full bg-white border border-slate-200 rounded-lg px-4 py-2.5 text-[14px] focus:ring-2 focus:ring-indigo-500/20 focus:border-indigo-500 outline-none">
                        <option value="sse">SSE</option>
                        <option value="streamable_http">Streamable HTTP</option>
                        <option value="stdio">stdio</option>
                      </select>
                    </div>
                    <div>
                      <label className="block text-[13px] font-medium text-slate-700 mb-1.5">URL / Command</label>
                      <input type="text" value={mcpForm.commandOrUrl} onChange={e => setMcpForm({ ...mcpForm, commandOrUrl: e.target.value })} placeholder={isRemoteMcpType(mcpForm.serverType) ? 'http://localhost:8000/sse' : 'npx -y @modelcontextprotocol/server-filesystem'} className="w-full bg-white border border-slate-200 rounded-lg px-4 py-2.5 text-[14px] focus:ring-2 focus:ring-indigo-500/20 focus:border-indigo-500 outline-none font-mono" />
                    </div>
                    <div>
                      <label className="block text-[13px] font-medium text-slate-700 mb-1.5">环境变量引用 JSON</label>
                      <textarea value={mcpForm.envRefs} onChange={e => setMcpForm({ ...mcpForm, envRefs: e.target.value })} rows={3} className="w-full bg-white border border-slate-200 rounded-lg px-4 py-2.5 text-[14px] focus:ring-2 focus:ring-indigo-500/20 focus:border-indigo-500 outline-none font-mono resize-none" />
                      <p className="mt-1.5 text-[12px] text-slate-400">示例：{`{"TOKEN":"env:CALENDAR_TOKEN"}`}</p>
                    </div>
                    <div>
                      <label className="block text-[13px] font-medium text-slate-700 mb-1.5">描述（选填）</label>
                      <textarea value={mcpForm.description} onChange={e => setMcpForm({ ...mcpForm, description: e.target.value })} rows={2} className="w-full bg-white border border-slate-200 rounded-lg px-4 py-2.5 text-[14px] focus:ring-2 focus:ring-indigo-500/20 focus:border-indigo-500 outline-none resize-none" />
                    </div>
                  </div>
                  <div className="mt-6 flex justify-end gap-3">
                    <button onClick={() => setIsEditingMcp(false)} className="px-5 py-2.5 border border-slate-300 rounded-lg text-[13px] font-medium text-slate-700 hover:bg-slate-100 transition-colors">取消</button>
                    <button onClick={handleSaveMcp} disabled={isSavingMcp} className="px-5 py-2.5 bg-indigo-600 text-white rounded-lg text-[13px] font-medium hover:bg-indigo-700 transition-colors shadow-sm disabled:opacity-50">
                      {isSavingMcp ? <Loader2 size={14} className="animate-loading-spin inline mr-1" /> : null}
                      保存 MCP server
                    </button>
                  </div>
                </div>
              )}

              {pendingDeleteMcp && (
                <div className="fixed inset-0 z-[60] flex items-center justify-center bg-slate-900/30 px-4">
                  <div className="w-full max-w-md rounded-2xl border border-slate-200 bg-white p-6 shadow-2xl">
                    <h4 className="text-[16px] font-semibold text-slate-800">确认删除 MCP server？</h4>
                    <p className="mt-2 text-[13px] leading-relaxed text-slate-500">
                      删除后会从全局 MCP 工具列表移除「{pendingDeleteMcp.name}」，并解除所有角色绑定。
                    </p>
                    <div className="mt-6 flex justify-end gap-3">
                      <button onClick={() => setPendingDeleteMcp(null)} className="px-4 py-2 rounded-lg border border-slate-300 text-[13px] font-medium text-slate-700 hover:bg-slate-50 transition-colors">取消</button>
                      <button onClick={() => handleDeleteMcp(pendingDeleteMcp.id)} className="px-4 py-2 rounded-lg bg-red-600 text-[13px] font-medium text-white hover:bg-red-700 transition-colors">确认删除</button>
                    </div>
                  </div>
                </div>
              )}
            </div>
          )}

          {tab === 'scheduler' && (
            <div className="space-y-6">
              <div className="bg-indigo-50 border border-indigo-100 rounded-xl p-4 text-[13px] text-indigo-800 leading-relaxed flex items-start gap-2">
                <Clock size={16} className="mt-0.5 shrink-0" />
                <span>为每个主动性档位设置每日触发时间点（本地时间，精确到分钟）。所有 moderate / proactive 角色共用对应档位的时间点，保存后下次调度自动生效。</span>
              </div>

              {schedulerError && (
                <div className="rounded-lg border border-red-100 bg-red-50 px-3 py-2 text-[13px] text-red-600 flex items-center gap-2">
                  <AlertCircle size={14} /> {schedulerError}
                </div>
              )}

              <div className="space-y-6">
                {(['moderate', 'proactive'] as const).map(level => (
                  <div key={level} className="bg-white border border-slate-200 rounded-xl p-5 shadow-sm">
                    <div className="flex items-center gap-2 mb-4">
                      <span className="text-[15px] font-medium text-slate-800">
                        {level === 'moderate' ? '适度建议' : '积极主动'}
                      </span>
                      <span className="rounded-full border border-slate-200 bg-slate-50 px-2 py-0.5 text-[11px] font-medium text-slate-500">
                        {schedulerTimes[level].length} 个时间点
                      </span>
                    </div>
                    <div className="flex flex-wrap items-center gap-2">
                      {schedulerTimes[level].map((time, index) => (
                        <div key={index} className="flex items-center gap-1 rounded-lg border border-slate-200 bg-white px-2 py-1.5">
                          <input
                            type="time"
                            value={time}
                            onChange={e => handleSchedulerTimeChange(level, index, e.target.value)}
                            className="text-[13px] text-slate-700 outline-none bg-transparent"
                          />
                          <button
                            type="button"
                            aria-label={`删除 ${time}`}
                            onClick={() => handleRemoveSchedulerTime(level, index)}
                            className="text-slate-400 hover:text-red-500 transition-colors"
                          >
                            <X size={14} />
                          </button>
                        </div>
                      ))}
                      {schedulerTimes[level].length < 12 && (
                        <button
                          type="button"
                          onClick={() => handleAddSchedulerTime(level)}
                          className="inline-flex items-center gap-1 rounded-lg border border-dashed border-slate-300 bg-white px-3 py-1.5 text-[12px] font-medium text-slate-500 hover:border-indigo-300 hover:text-indigo-600 transition-colors"
                        >
                          <Plus size={14} /> 添加
                        </button>
                      )}
                    </div>
                  </div>
                ))}
              </div>

              <div className="flex items-center gap-3">
                <button
                  onClick={handleSaveSchedulerTimes}
                  disabled={isSavingScheduler}
                  className="px-5 py-2.5 bg-indigo-600 text-white rounded-lg text-[13px] font-medium hover:bg-indigo-700 transition-colors shadow-sm disabled:opacity-50 flex items-center gap-1.5"
                >
                  {isSavingScheduler ? <Loader2 size={14} className="animate-loading-spin" /> : null}
                  保存调度时间
                </button>
                {schedulerSaved && (
                  <span className="text-[13px] text-emerald-600 flex items-center gap-1.5">
                    <Check size={14} /> 已保存
                  </span>
                )}
              </div>

              <div className="pt-6 border-t border-slate-200">
                <div className="flex items-center justify-between">
                  <div className="flex items-start gap-3">
                    <div className="w-10 h-10 rounded-lg bg-indigo-50 flex items-center justify-center text-indigo-600 shrink-0">
                      <Bell size={18} />
                    </div>
                    <div>
                      <h4 className="text-[15px] font-medium text-slate-800">敲门通知声音</h4>
                      <p className="text-[13px] text-slate-500 mt-0.5">收到「敲门」级别通知时播放提示音</p>
                    </div>
                  </div>
                  <button
                    type="button"
                    role="switch"
                    aria-checked={knockSoundEnabled}
                    aria-label="敲门通知声音开关"
                    disabled={isSavingSound}
                    onClick={handleToggleKnockSound}
                    className={cn(
                      "relative w-12 h-6 rounded-full transition-colors duration-200 shrink-0 disabled:opacity-50",
                      knockSoundEnabled ? "bg-indigo-600" : "bg-slate-300"
                    )}
                  >
                    <span className={cn(
                      "absolute top-0.5 left-0.5 w-5 h-5 bg-white rounded-full shadow-sm transition-transform duration-200",
                      knockSoundEnabled && "translate-x-6"
                    )} />
                  </button>
                </div>
              </div>
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

function normalizeMcpServerType(value: unknown): McpServerType {
  if (value === 'http_sse') return 'sse';
  if (value === 'command') return 'stdio';
  if (value === 'streamable_http' || value === 'stdio' || value === 'sse') return value;
  if (value === 'remote') return 'sse';
  if (value === 'local') return 'stdio';
  return 'sse';
}

function mcpServerTypeLabel(value: McpServerType) {
  const normalized = normalizeMcpServerType(value);
  if (normalized === 'streamable_http') return 'Streamable HTTP';
  if (normalized === 'stdio') return 'stdio';
  return 'SSE';
}

function isRemoteMcpType(value: McpServerType) {
  return normalizeMcpServerType(value) !== 'stdio';
}

function extractImportedMcpServer(object: Record<string, unknown>) {
  const servers = object.mcpServers;
  if (servers && typeof servers === 'object' && !Array.isArray(servers)) {
    const firstServer = Object.entries(servers as Record<string, unknown>).find(([, value]) => (
      value && typeof value === 'object' && !Array.isArray(value)
    ));
    if (firstServer) {
      return {
        name: firstServer[0],
        config: firstServer[1] as Record<string, unknown>,
      };
    }
    return null;
  }

  return {
    name: '',
    config: object,
  };
}

function stringValue(value: unknown) {
  return typeof value === 'string' ? value : '';
}

function formatImportedEnvRefs(value: unknown) {
  if (!value) return '{}';
  if (typeof value === 'string') return value;
  if (typeof value === 'object' && !Array.isArray(value)) {
    return JSON.stringify(value, null, 2);
  }
  return '{}';
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

function toFriendlyMcpError(error: unknown, fallback: string) {
  if (error == null) return fallback;
  const text = typeof error === 'string'
    ? error
    : error instanceof Error
      ? error.message
      : (JSON.stringify(error) ?? String(error));
  if (text.includes('secret 只能保存 env: 引用')) return 'secret 只能保存 env: 引用';
  if (text.includes('环境变量引用必须是 JSON 对象')) return '环境变量引用必须是 JSON 对象';
  if (text.includes('MCP server 名称不能为空')) return 'MCP server 名称不能为空';
  if (text.includes('MCP server 连接参数不能为空')) return 'MCP server 连接参数不能为空';
  if (text.includes('无法连接') || text.includes('不可达')) return 'MCP server 无法连接，请检查配置';
  return fallback;
}
