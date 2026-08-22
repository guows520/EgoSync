import { useState, useEffect, useCallback, useRef } from 'react';
import { Plus, X, Download, Trash2, Loader2, Check, AlertCircle, Clock, Bell, Upload } from 'lucide-react';
import { cn } from '../../lib/utils';
import { Modal } from '../layout/Modal';
import { llmConfigService } from '../../services/llmConfigService';
import { mcpService } from '../../services/mcpService';
import { schedulerService } from '../../services/schedulerService';
import { appService } from '../../services/appService';
import { dataService } from '../../services/dataService';
import type { ExportFormat, ExportResult, ImportResult } from '../../services/dataService';
import type { LlmConfig, CreateLlmConfigInput, UpdateLlmConfigInput, LlmProviderType, NetworkLocation } from '../../types/settings';
import type { McpServer, McpServerType } from '../../types/mcp';

// 提供商默认 API 地址映射
const PROVIDER_DEFAULT_BASE_URL: Record<LlmProviderType, string> = {
  openai_compatible: 'https://api.openai.com/v1',
  anthropic: 'https://api.anthropic.com',
  minimax: 'https://api.minimaxi.com/v1',
  zhipu: 'https://open.bigmodel.cn/api/paas/v4',
  deepseek: 'https://api.deepseek.com/v1',
  kimi: 'https://api.moonshot.cn/v1',
  bailian: 'https://dashscope.aliyuncs.com/compatible-mode/v1',
};

// 提供商显示名称映射
const PROVIDER_LABEL: Record<string, string> = {
  openai_compatible: 'OpenAI兼容',
  anthropic: 'Anthropic兼容',
  minimax: 'MiniMax',
  zhipu: '智谱',
  deepseek: 'Deepseek',
  kimi: 'Kimi',
  bailian: '阿里云百炼',
};

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

export function GlobalSettingsModal({ onClose, onDataDestroyed, onDataImported }: any) {
  const [tab, setTab] = useState('llm');
  const [configs, setConfigs] = useState<LlmConfig[]>([]);
  const [isEditing, setIsEditing] = useState(false);
  const [editForm, setEditForm] = useState<any>(null);
  const [editingId, setEditingId] = useState<string | null>(null);
  const [isSaving, setIsSaving] = useState(false);
  const [testStatus, setTestStatus] = useState<TestStatus>('idle');
  const onCloseRef = useRef(onClose);

  useEffect(() => {
    onCloseRef.current = onClose;
  }, [onClose]);

  useEffect(() => {
    const handleKeyDown = (e: KeyboardEvent) => {
      if (e.key === 'Escape') {
        e.stopPropagation();
        onCloseRef.current();
      }
    };
    document.addEventListener('keydown', handleKeyDown);
    return () => document.removeEventListener('keydown', handleKeyDown);
  }, []);

  const [testError, setTestError] = useState('');
  const [testingConfigId, setTestingConfigId] = useState<string | null>(null);
  const [lastTestedConfigId, setLastTestedConfigId] = useState<string | null>(null);
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
  const [saveError, setSaveError] = useState('');
  const [runtimeRefreshError, setRuntimeRefreshError] = useState('');
  const [showFormatSelect, setShowFormatSelect] = useState(false);
  const [selectedFormats, setSelectedFormats] = useState<ExportFormat[]>([]);
  const [availableModels, setAvailableModels] = useState<string[]>([]);
  const [isLoadingModels, setIsLoadingModels] = useState(false);
  const [showModelDropdown, setShowModelDropdown] = useState(false);
  const [modelLoadError, setModelLoadError] = useState('');
  const [isExporting, setIsExporting] = useState(false);
  const [exportResult, setExportResult] = useState<ExportResult | null>(null);
  const [exportError, setExportError] = useState('');
  const [showDestroyConfirm, setShowDestroyConfirm] = useState(false);
  const [destroyConfirmText, setDestroyConfirmText] = useState('');
  const [isDestroying, setIsDestroying] = useState(false);
  const [destroyError, setDestroyError] = useState('');
  const [showImportConfirm, setShowImportConfirm] = useState(false);
  const [pendingImportPath, setPendingImportPath] = useState<string | null>(null);
  const [isImporting, setIsImporting] = useState(false);
  const [importResult, setImportResult] = useState<ImportResult | null>(null);
  const [importError, setImportError] = useState('');

  const loadConfigs = useCallback(async () => {
    try {
      const data = await llmConfigService.list();
      setConfigs(data);
    } catch (e) {
      console.error('加载 LLM 配置失败:', e);
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
    loadMcpServers();
    loadSchedulerTimes();
    loadKnockSound();
  }, [loadConfigs, loadMcpServers, loadSchedulerTimes, loadKnockSound]);

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
    setEditForm({ name: conf.name, provider: conf.provider, baseUrl: conf.baseUrl, model: conf.model, apiKey: '', networkLocation: (conf.networkLocation === 'internal' ? 'internal' : 'external') as NetworkLocation });
    setEditingId(conf.id);
    setIsEditing(true);
    setTestStatus('idle');
    setTestError('');
    setAvailableModels([]);
    setShowModelDropdown(false);
    setModelLoadError('');
  };

  const handleNew = () => {
    setEditForm({ name: '新配置', provider: 'openai_compatible', baseUrl: PROVIDER_DEFAULT_BASE_URL.openai_compatible, apiKey: '', model: '', networkLocation: 'internal' as NetworkLocation });
    setEditingId(null);
    setIsEditing(true);
    setTestStatus('idle');
    setTestError('');
    setAvailableModels([]);
    setShowModelDropdown(false);
    setModelLoadError('');
  };

  // 切换提供商时自动填入默认 API 地址（仅当用户未自定义或新建时）
  const handleProviderChange = (provider: string) => {
    const defaultUrl = PROVIDER_DEFAULT_BASE_URL[provider as LlmProviderType] || '';
    const currentUrl = editForm?.baseUrl || '';
    // 如果当前 URL 为空，或等于某个提供商的默认值，则自动切换
    const isDefaultUrl = Object.values(PROVIDER_DEFAULT_BASE_URL).includes(currentUrl);
    setEditForm({
      ...editForm,
      provider,
      baseUrl: isDefaultUrl || !editingId ? defaultUrl : currentUrl,
    });
    setAvailableModels([]);
    setShowModelDropdown(false);
    setModelLoadError('');
  };

  // 获取模型列表（直接用表单数据，无需先保存）
  const handleFetchModels = async () => {
    if (!editForm.apiKey) {
      setModelLoadError('请先填写 API Key');
      return;
    }
    setIsLoadingModels(true);
    setModelLoadError('');
    try {
      const models = await llmConfigService.listModelsByParams(
        editForm.provider,
        editForm.baseUrl,
        editForm.apiKey,
        editForm.networkLocation,
      );
      setAvailableModels(models);
      setShowModelDropdown(true);
    } catch (e: any) {
      const errMsg = typeof e === 'string' ? e : (e?.message || JSON.stringify(e));
      setModelLoadError(errMsg);
    } finally {
      setIsLoadingModels(false);
    }
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
          networkLocation: editForm.networkLocation,
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
          networkLocation: editForm.networkLocation,
        };
        await llmConfigService.create(input);
      }
      await loadConfigs();
      setIsEditing(false);
    } catch (e: any) {
      if (e && typeof e === 'object' && typeof e.RuntimeRefreshError === 'string') {
        await loadConfigs();
        setIsEditing(false);
        setRuntimeRefreshError(e.RuntimeRefreshError);
      } else {
        const errMsg = typeof e === 'string' ? e : (e?.message || JSON.stringify(e));
        setSaveError(errMsg);
      }
    } finally {
      setIsSaving(false);
    }
  };

  const handleDelete = async (id: string) => {
    try {
      await llmConfigService.delete(id);
      await loadConfigs();
    } catch (e: any) {
      if (e && typeof e === 'object' && typeof e.RuntimeRefreshError === 'string') {
        await loadConfigs();
        setRuntimeRefreshError(e.RuntimeRefreshError);
      } else {
        const errMsg = typeof e === 'string' ? e : (e?.message || JSON.stringify(e));
        setSaveError(errMsg);
      }
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

  const toggleFormat = (fmt: ExportFormat) => {
    setSelectedFormats(prev =>
      prev.includes(fmt) ? prev.filter(f => f !== fmt) : [...prev, fmt]
    );
  };

  const handleExport = async () => {
    if (selectedFormats.length === 0) return;
    setIsExporting(true);
    setExportError('');
    setExportResult(null);
    try {
      const result = await dataService.dataExport(selectedFormats);
      // files 为空表示用户取消了目录选择，属正常操作，不显示成功或错误。
      if (result.files.length === 0) {
        return;
      }
      setExportResult(result);
      setShowFormatSelect(false);
    } catch (e: any) {
      const msg = typeof e === 'object' && e !== null
        ? (e.ValidationError || e.DbError || Object.values(e)[0] || '导出失败')
        : String(e);
      setExportError(typeof msg === 'string' ? msg : '导出失败');
    } finally {
      setIsExporting(false);
    }
  };

  const handleDestroy = async () => {
    setIsDestroying(true);
    setDestroyError('');
    try {
      await dataService.dataDestroy();
      onDataDestroyed?.();
    } catch (e: any) {
      const msg = typeof e === 'object' && e !== null
        ? (e.ValidationError || e.DbError || Object.values(e)[0] || '销毁失败')
        : String(e);
      setDestroyError(typeof msg === 'string' ? msg : '销毁失败');
    } finally {
      setIsDestroying(false);
    }
  };

  const handleImportSelect = async () => {
    setImportError('');
    setImportResult(null);
    try {
      const filePath = await dataService.pickImportFile();
      if (!filePath) return;
      setPendingImportPath(filePath);
      setShowImportConfirm(true);
    } catch (e: any) {
      const msg = typeof e === 'object' && e !== null
        ? (e.ValidationError || Object.values(e)[0] || '文件选择失败')
        : String(e);
      setImportError(typeof msg === 'string' ? msg : '文件选择失败');
    }
  };

  const handleImport = async () => {
    if (!pendingImportPath) return;
    setIsImporting(true);
    setImportError('');
    try {
      const result = await dataService.dataImport(pendingImportPath);
      setImportResult(result);
      setShowImportConfirm(false);
      onDataImported?.();
    } catch (e: any) {
      const msg = typeof e === 'object' && e !== null
        ? (e.ValidationError || e.DbError || Object.values(e)[0] || '导入失败')
        : String(e);
      setImportError(typeof msg === 'string' ? msg : '导入失败');
    } finally {
      setIsImporting(false);
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
      <div className="h-full bg-white dark:bg-slate-900 shadow-2xl flex">
        <div className="w-64 bg-slate-50 dark:bg-slate-800 border-r border-slate-200 dark:border-slate-700 p-6 flex flex-col gap-2 shrink-0">
          <div className="flex items-center justify-between mb-8 px-3">
            <h2 className="text-[16px] font-semibold text-slate-800 dark:text-slate-100">全局设置</h2>
          </div>
          <button onClick={() => { setTab('llm'); setIsEditing(false); }} className={cn("text-left px-3 py-2 rounded-lg text-[14px] font-medium transition-colors", tab === 'llm' ? "bg-white dark:bg-slate-900 text-indigo-600 shadow-sm border border-slate-200/60" : "text-slate-600 dark:text-slate-300 hover:bg-slate-200/50 dark:hover:bg-slate-700/50")}>模型服务配置</button>
          <button onClick={() => { setTab('mcp'); setIsEditing(false); setIsEditingMcp(false); }} className={cn("text-left px-3 py-2 rounded-lg text-[14px] font-medium transition-colors", tab === 'mcp' ? "bg-white dark:bg-slate-900 text-indigo-600 shadow-sm border border-slate-200/60" : "text-slate-600 dark:text-slate-300 hover:bg-slate-200/50 dark:hover:bg-slate-700/50")}>MCP Server</button>
          <button onClick={() => { setTab('scheduler'); setIsEditing(false); setIsEditingMcp(false); }} className={cn("text-left px-3 py-2 rounded-lg text-[14px] font-medium transition-colors", tab === 'scheduler' ? "bg-white dark:bg-slate-900 text-indigo-600 shadow-sm border border-slate-200/60" : "text-slate-600 dark:text-slate-300 hover:bg-slate-200/50 dark:hover:bg-slate-700/50")}>调度时间</button>
          <button onClick={() => { setTab('data'); setIsEditing(false); setIsEditingMcp(false); }} className={cn("text-left px-3 py-2 rounded-lg text-[14px] font-medium transition-colors", tab === 'data' ? "bg-white dark:bg-slate-900 text-indigo-600 shadow-sm border border-slate-200/60" : "text-slate-600 dark:text-slate-300 hover:bg-slate-200/50 dark:hover:bg-slate-700/50")}>数据与隐私</button>
        </div>
        <div className="flex-1 p-10 overflow-y-auto">
          <div className="flex justify-between items-center mb-8">
            <h3 className="text-[24px] font-semibold text-slate-800 dark:text-slate-100">{tab === 'llm' ? 'LLM Provider 配置' : tab === 'mcp' ? 'MCP Server配置' : tab === 'scheduler' ? '调度时间配置' : '数据与隐私'}</h3>
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
                if (tab === 'llm' && isEditing) {
                  setIsEditing(false);
                  return;
                }
                onClose();
              }}
              aria-label={tab === 'mcp' && isEditingMcp ? '返回 MCP 工具列表' : tab === 'llm' && isEditing ? '返回 LLM 配置列表' : '关闭全局设置'}
              className="p-2 text-slate-400 dark:text-slate-500 hover:bg-slate-100 dark:hover:bg-slate-700 rounded-full transition-colors"
            ><X size={24}/></button>
          </div>
          
          {tab === 'llm' && (
            <div className="space-y-6">
              <div className="bg-indigo-50 dark:bg-indigo-900/30 border border-indigo-100 dark:border-indigo-700 rounded-xl p-4 text-[13px] text-indigo-800 leading-relaxed">
                支持配置多个自定义大模型服务，你可以自由切换使用。
              </div>

              {!isEditing ? (
                <div className="space-y-4">
                  {configs.map((conf) => (
                    <div key={conf.id} className={cn("border rounded-xl p-4 transition-all", conf.isDefault ? "bg-indigo-50/30 dark:bg-indigo-900/30 border-indigo-200 dark:border-indigo-700 shadow-sm" : "bg-white dark:bg-slate-900 border-slate-200 dark:border-slate-700 hover:border-slate-300 dark:hover:border-slate-600")}>
                      <div className="flex items-center justify-between">
                        <div className="flex items-center gap-3">
                          <input type="radio" id={`conf-${conf.id}`} name="activeConfig" checked={conf.isDefault} onChange={() => handleSetDefault(conf.id)} className="w-4 h-4 text-indigo-600 accent-indigo-600" />
                          <label htmlFor={`conf-${conf.id}`} className="font-medium text-[15px] text-slate-800 dark:text-slate-100 cursor-pointer">{conf.name}</label>
                          {conf.isDefault && <span className="px-2 py-0.5 rounded text-[11px] font-medium bg-indigo-100 text-indigo-700">当前启用</span>}
                        </div>
                        <div className="flex items-center gap-2">
                          <button onClick={() => handleTestConnection(conf.id)} disabled={testingConfigId === conf.id} className="px-3 py-1.5 text-[13px] font-medium text-slate-600 dark:text-slate-300 hover:text-green-600 hover:bg-green-50 dark:hover:bg-green-900/30 rounded-lg transition-colors disabled:opacity-50">
                            {testingConfigId === conf.id ? <Loader2 size={14} className="animate-loading-spin" /> : '测试连接'}
                          </button>
                          <button onClick={() => handleEdit(conf)} className="px-3 py-1.5 text-[13px] font-medium text-slate-600 dark:text-slate-300 hover:text-indigo-600 hover:bg-indigo-50 dark:hover:bg-indigo-900/30 rounded-lg transition-colors">编辑</button>
                          <button onClick={() => handleDelete(conf.id)} className="px-3 py-1.5 text-[13px] font-medium text-slate-600 dark:text-slate-300 hover:text-red-600 hover:bg-red-50 dark:hover:bg-red-900/30 rounded-lg transition-colors">删除</button>
                        </div>
                      </div>
                      <div className="mt-3 pl-7 grid grid-cols-2 gap-y-2 text-[13px] text-slate-500 dark:text-slate-400">
                        <div><span className="text-slate-400 dark:text-slate-500 mr-2">模型:</span>{conf.model || '-'}</div>
                        <div><span className="text-slate-400 dark:text-slate-500 mr-2">提供商:</span>{PROVIDER_LABEL[conf.provider] || conf.provider}</div>
                        <div><span className="text-slate-400 dark:text-slate-500 mr-2">网络:</span>{conf.networkLocation === 'internal' ? '内网直连' : '外网代理'}</div>
                      </div>
                      {testStatus !== 'idle' && testingConfigId === null && lastTestedConfigId === conf.id && (
                        <div className={cn("mt-3 pl-7 text-[13px] flex items-center gap-1.5", testStatus === 'success' ? 'text-green-600' : testStatus === 'error' ? 'text-red-600' : 'text-slate-500 dark:text-slate-400')}>
                          {testStatus === 'success' && <><Check size={14} /> 连接成功</>}
                          {testStatus === 'error' && <><AlertCircle size={14} /> {testError}</>}
                        </div>
                      )}
                    </div>
                  ))}
                  <button onClick={handleNew} className="w-full py-4 border-2 border-dashed border-slate-200 dark:border-slate-700 rounded-xl text-[14px] font-medium text-slate-500 dark:text-slate-400 hover:border-indigo-300 dark:hover:border-indigo-600 hover:text-indigo-600 hover:bg-indigo-50/50 dark:hover:bg-indigo-900/30 transition-all flex items-center justify-center gap-2">
                    <Plus size={16} /> 添加新配置
                  </button>
                </div>
              ) : (
                <div className="bg-slate-50 dark:bg-slate-800 border border-slate-200 dark:border-slate-700 rounded-xl p-6 animate-in slide-in-from-bottom-2">
                  <div className="flex justify-between items-center mb-6">
                    <h4 className="text-[16px] font-medium text-slate-800 dark:text-slate-100">{editingId ? '编辑配置' : '新建配置'}</h4>
                  </div>
                  <div className="space-y-4">
                    <div>
                      <label className="block text-[13px] font-medium text-slate-700 dark:text-slate-300 mb-1.5">配置名称</label>
                      <input type="text" value={editForm.name} onChange={e => setEditForm({...editForm, name: e.target.value})} className="w-full bg-white dark:bg-slate-900 border border-slate-200 dark:border-slate-700 rounded-lg px-4 py-2.5 text-[14px] focus:ring-2 focus:ring-indigo-500/20 focus:border-indigo-500 outline-none" />
                    </div>
                    <div>
                      <label className="block text-[13px] font-medium text-slate-700 dark:text-slate-300 mb-1.5">提供商</label>
                      <select value={editForm.provider} onChange={e => handleProviderChange(e.target.value)} className="w-full bg-white dark:bg-slate-900 border border-slate-200 dark:border-slate-700 rounded-lg px-4 py-2.5 text-[14px] focus:ring-2 focus:ring-indigo-500/20 focus:border-indigo-500 outline-none">
                        <option value="openai_compatible">OpenAI兼容</option>
                        <option value="anthropic">Anthropic兼容</option>
                        <option value="minimax">MiniMax</option>
                        <option value="zhipu">智谱</option>
                        <option value="deepseek">Deepseek</option>
                        <option value="kimi">Kimi</option>
                        <option value="bailian">阿里云百炼</option>
                      </select>
                    </div>
                    <div>
                      <label className="block text-[13px] font-medium text-slate-700 dark:text-slate-300 mb-1.5">API地址</label>
                      <input type="text" value={editForm.baseUrl} onChange={e => setEditForm({...editForm, baseUrl: e.target.value})} className="w-full bg-white dark:bg-slate-900 border border-slate-200 dark:border-slate-700 rounded-lg px-4 py-2.5 text-[14px] focus:ring-2 focus:ring-indigo-500/20 focus:border-indigo-500 outline-none font-mono" />
                    </div>
                    <div>
                      <label className="block text-[13px] font-medium text-slate-700 dark:text-slate-300 mb-1.5">API 密钥{editingId ? ' (留空则不修改)' : ''}</label>
                      <input type="password" value={editForm.apiKey} onChange={e => setEditForm({...editForm, apiKey: e.target.value})} placeholder={editingId ? '••••••••' : 'sk-...'} className="w-full bg-white dark:bg-slate-900 border border-slate-200 dark:border-slate-700 rounded-lg px-4 py-2.5 text-[14px] focus:ring-2 focus:ring-indigo-500/20 focus:border-indigo-500 outline-none font-mono" />
                    </div>
                    <div>
                      <label className="block text-[13px] font-medium text-slate-700 dark:text-slate-300 mb-1.5">模型名称</label>
                      <div className="flex gap-2">
                        <input type="text" value={editForm.model} onChange={e => setEditForm({...editForm, model: e.target.value})} placeholder="gpt-4o" className="flex-1 bg-white dark:bg-slate-900 border border-slate-200 dark:border-slate-700 rounded-lg px-4 py-2.5 text-[14px] focus:ring-2 focus:ring-indigo-500/20 focus:border-indigo-500 outline-none font-mono" />
                        <button type="button" onClick={handleFetchModels} disabled={isLoadingModels} className="shrink-0 px-3 py-2.5 border border-slate-300 dark:border-slate-600 rounded-lg text-[13px] font-medium text-slate-700 dark:text-slate-300 hover:bg-slate-100 dark:hover:bg-slate-700 transition-colors disabled:opacity-50 whitespace-nowrap">
                          {isLoadingModels ? <Loader2 size={14} className="animate-loading-spin inline" /> : '获取模型列表'}
                        </button>
                      </div>
                      {modelLoadError && (
                        <div className="mt-1.5 text-[12px] text-red-600 flex items-center gap-1.5">
                          <AlertCircle size={12} /> {modelLoadError}
                        </div>
                      )}
                      {showModelDropdown && availableModels.length > 0 && (
                        <div className="mt-2 relative">
                          <select
                            value=""
                            onChange={e => {
                              if (e.target.value) {
                                setEditForm({...editForm, model: e.target.value});
                                setShowModelDropdown(false);
                              }
                            }}
                            className="w-full bg-white dark:bg-slate-900 border border-slate-200 dark:border-slate-700 rounded-lg px-4 py-2.5 text-[14px] focus:ring-2 focus:ring-indigo-500/20 focus:border-indigo-500 outline-none font-mono"
                          >
                            <option value="">-- 选择模型 ({availableModels.length} 个) --</option>
                            {availableModels.map(m => <option key={m} value={m}>{m}</option>)}
                          </select>
                        </div>
                      )}
                      {showModelDropdown && availableModels.length === 0 && (
                        <div className="mt-1.5 text-[12px] text-slate-500 dark:text-slate-400">未获取到可用模型</div>
                      )}
                    </div>
                    <div>
                      <label className="block text-[13px] font-medium text-slate-700 dark:text-slate-300 mb-1.5">网络位置</label>
                      <select value={editForm.networkLocation} onChange={e => setEditForm({...editForm, networkLocation: e.target.value as NetworkLocation})} className="w-full bg-white dark:bg-slate-900 border border-slate-200 dark:border-slate-700 rounded-lg px-4 py-2.5 text-[14px] focus:ring-2 focus:ring-indigo-500/20 focus:border-indigo-500 outline-none">
                        <option value="external">外网（走系统代理）</option>
                        <option value="internal">内网（直连，绕过代理）</option>
                      </select>
                    </div>
                  </div>
                  <div className="mt-6 flex justify-end gap-3">
                    <button onClick={() => setIsEditing(false)} className="px-5 py-2.5 border border-slate-300 dark:border-slate-600 rounded-lg text-[13px] font-medium text-slate-700 dark:text-slate-300 hover:bg-slate-100 dark:hover:bg-slate-700 transition-colors">取消</button>
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
              {mcpError && (
                <div className="rounded-lg border border-red-100 bg-red-50 px-3 py-2 text-[13px] text-red-600 flex items-center gap-2">
                  <AlertCircle size={14} /> {mcpError}
                </div>
              )}

              {!isEditingMcp ? (
                <div className="space-y-4">
                  {mcpServers.length === 0 ? (
                    <div className="rounded-xl border border-dashed border-slate-200 dark:border-slate-700 bg-slate-50 dark:bg-slate-800 px-4 py-8 text-center text-[13px] text-slate-400 dark:text-slate-500">暂无 MCP server</div>
                  ) : mcpServers.map(server => (
                    <div key={server.id} className="bg-white dark:bg-slate-900 border border-slate-200 dark:border-slate-700 rounded-xl p-4 shadow-sm">
                      <div className="flex items-start justify-between gap-4">
                        <div className="min-w-0">
                          <div className="flex flex-wrap items-center gap-2">
                            <span className={cn('w-2.5 h-2.5 rounded-full', server.enabled ? 'bg-emerald-500' : 'bg-slate-300')} />
                            <span className="text-[15px] font-medium text-slate-800 dark:text-slate-100 break-words">{server.name}</span>
                            <span className="rounded-full border border-slate-200 dark:border-slate-700 bg-slate-50 dark:bg-slate-800 px-2 py-0.5 text-[11px] font-medium text-slate-500 dark:text-slate-400">{mcpServerTypeLabel(server.serverType)}</span>
                            {!server.enabled && <span className="rounded-full border border-slate-200 dark:border-slate-700 bg-slate-50 dark:bg-slate-800 px-2 py-0.5 text-[11px] font-medium text-slate-500 dark:text-slate-400">已停用</span>}
                          </div>
                          <p className="mt-2 break-words font-mono text-[12px] text-slate-500 dark:text-slate-400">{server.commandOrUrl}</p>
                          <p className={cn('mt-1.5 break-words text-[12.5px] leading-relaxed', server.description ? 'text-slate-500 dark:text-slate-400' : 'text-slate-400 dark:text-slate-500')}>描述：{server.description || '暂无描述'}</p>
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
                                'absolute left-0.5 top-0.5 h-5 w-5 rounded-full bg-white dark:bg-slate-900 shadow-sm transition-transform',
                                server.enabled ? 'translate-x-5' : 'translate-x-0'
                              )}
                            />
                          </button>
                          <button onClick={() => handleTestMcp(server.id)} disabled={testingMcpId === server.id} className="px-3 py-1.5 text-[13px] font-medium text-slate-600 dark:text-slate-300 hover:text-green-600 hover:bg-green-50 dark:hover:bg-green-900/30 rounded-lg transition-colors disabled:opacity-50">
                            {testingMcpId === server.id ? <Loader2 size={14} className="animate-loading-spin" /> : '测试连接'}
                          </button>
                          <button onClick={() => handleEditMcp(server)} className="px-3 py-1.5 text-[13px] font-medium text-slate-600 dark:text-slate-300 hover:text-indigo-600 hover:bg-indigo-50 dark:hover:bg-indigo-900/30 rounded-lg transition-colors">编辑</button>
                          <button onClick={() => setPendingDeleteMcp(server)} className="px-3 py-1.5 text-[13px] font-medium text-slate-600 dark:text-slate-300 hover:text-red-600 hover:bg-red-50 dark:hover:bg-red-900/30 rounded-lg transition-colors">删除</button>
                        </div>
                      </div>
                    </div>
                  ))}
                  <button onClick={handleNewMcp} className="w-full py-4 border-2 border-dashed border-slate-200 dark:border-slate-700 rounded-xl text-[14px] font-medium text-slate-500 dark:text-slate-400 hover:border-indigo-300 dark:hover:border-indigo-600 hover:text-indigo-600 hover:bg-indigo-50/50 dark:hover:bg-indigo-900/30 transition-all flex items-center justify-center gap-2">
                    <Plus size={16} /> 添加 MCP server
                  </button>
                </div>
              ) : (
                <div className="bg-slate-50 dark:bg-slate-800 border border-slate-200 dark:border-slate-700 rounded-xl p-6 animate-in slide-in-from-bottom-2">
                  <h4 className="text-[16px] font-medium text-slate-800 dark:text-slate-100 mb-6">{editingMcpId ? '编辑 MCP server' : '新增 MCP server'}</h4>
                  {!editingMcpId && (
                    <div className="mb-6 rounded-xl border border-dashed border-slate-200 dark:border-slate-700 bg-white dark:bg-slate-900 p-4">
                      <label htmlFor="mcp-json-import" className="block text-[13px] font-medium text-slate-700 dark:text-slate-300 mb-1.5">MCP JSON</label>
                      <textarea
                        id="mcp-json-import"
                        aria-label="MCP JSON"
                        value={mcpImportJson}
                        onChange={e => setMcpImportJson(e.target.value)}
                        rows={4}
                        placeholder='{ "name": "天气查询", "type": "sse", "url": "https://example.com/mcp" }'
                        className="w-full bg-white dark:bg-slate-900 border border-slate-200 dark:border-slate-700 rounded-lg px-4 py-2.5 text-[14px] focus:ring-2 focus:ring-indigo-500/20 focus:border-indigo-500 outline-none font-mono resize-none"
                      />
                      <div className="mt-3 flex justify-end">
                        <button onClick={handleImportMcpJson} className="px-4 py-2 rounded-lg text-[13px] font-medium text-indigo-600 hover:bg-indigo-50 dark:hover:bg-indigo-900/30 border border-indigo-200 dark:border-indigo-700 transition-colors">导入 JSON</button>
                      </div>
                    </div>
                  )}
                  <div className="space-y-4">
                    <div>
                      <label className="block text-[13px] font-medium text-slate-700 dark:text-slate-300 mb-1.5">名称</label>
                      <input type="text" value={mcpForm.name} onChange={e => setMcpForm({ ...mcpForm, name: e.target.value })} className="w-full bg-white dark:bg-slate-900 border border-slate-200 dark:border-slate-700 rounded-lg px-4 py-2.5 text-[14px] focus:ring-2 focus:ring-indigo-500/20 focus:border-indigo-500 outline-none" />
                    </div>
                    <div>
                      <label className="block text-[13px] font-medium text-slate-700 dark:text-slate-300 mb-1.5">类型</label>
                      <select value={mcpForm.serverType} onChange={e => setMcpForm({ ...mcpForm, serverType: e.target.value as McpServerType })} className="w-full bg-white dark:bg-slate-900 border border-slate-200 dark:border-slate-700 rounded-lg px-4 py-2.5 text-[14px] focus:ring-2 focus:ring-indigo-500/20 focus:border-indigo-500 outline-none">
                        <option value="sse">SSE</option>
                        <option value="streamable_http">Streamable HTTP</option>
                        <option value="stdio">stdio</option>
                      </select>
                    </div>
                    <div>
                      <label className="block text-[13px] font-medium text-slate-700 dark:text-slate-300 mb-1.5">URL / Command</label>
                      <input type="text" value={mcpForm.commandOrUrl} onChange={e => setMcpForm({ ...mcpForm, commandOrUrl: e.target.value })} placeholder={isRemoteMcpType(mcpForm.serverType) ? 'http://localhost:8000/sse' : 'npx -y @modelcontextprotocol/server-filesystem'} className="w-full bg-white dark:bg-slate-900 border border-slate-200 dark:border-slate-700 rounded-lg px-4 py-2.5 text-[14px] focus:ring-2 focus:ring-indigo-500/20 focus:border-indigo-500 outline-none font-mono" />
                    </div>
                    <div>
                      <label className="block text-[13px] font-medium text-slate-700 dark:text-slate-300 mb-1.5">环境变量引用 JSON</label>
                      <textarea value={mcpForm.envRefs} onChange={e => setMcpForm({ ...mcpForm, envRefs: e.target.value })} rows={3} className="w-full bg-white dark:bg-slate-900 border border-slate-200 dark:border-slate-700 rounded-lg px-4 py-2.5 text-[14px] focus:ring-2 focus:ring-indigo-500/20 focus:border-indigo-500 outline-none font-mono resize-none" />
                      <p className="mt-1.5 text-[12px] text-slate-400 dark:text-slate-500">示例：{`{"TOKEN":"env:CALENDAR_TOKEN"}`}</p>
                    </div>
                    <div>
                      <label className="block text-[13px] font-medium text-slate-700 dark:text-slate-300 mb-1.5">描述（选填）</label>
                      <textarea value={mcpForm.description} onChange={e => setMcpForm({ ...mcpForm, description: e.target.value })} rows={2} className="w-full bg-white dark:bg-slate-900 border border-slate-200 dark:border-slate-700 rounded-lg px-4 py-2.5 text-[14px] focus:ring-2 focus:ring-indigo-500/20 focus:border-indigo-500 outline-none resize-none" />
                    </div>
                  </div>
                  <div className="mt-6 flex justify-end gap-3">
                    <button onClick={() => setIsEditingMcp(false)} className="px-5 py-2.5 border border-slate-300 dark:border-slate-600 rounded-lg text-[13px] font-medium text-slate-700 dark:text-slate-300 hover:bg-slate-100 dark:hover:bg-slate-700 transition-colors">取消</button>
                    <button onClick={handleSaveMcp} disabled={isSavingMcp} className="px-5 py-2.5 bg-indigo-600 text-white rounded-lg text-[13px] font-medium hover:bg-indigo-700 transition-colors shadow-sm disabled:opacity-50">
                      {isSavingMcp ? <Loader2 size={14} className="animate-loading-spin inline mr-1" /> : null}
                      保存 MCP server
                    </button>
                  </div>
                </div>
              )}

              {pendingDeleteMcp && (
                <div className="fixed inset-0 z-[60] flex items-center justify-center bg-slate-900/30 px-4">
                  <div className="w-full max-w-md rounded-2xl border border-slate-200 dark:border-slate-700 bg-white dark:bg-slate-900 p-6 shadow-2xl">
                    <h4 className="text-[16px] font-semibold text-slate-800 dark:text-slate-100">确认删除 MCP server？</h4>
                    <p className="mt-2 text-[13px] leading-relaxed text-slate-500 dark:text-slate-400">
                      删除后会从全局 MCP 工具列表移除「{pendingDeleteMcp.name}」，并解除所有角色绑定。
                    </p>
                    <div className="mt-6 flex justify-end gap-3">
                      <button onClick={() => setPendingDeleteMcp(null)} className="px-4 py-2 rounded-lg border border-slate-300 dark:border-slate-600 text-[13px] font-medium text-slate-700 dark:text-slate-300 hover:bg-slate-50 dark:hover:bg-slate-700 transition-colors">取消</button>
                      <button onClick={() => handleDeleteMcp(pendingDeleteMcp.id)} className="px-4 py-2 rounded-lg bg-red-600 text-[13px] font-medium text-white hover:bg-red-700 transition-colors">确认删除</button>
                    </div>
                  </div>
                </div>
              )}
            </div>
          )}

          {tab === 'scheduler' && (
            <div className="space-y-6">
              <div className="bg-indigo-50 dark:bg-indigo-900/30 border border-indigo-100 dark:border-indigo-700 rounded-xl p-4 text-[13px] text-indigo-800 leading-relaxed flex items-start gap-2">
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
                  <div key={level} className="bg-white dark:bg-slate-900 border border-slate-200 dark:border-slate-700 rounded-xl p-5 shadow-sm">
                    <div className="flex items-center gap-2 mb-4">
                      <span className="text-[15px] font-medium text-slate-800 dark:text-slate-100">
                        {level === 'moderate' ? '适度建议' : '积极主动'}
                      </span>
                      <span className="rounded-full border border-slate-200 dark:border-slate-700 bg-slate-50 dark:bg-slate-800 px-2 py-0.5 text-[11px] font-medium text-slate-500 dark:text-slate-400">
                        {schedulerTimes[level].length} 个时间点
                      </span>
                    </div>
                    <div className="flex flex-wrap items-center gap-2">
                      {schedulerTimes[level].map((time, index) => (
                        <div key={index} className="flex items-center gap-1 rounded-lg border border-slate-200 dark:border-slate-700 bg-white dark:bg-slate-900 px-2 py-1.5">
                          <input
                            type="time"
                            value={time}
                            onChange={e => handleSchedulerTimeChange(level, index, e.target.value)}
                            className="text-[13px] text-slate-700 dark:text-slate-300 outline-none bg-transparent"
                          />
                          <button
                            type="button"
                            aria-label={`删除 ${time}`}
                            onClick={() => handleRemoveSchedulerTime(level, index)}
                            className="text-slate-400 dark:text-slate-500 hover:text-red-500 transition-colors"
                          >
                            <X size={14} />
                          </button>
                        </div>
                      ))}
                      {schedulerTimes[level].length < 12 && (
                        <button
                          type="button"
                          onClick={() => handleAddSchedulerTime(level)}
                          className="inline-flex items-center gap-1 rounded-lg border border-dashed border-slate-300 dark:border-slate-600 bg-white dark:bg-slate-900 px-3 py-1.5 text-[12px] font-medium text-slate-500 dark:text-slate-400 hover:border-indigo-300 dark:hover:border-indigo-600 hover:text-indigo-600 transition-colors"
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

              <div className="pt-6 border-t border-slate-200 dark:border-slate-700">
                <div className="flex items-center justify-between">
                  <div className="flex items-start gap-3">
                    <div className="w-10 h-10 rounded-lg bg-indigo-50 dark:bg-indigo-900/30 flex items-center justify-center text-indigo-600 shrink-0">
                      <Bell size={18} />
                    </div>
                    <div>
                      <h4 className="text-[15px] font-medium text-slate-800 dark:text-slate-100">敲门通知声音</h4>
                      <p className="text-[13px] text-slate-500 dark:text-slate-400 mt-0.5">收到「敲门」级别通知时播放提示音</p>
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
                      "absolute top-0.5 left-0.5 w-5 h-5 bg-white dark:bg-slate-900 rounded-full shadow-sm transition-transform duration-200",
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
                <h4 className="text-[15px] font-medium text-slate-800 dark:text-slate-100 mb-2">导出数据</h4>
                <p className="text-[13px] text-slate-500 dark:text-slate-400 mb-4">将所有角色的记忆、任务和对话记录导出为标准格式。选择需要的格式后点击确认，系统会弹出文件夹选择对话框。</p>

                {exportError && (
                  <div className="mb-3 rounded-lg border border-red-100 bg-red-50 px-3 py-2 text-[13px] text-red-600 flex items-center gap-2">
                    <AlertCircle size={14} /> {exportError}
                  </div>
                )}

                {exportResult && (
                  <div className="mb-3 rounded-lg border border-green-100 bg-green-50 px-3 py-2 text-[13px] text-green-700">
                    <div className="flex items-center gap-2 mb-1">
                      <Check size={14} /> 导出完成
                    </div>
                    <ul className="ml-6 list-disc space-y-0.5">
                      {exportResult.files.map((f, i) => (
                        <li key={i} className="text-[12px] text-green-600 break-all">{f}</li>
                      ))}
                    </ul>
                  </div>
                )}

                {!showFormatSelect && !isExporting && (
                  <button
                    onClick={() => { setShowFormatSelect(true); setExportResult(null); setExportError(''); }}
                    className="flex items-center gap-2 px-5 py-2.5 border border-slate-300 dark:border-slate-600 rounded-lg text-[14px] font-medium text-slate-700 dark:text-slate-300 hover:bg-slate-50 dark:hover:bg-slate-700 transition-colors"
                  >
                    <Download size={16}/> 导出存档
                  </button>
                )}

                {showFormatSelect && !isExporting && (
                  <div className="rounded-xl border border-slate-200 dark:border-slate-700 bg-slate-50 dark:bg-slate-800 p-4 space-y-3">
                    <p className="text-[13px] font-medium text-slate-700 dark:text-slate-300">选择导出格式（可多选）</p>
                    <div className="space-y-2">
                      <label className="flex items-center gap-2 cursor-pointer">
                        <input
                          type="checkbox"
                          checked={selectedFormats.includes('sqlite')}
                          onChange={() => toggleFormat('sqlite')}
                          className="w-4 h-4 rounded border-slate-300 dark:border-slate-600 text-indigo-600 focus:ring-indigo-500"
                        />
                        <span className="text-[14px] text-slate-700 dark:text-slate-300">SQLite 备份 <span className="text-slate-400 dark:text-slate-500 text-[12px]">（推荐，可用于数据恢复）</span></span>
                      </label>
                      <label className="flex items-center gap-2 cursor-pointer">
                        <input
                          type="checkbox"
                          checked={selectedFormats.includes('json')}
                          onChange={() => toggleFormat('json')}
                          className="w-4 h-4 rounded border-slate-300 dark:border-slate-600 text-indigo-600 focus:ring-indigo-500"
                        />
                        <span className="text-[14px] text-slate-700 dark:text-slate-300">JSON 格式 <span className="text-slate-400 dark:text-slate-500 text-[12px]">（更适合跨版本数据迁移）</span></span>
                      </label>
                      <label className="flex items-center gap-2 cursor-pointer">
                        <input
                          type="checkbox"
                          checked={selectedFormats.includes('markdown')}
                          onChange={() => toggleFormat('markdown')}
                          className="w-4 h-4 rounded border-slate-300 dark:border-slate-600 text-indigo-600 focus:ring-indigo-500"
                        />
                        <span className="text-[14px] text-slate-700 dark:text-slate-300">Markdown 报告 <span className="text-slate-400 dark:text-slate-500 text-[12px]">（可读性高，不可用于数据恢复）</span></span>
                      </label>
                    </div>
                    <div className="flex items-center gap-2 pt-1">
                      <button
                        onClick={handleExport}
                        disabled={selectedFormats.length === 0}
                        className="px-4 py-2 rounded-lg text-[13px] font-medium text-white bg-indigo-600 hover:bg-indigo-700 transition-colors disabled:opacity-50 disabled:cursor-not-allowed"
                      >
                        确认导出
                      </button>
                      <button
                        onClick={() => { setShowFormatSelect(false); setSelectedFormats([]); }}
                        className="px-4 py-2 rounded-lg text-[13px] font-medium text-slate-600 dark:text-slate-300 hover:bg-slate-100 dark:hover:bg-slate-700 transition-colors"
                      >
                        取消
                      </button>
                    </div>
                  </div>
                )}

                {isExporting && (
                  <div className="flex items-center gap-2 px-5 py-2.5 text-[14px] text-slate-600 dark:text-slate-300">
                    <Loader2 size={16} className="animate-loading-spin" /> 导出中...
                  </div>
                )}
              </div>

              <div>
                <h4 className="text-[15px] font-medium text-slate-800 dark:text-slate-100 mb-2">导入数据</h4>
                <p className="text-[13px] text-slate-500 dark:text-slate-400 mb-4">导入 存档文件（.db 或 .json）恢复数据。导入前会自动备份当前数据。</p>

                {importError && (
                  <div className="mb-3 rounded-lg border border-red-100 bg-red-50 px-3 py-2 text-[13px] text-red-600 flex items-center gap-2">
                    <AlertCircle size={14} /> {importError}
                  </div>
                )}

                {importResult && (
                  <div className="mb-3 rounded-lg border border-green-100 bg-green-50 px-3 py-2 text-[13px] text-green-700">
                    <div className="flex items-center gap-2 mb-1">
                      <Check size={14} /> 导入完成
                    </div>
                    <p className="ml-6 text-[12px] text-green-600">
                      已恢复 {importResult.rolesCount} 个角色、{importResult.tasksCount} 个任务、{importResult.memoriesCount} 条记忆、{importResult.conversationsCount} 个对话、{importResult.messagesCount} 条消息
                    </p>
                  </div>
                )}

                {showImportConfirm && !isImporting && (
                  <div className="mb-3 rounded-xl border border-amber-200 bg-amber-50/50 p-4 space-y-3">
                    <p className="text-[13px] text-amber-700 leading-relaxed">
                      导入将覆盖当前所有数据（导入前已自动备份）。确认要继续吗？
                    </p>
                    <p className="text-[12px] text-slate-500 dark:text-slate-400 break-all">文件：{pendingImportPath}</p>
                    <div className="flex items-center gap-2 pt-1">
                      <button
                        onClick={handleImport}
                        className="px-4 py-2 rounded-lg text-[13px] font-medium text-white bg-indigo-600 hover:bg-indigo-700 transition-colors"
                      >
                        确认导入
                      </button>
                      <button
                        onClick={() => { setShowImportConfirm(false); setPendingImportPath(null); setImportError(''); }}
                        className="px-4 py-2 rounded-lg text-[13px] font-medium text-slate-600 dark:text-slate-300 hover:bg-slate-100 dark:hover:bg-slate-700 transition-colors"
                      >
                        取消
                      </button>
                    </div>
                  </div>
                )}

                {!showImportConfirm && !isImporting && (
                  <div className="space-y-2">
                    <p className="text-[12px] text-slate-400 dark:text-slate-500">支持 JSON 或 SQLite 存档文件；SQLite 导出含两个 .db 文件，选择其中任一即可自动导入全部数据。</p>
                    <button
                      onClick={handleImportSelect}
                      className="flex items-center gap-2 px-5 py-2.5 border border-slate-300 dark:border-slate-600 rounded-lg text-[14px] font-medium text-slate-700 dark:text-slate-300 hover:bg-slate-50 dark:hover:bg-slate-700 transition-colors"
                    >
                      <Upload size={16}/> 导入存档
                    </button>
                  </div>
                )}

                {isImporting && (
                  <div className="flex items-center gap-2 px-5 py-2.5 text-[14px] text-slate-600 dark:text-slate-300">
                    <Loader2 size={16} className="animate-loading-spin" /> 导入中...
                  </div>
                )}
              </div>

              <div className="pt-6 border-t border-slate-200 dark:border-slate-700">
                <h4 className="text-[15px] font-medium text-red-600 mb-2 flex items-center gap-2">危险区域</h4>
                <p className="text-[13px] text-slate-500 dark:text-slate-400 mb-4">永久销毁本地数据库中的所有数据。此操作不可逆！</p>

                {destroyError && (
                  <div className="mb-3 rounded-lg border border-red-100 bg-red-50 px-3 py-2 text-[13px] text-red-600 flex items-center gap-2">
                    <AlertCircle size={14} /> {destroyError}
                  </div>
                )}

                {!showDestroyConfirm && !isDestroying && (
                  <button
                    onClick={() => { setShowDestroyConfirm(true); setDestroyError(''); setDestroyConfirmText(''); }}
                    className="flex items-center gap-2 px-5 py-2.5 bg-red-50 border border-red-200 rounded-lg text-[14px] font-medium text-red-600 hover:bg-red-100 transition-colors"
                  >
                    <Trash2 size={16}/> 销毁所有数据
                  </button>
                )}

                {showDestroyConfirm && !isDestroying && (
                  <div className="rounded-xl border border-red-200 bg-red-50/50 p-4 space-y-3">
                    <p className="text-[13px] text-red-700 leading-relaxed">此操作将永久删除所有角色、记忆、任务和对话数据，且不可恢复。</p>
                    <div>
                      <input
                        type="text"
                        value={destroyConfirmText}
                        onChange={e => setDestroyConfirmText(e.target.value)}
                        placeholder='输入"确认销毁"以继续'
                        className="w-full bg-white dark:bg-slate-900 border border-red-200 rounded-lg px-4 py-2.5 text-[14px] focus:ring-2 focus:ring-red-500/20 focus:border-red-500 outline-none"
                      />
                    </div>
                    <div className="flex items-center gap-2 pt-1">
                      <button
                        onClick={handleDestroy}
                        disabled={destroyConfirmText !== '确认销毁'}
                        className="px-4 py-2 rounded-lg text-[13px] font-medium text-white bg-red-600 hover:bg-red-700 transition-colors disabled:opacity-50 disabled:cursor-not-allowed"
                      >
                        确认销毁
                      </button>
                      <button
                        onClick={() => { setShowDestroyConfirm(false); setDestroyConfirmText(''); setDestroyError(''); }}
                        className="px-4 py-2 rounded-lg text-[13px] font-medium text-slate-600 dark:text-slate-300 hover:bg-slate-100 dark:hover:bg-slate-700 transition-colors"
                      >
                        取消
                      </button>
                    </div>
                  </div>
                )}

                {isDestroying && (
                  <div className="flex items-center gap-2 px-5 py-2.5 text-[14px] text-red-600">
                    <Loader2 size={16} className="animate-loading-spin" /> 销毁中...
                  </div>
                )}
              </div>
            </div>
          )}
        </div>
      </div>

      {runtimeRefreshError && (
        <Modal onClose={() => setRuntimeRefreshError('')} width="w-[420px]" ariaLabel="配置已保存，但运行时刷新失败">
          <div className="p-6">
            <h2 className="text-[18px] font-semibold text-slate-800 dark:text-slate-100 flex items-center gap-2">
              <AlertCircle size={20} className="text-amber-500" /> 配置已保存，但运行时刷新失败
            </h2>
            <p className="mt-3 text-[13px] leading-6 text-slate-600 dark:text-slate-300 break-all">{runtimeRefreshError}</p>
            <p className="mt-2 text-[13px] text-slate-500 dark:text-slate-400">请重试或重启应用，使新配置生效。</p>
            <div className="mt-6 flex justify-end">
              <button onClick={() => setRuntimeRefreshError('')} className="px-5 py-2.5 bg-slate-800 text-white rounded-lg text-[13px] font-medium hover:bg-slate-700 transition-colors">
                知道了
              </button>
            </div>
          </div>
        </Modal>
      )}

      {saveError && (
        <Modal onClose={() => setSaveError('')} width="w-[420px]" ariaLabel="保存失败">
          <div className="p-6">
            <h2 className="text-[18px] font-semibold text-slate-800 dark:text-slate-100 flex items-center gap-2">
              <AlertCircle size={20} className="text-red-500" /> 保存失败
            </h2>
            <p className="mt-3 text-[13px] leading-6 text-slate-600 dark:text-slate-300 break-all">{saveError}</p>
            <div className="mt-6 flex justify-end">
              <button onClick={() => setSaveError('')} className="px-5 py-2.5 bg-slate-800 text-white rounded-lg text-[13px] font-medium hover:bg-slate-700 transition-colors">
                知道了
              </button>
            </div>
          </div>
        </Modal>
      )}
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
