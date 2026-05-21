import { useState } from 'react';
import { Plus, X, Download, Trash2 } from 'lucide-react';
import { cn } from '../../lib/utils';

export function GlobalSettingsModal({ onClose }: any) {
  const [tab, setTab] = useState('llm');
  const [configs, setConfigs] = useState([
    { id: '1', name: 'OpenAI 官方', provider: 'OpenAI 兼容 (OpenAI, DeepSeek, Ollama...)', baseUrl: 'https://api.openai.com/v1', apiKey: 'sk-xxxxxxxx', model: 'gpt-4o' },
    { id: '2', name: '本地 Ollama', provider: 'OpenAI 兼容 (OpenAI, DeepSeek, Ollama...)', baseUrl: 'http://localhost:11434/v1', apiKey: 'none', model: 'llama3' }
  ]);
  const [activeId, setActiveId] = useState('1');
  const [isEditing, setIsEditing] = useState(false);
  const [editForm, setEditForm] = useState<any>(null);

  const handleEdit = (conf: any) => {
    setEditForm(conf);
    setIsEditing(true);
  };

  const handleNew = () => {
    setEditForm({ id: Date.now().toString(), name: '新配置', provider: 'OpenAI 兼容 (OpenAI, DeepSeek, Ollama...)', baseUrl: '', apiKey: '', model: '' });
    setIsEditing(true);
  };

  const handleSave = () => {
    if (configs.find(c => c.id === editForm.id)) {
      setConfigs(configs.map(c => c.id === editForm.id ? editForm : c));
    } else {
      setConfigs([...configs, editForm]);
    }
    setIsEditing(false);
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
                    <div key={conf.id} className={cn("border rounded-xl p-4 transition-all", activeId === conf.id ? "bg-indigo-50/30 border-indigo-200 shadow-sm" : "bg-white border-slate-200 hover:border-slate-300")}>
                      <div className="flex items-center justify-between">
                        <div className="flex items-center gap-3">
                          <input type="radio" id={`conf-${conf.id}`} name="activeConfig" checked={activeId === conf.id} onChange={() => setActiveId(conf.id)} className="w-4 h-4 text-indigo-600 accent-indigo-600" />
                          <label htmlFor={`conf-${conf.id}`} className="font-medium text-[15px] text-slate-800 cursor-pointer">{conf.name}</label>
                          {activeId === conf.id && <span className="px-2 py-0.5 rounded text-[11px] font-medium bg-indigo-100 text-indigo-700">当前启用</span>}
                        </div>
                        <div className="flex items-center gap-2">
                          <button onClick={() => handleEdit(conf)} className="px-3 py-1.5 text-[13px] font-medium text-slate-600 hover:text-indigo-600 hover:bg-indigo-50 rounded-lg transition-colors">编辑</button>
                          <button onClick={() => setConfigs(configs.filter(c => c.id !== conf.id))} className="px-3 py-1.5 text-[13px] font-medium text-slate-600 hover:text-red-600 hover:bg-red-50 rounded-lg transition-colors">删除</button>
                        </div>
                      </div>
                      <div className="mt-3 pl-7 grid grid-cols-2 gap-y-2 text-[13px] text-slate-500">
                        <div><span className="text-slate-400 mr-2">模型:</span>{conf.model || '-'}</div>
                        <div><span className="text-slate-400 mr-2">标准:</span>{conf.provider}</div>
                      </div>
                    </div>
                  ))}
                  <button onClick={handleNew} className="w-full py-4 border-2 border-dashed border-slate-200 rounded-xl text-[14px] font-medium text-slate-500 hover:border-indigo-300 hover:text-indigo-600 hover:bg-indigo-50/50 transition-all flex items-center justify-center gap-2">
                    <Plus size={16} /> 添加新配置
                  </button>
                </div>
              ) : (
                <div className="bg-slate-50 border border-slate-200 rounded-xl p-6 animate-in slide-in-from-bottom-2">
                  <div className="flex justify-between items-center mb-6">
                    <h4 className="text-[16px] font-medium text-slate-800">{editForm.id && configs.find(c=>c.id === editForm.id) ? '编辑配置' : '新建配置'}</h4>
                  </div>
                  <div className="space-y-4">
                    <div>
                      <label className="block text-[13px] font-medium text-slate-700 mb-1.5">配置名称</label>
                      <input type="text" value={editForm.name} onChange={e => setEditForm({...editForm, name: e.target.value})} className="w-full bg-white border border-slate-200 rounded-lg px-4 py-2.5 text-[14px] focus:ring-2 focus:ring-indigo-500/20 focus:border-indigo-500 outline-none" />
                    </div>
                    <div>
                      <label className="block text-[13px] font-medium text-slate-700 mb-1.5">Provider 标准</label>
                      <select value={editForm.provider} onChange={e => setEditForm({...editForm, provider: e.target.value})} className="w-full bg-white border border-slate-200 rounded-lg px-4 py-2.5 text-[14px] focus:ring-2 focus:ring-indigo-500/20 focus:border-indigo-500 outline-none">
                        <option>OpenAI 兼容 (OpenAI, DeepSeek, Ollama...)</option>
                        <option>Anthropic (Claude)</option>
                      </select>
                    </div>
                    <div>
                      <label className="block text-[13px] font-medium text-slate-700 mb-1.5">Base URL</label>
                      <input type="text" value={editForm.baseUrl} onChange={e => setEditForm({...editForm, baseUrl: e.target.value})} placeholder="https://api.openai.com/v1" className="w-full bg-white border border-slate-200 rounded-lg px-4 py-2.5 text-[14px] focus:ring-2 focus:ring-indigo-500/20 focus:border-indigo-500 outline-none font-mono" />
                    </div>
                    <div>
                      <label className="block text-[13px] font-medium text-slate-700 mb-1.5">API Key</label>
                      <input type="password" value={editForm.apiKey} onChange={e => setEditForm({...editForm, apiKey: e.target.value})} placeholder="sk-..." className="w-full bg-white border border-slate-200 rounded-lg px-4 py-2.5 text-[14px] focus:ring-2 focus:ring-indigo-500/20 focus:border-indigo-500 outline-none font-mono" />
                    </div>
                    <div>
                      <label className="block text-[13px] font-medium text-slate-700 mb-1.5">Model Name</label>
                      <input type="text" value={editForm.model} onChange={e => setEditForm({...editForm, model: e.target.value})} placeholder="gpt-4o" className="w-full bg-white border border-slate-200 rounded-lg px-4 py-2.5 text-[14px] focus:ring-2 focus:ring-indigo-500/20 focus:border-indigo-500 outline-none font-mono" />
                    </div>
                  </div>
                  <div className="mt-6 flex justify-end gap-3">
                    <button onClick={() => setIsEditing(false)} className="px-5 py-2.5 border border-slate-300 rounded-lg text-[13px] font-medium text-slate-700 hover:bg-slate-100 transition-colors">取消</button>
                    <button onClick={handleSave} className="px-5 py-2.5 bg-indigo-600 text-white rounded-lg text-[13px] font-medium hover:bg-indigo-700 transition-colors shadow-sm">保存配置</button>
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
