import { useState } from 'react';
import { X } from 'lucide-react';
import { cn } from '../../lib/utils';
import { Modal } from '../layout/Modal';
import { ICON_OPTIONS, COLOR_OPTIONS } from '../../constants/mockData';

export function AddRoleModal({ onClose, onAdd }: any) {
  const [name, setName] = useState('');
  const [selectedIcon, setSelectedIcon] = useState(0);
  const [selectedColor, setSelectedColor] = useState(0);

  const handleSubmit = () => {
    if (!name.trim()) return;
    const ic = ICON_OPTIONS[selectedIcon];
    const cl = COLOR_OPTIONS[selectedColor];
    onAdd({
      id: Date.now().toString(),
      name: name.trim(),
      icon: ic.icon,
      color: cl.color,
      text: cl.text,
      tint: cl.tint,
      energy: 50,
      status: 'none'
    });
  };

  return (
    <Modal onClose={onClose} width="w-[480px]">
      <div className="p-6">
        <div className="flex justify-between items-center mb-6">
          <h2 className="text-[18px] font-semibold text-slate-800">添加新角色</h2>
          <button onClick={onClose} className="text-slate-400 hover:text-slate-600"><X size={20}/></button>
        </div>
        
        <div className="space-y-5">
          <div>
            <label className="block text-[13px] font-medium text-slate-700 mb-1.5">角色名称</label>
            <input type="text" value={name} onChange={e => setName(e.target.value)} placeholder="例如：健身教练、投资者..." className="w-full bg-white border border-slate-200 rounded-lg px-4 py-2.5 text-[14px] focus:ring-2 focus:ring-indigo-500/20 focus:border-indigo-500 outline-none" />
          </div>

          <div>
            <label className="block text-[13px] font-medium text-slate-700 mb-2">选择图标</label>
            <div className="grid grid-cols-6 gap-2">
              {ICON_OPTIONS.map((opt, i) => (
                <button key={i} onClick={() => setSelectedIcon(i)} className={cn("w-full aspect-square rounded-xl flex items-center justify-center transition-all border-2", selectedIcon === i ? `${COLOR_OPTIONS[selectedColor].color} text-white border-transparent shadow-lg` : "border-slate-200 text-slate-500 hover:border-slate-300 hover:bg-slate-50")}>
                  <opt.icon size={20} />
                </button>
              ))}
            </div>
          </div>

          <div>
            <label className="block text-[13px] font-medium text-slate-700 mb-2">选择颜色</label>
            <div className="flex gap-3">
              {COLOR_OPTIONS.map((opt, i) => (
                <button key={i} onClick={() => setSelectedColor(i)} className={cn("w-10 h-10 rounded-full transition-all", opt.color, selectedColor === i ? "ring-4 ring-offset-2 " + opt.ring + " scale-110" : "opacity-70 hover:opacity-100")}>
                </button>
              ))}
            </div>
          </div>
        </div>

        <div className="mt-8 flex justify-end gap-3">
          <button onClick={onClose} className="px-5 py-2.5 border border-slate-200 text-slate-600 rounded-lg text-[13px] font-medium hover:bg-slate-50">取消</button>
          <button onClick={handleSubmit} className="px-5 py-2.5 bg-indigo-600 text-white rounded-lg text-[13px] font-medium hover:bg-indigo-700">创建角色</button>
        </div>
      </div>
    </Modal>
  );
}
