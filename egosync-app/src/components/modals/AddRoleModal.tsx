import { useState } from 'react';
import { X, AlertCircle } from 'lucide-react';
import { cn } from '../../lib/utils';
import { Modal } from '../layout/Modal';
import { ROLE_COLORS, ROLE_ICONS, DEFAULT_COLOR_HEX, DEFAULT_ICON_ID } from '../../lib/roleIcons';
import { roleService } from '../../services/roleService';
import type { Role } from '../../types/role';

interface AddRoleModalProps {
  onClose: () => void;
  onAdd: (role: Role) => void;
}

export function AddRoleModal({ onClose, onAdd }: AddRoleModalProps) {
  const [name, setName] = useState('');
  const [iconId, setIconId] = useState(DEFAULT_ICON_ID);
  const [colorHex, setColorHex] = useState(DEFAULT_COLOR_HEX);
  const [isCreating, setIsCreating] = useState(false);
  const [error, setError] = useState('');

  const handleSubmit = async () => {
    const trimmed = name.trim();
    if (!trimmed) {
      setError('角色名称不能为空');
      return;
    }
    setIsCreating(true);
    setError('');
    try {
      const role = await roleService.create({
        name: trimmed,
        icon: iconId,
        color: colorHex,
      });
      onAdd(role);
    } catch (e) {
      const text = typeof e === 'string' ? e : JSON.stringify(e);
      if (text.includes('角色名称不能为空')) setError('角色名称不能为空');
      else setError('创建失败，请稍后重试');
    } finally {
      setIsCreating(false);
    }
  };

  return (
    <Modal onClose={onClose} width="w-[480px]" ariaLabel="添加新角色">
      <div className="p-6">
        <div className="flex justify-between items-center mb-6">
          <h2 className="text-[18px] font-semibold text-slate-800">添加新角色</h2>
          <button onClick={onClose} className="text-slate-400 hover:text-slate-600"><X size={20}/></button>
        </div>
        
        <div className="space-y-5">
          <div>
            <label htmlFor="add-role-name" className="block text-[13px] font-medium text-slate-700 mb-1.5">角色名称</label>
            <input id="add-role-name" type="text" value={name} onChange={e => setName(e.target.value)} placeholder="例如：健身教练、投资者..." className="w-full bg-white border border-slate-200 rounded-lg px-4 py-2.5 text-[14px] focus:ring-2 focus:ring-indigo-500/20 focus:border-indigo-500 outline-none" />
          </div>

          <div>
            <label className="block text-[13px] font-medium text-slate-700 mb-2">选择图标</label>
            <div className="grid grid-cols-6 gap-2">
              {ROLE_ICONS.map(option => {
                const Icon = option.component;
                const isActive = iconId === option.id;
                return (
                  <button
                    key={option.id}
                    type="button"
                    title={option.label}
                    onClick={() => setIconId(option.id)}
                    className={cn("w-full aspect-square rounded-xl flex items-center justify-center transition-all border-2", isActive ? "text-white border-transparent shadow-lg" : "border-slate-200 text-slate-500 hover:border-slate-300 hover:bg-slate-50")}
                    style={isActive ? { backgroundColor: colorHex } : undefined}
                  >
                    <Icon size={20} />
                  </button>
                );
              })}
            </div>
          </div>

          <div>
            <label className="block text-[13px] font-medium text-slate-700 mb-2">选择颜色</label>
            <div className="flex gap-3 flex-wrap">
              {ROLE_COLORS.map(option => (
                <button
                  key={option.hex}
                  type="button"
                  title={option.label}
                  aria-label={option.label}
                  onClick={() => setColorHex(option.hex)}
                  className={cn("w-10 h-10 rounded-full transition-all", colorHex === option.hex ? "ring-4 ring-offset-2 ring-slate-400 scale-110" : "opacity-70 hover:opacity-100")}
                  style={{ backgroundColor: option.hex }}
                />
              ))}
            </div>
          </div>
        </div>

        {error && (
          <div className="mt-5 rounded-lg border border-red-100 bg-red-50 px-3 py-2 text-[13px] text-red-600 flex items-center gap-2">
            <AlertCircle size={14} /> {error}
          </div>
        )}

        <div className="mt-8 flex justify-end gap-3">
          <button onClick={onClose} className="px-5 py-2.5 border border-slate-200 text-slate-600 rounded-lg text-[13px] font-medium hover:bg-slate-50">取消</button>
          <button onClick={handleSubmit} disabled={isCreating} className="px-5 py-2.5 bg-indigo-600 text-white rounded-lg text-[13px] font-medium hover:bg-indigo-700 disabled:opacity-60">{isCreating ? '创建中...' : '创建角色'}</button>
        </div>
      </div>
    </Modal>
  );
}