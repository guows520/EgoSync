[ignoring loop detection]
# EgoSync 应用 Logo 设计提案

本提案为多角色自我协同平台 **EgoSync (GUI)** 专属定制。在应用开发和分发中，矢量图（SVG）非常适合做应用图标或多端渲染。鉴于远程图片生成系统暂时超载，我们精心手绘了 3 套极具现代感和视觉冲击力的纯 SVG 矢量 Logo，你可以通过 Markdown 预览直接欣赏其视觉效果，并直接拷贝至项目中使用。

---

## 方案一：多面人格棱镜 (The Prismatic Ego)

### 💡 设计理念
**EgoSync** 的核心在于管理我们面对不同社会、工作与个人场景的“多重自我（Roles）”。
本版本将自我的各个切片角色比作**色彩缤纷的透光几何棱镜**。在“管家”的梳理下，这些折射块在空间中精密拼接，形成一颗和谐完美且闪闪发光的晶体星。这象征着**多维人格在井然有序的架构下，走向自我的完整与统合（Ego-Integrity）**。

### 🎨 色彩与材质
* **基调**：高科技全息玻璃态（Prismatic Glassmorphism）
* **渐变**：靛蓝（Indigo）、碧绿（Teal）、幻彩紫（Violet），带有柔和的 3D 发光与折射阴影。

### 🖼️ 矢量展示 (SVG)

```xml
<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 200 200" width="200" height="200" style="background:#0F172A; border-radius:32px;">
  <!-- 背景径向渐变 -->
  <defs>
    <radialGradient id="bgGlow" cx="50%" cy="50%" r="50%">
      <stop offset="0%" stop-color="#4F46E5" stop-opacity="0.25"/>
      <stop offset="100%" stop-color="#0F172A" stop-opacity="0"/>
    </radialGradient>
    <!-- 多色棱镜层渐变 -->
    <linearGradient id="g1" x1="0%" y1="0%" x2="100%" y2="100%">
      <stop offset="0%" stop-color="#6366F1"/><stop offset="100%" stop-color="#4338CA"/>
    </linearGradient>
    <linearGradient id="g2" x1="100%" y1="0%" x2="0%" y2="100%">
      <stop offset="0%" stop-color="#10B981"/><stop offset="100%" stop-color="#047857"/>
    </linearGradient>
    <linearGradient id="g3" x1="0%" y1="100%" x2="100%" y2="0%">
      <stop offset="0%" stop-color="#A855F7"/><stop offset="100%" stop-color="#6D28D9"/>
    </linearGradient>
    <linearGradient id="g4" x1="100%" y1="100%" x2="0%" y2="0%">
      <stop offset="0%" stop-color="#F43F5E"/><stop offset="100%" stop-color="#BE123C"/>
    </linearGradient>
  </defs>
  <rect width="200" height="200" fill="#0F172A" rx="32"/>
  <circle cx="100" cy="100" r="100" fill="url(#bgGlow)"/>
  
  <!-- 晶体拼合 -->
  <g transform="translate(100,100) scale(1.1)">
    <!-- 上棱镜 -->
    <polygon points="0,-50 35,-15 -35,-15" fill="url(#g1)" opacity="0.95"/>
    <polygon points="0,-50 0,0 35,-15" fill="#FFF" opacity="0.15"/>
    <!-- 右棱镜 -->
    <polygon points="0,0 35,-15 50,30" fill="url(#g2)" opacity="0.9"/>
    <!-- 下棱镜 -->
    <polygon points="0,0 50,30 -50,30" fill="url(#g3)" opacity="0.85"/>
    <polygon points="0,50 50,30 -50,30" fill="url(#g3)" opacity="0.95"/>
    <!-- 左棱镜 -->
    <polygon points="0,0 -50,30 -35,-15" fill="url(#g4)" opacity="0.9"/>
    
    <!-- 反射高亮 -->
    <line x1="0" y1="-50" x2="0" y2="50" stroke="#FFF" stroke-opacity="0.3" stroke-width="1.5"/>
    <line x1="-50" y1="30" x2="50" y2="30" stroke="#FFF" stroke-opacity="0.2" stroke-width="1"/>
    <circle cx="0" cy="0" r="4" fill="#FFF"/>
  </g>
</svg>
```

---

## 方案二：无限轨与管家星 (Orbiting EgoSync)

### 💡 设计理念
本版本着意体现 **Sync（同步）和平衡**。我们将整个系统抽象为一条**流动循环的莫比乌斯环（以及无限符号 $\infty$）**，它代表心流、任务迭代和源源不断的能量流动。环带交织的中心，是一颗温暖且发光的星核（代表管家 Butler），源源不断地提供通知、晨报和保护。多重角色的彩色微粒如同卫星一般，在星轨上和谐流畅地同步自转，永无冲突。

### 🎨 色彩与材质
* **基调**：流光缎带、极简运动微光风格
* **渐变**：霓虹极光粉（Pink）、梦幻紫（Purple）到深邃海蓝（Cyan/Blue），在黑色玻璃衬底下溢彩流金。

### 🖼️ 矢量展示 (SVG)

```xml
<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 200 200" width="200" height="200" style="background:#0F172A; border-radius:32px;">
  <defs>
    <linearGradient id="orbitGrad" x1="0%" y1="0%" x2="100%" y2="100%">
      <stop offset="0%" stop-color="#EC4899" />
      <stop offset="50%" stop-color="#8B5CF6" />
      <stop offset="100%" stop-color="#3B82F6" />
    </linearGradient>
    <filter id="neonGlow">
      <feGaussianBlur stdDeviation="4" result="blur" />
      <feMerge>
        <feMergeNode in="blur" />
        <feMergeNode in="SourceGraphic" />
      </feMerge>
    </filter>
  </defs>
  <rect width="200" height="200" fill="#0F172A" rx="32"/>
  
  <g transform="translate(100,100)">
    <!-- 莫比乌斯环 / 无限符号 -->
    <path d="M-50,0 C-50,-25 -25,-25 0,0 C25,25 50,25 50,0 C50,-25 25,-25 0,0 C-25,25 -50,25 -50,0 Z" 
          fill="none" stroke="url(#orbitGrad)" stroke-width="8" stroke-linecap="round" stroke-linejoin="round" filter="url(#neonGlow)"/>
          
    <!-- 外部发光细线条 -->
    <path d="M-50,0 C-50,-25 -25,-25 0,0 C25,25 50,25 50,0 C50,-25 25,-25 0,0 C-25,25 -50,25 -50,0 Z" 
          fill="none" stroke="#FFFFFF" stroke-width="1.5" stroke-opacity="0.6"/>

    <!-- 中心管家智能星核 (Butler Core) -->
    <circle cx="0" cy="0" r="10" fill="#FFFFFF" filter="url(#neonGlow)"/>
    <circle cx="0" cy="0" r="4" fill="#8B5CF6"/>
    
    <!-- 代表不同角色(Roles)的运转微粒 -->
    <circle cx="-35" cy="-14" r="5" fill="#10B981" filter="url(#neonGlow)"/> <!-- 绿色角色 -->
    <circle cx="35" cy="14" r="5" fill="#F59E0B" filter="url(#neonGlow)"/>  <!-- 黄色角色 -->
  </g>
</svg>
```

---

## 方案三：对称双翼与神经网络 (Neural Butterfly)

### 💡 设计理念
**蝴蝶**象征自我的蜕变和人格的进化。我们将蝴蝶与**大脑神经网络的同步电波**结合，表达自我在认知上的高效协同。左翼和右翼由精细、对称的高对比声波和网络节点组成。中轴躯干是一尊高贵的沙漏，代表时间管理与精力分配；整体寓意科学规划大石头，让自我在飞升中保持最佳动态平衡。

### 🎨 色彩与材质
* **基调**：赛博朋克极简发光线、动态波形
* **渐变**：魅惑蓝紫（Electric Violet）、霓虹粉（Neon Pink）。

### 🖼️ 矢量展示 (SVG)

```xml
<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 200 200" width="200" height="200" style="background:#090D1A; border-radius:32px;">
  <defs>
    <linearGradient id="wingL" x1="100%" y1="50%" x2="0%" y2="50%">
      <stop offset="0%" stop-color="#8B5CF6" />
      <stop offset="100%" stop-color="#3B82F6" stop-opacity="0.3" />
    </linearGradient>
    <linearGradient id="wingR" x1="0%" y1="50%" x2="100%" y2="50%">
      <stop offset="0%" stop-color="#8B5CF6" />
      <stop offset="100%" stop-color="#EC4899" stop-opacity="0.3" />
    </linearGradient>
    <filter id="glowF">
      <feGaussianBlur stdDeviation="3" result="blur" />
      <feMerge>
        <feMergeNode in="blur" />
        <feMergeNode in="SourceGraphic" />
      </feMerge>
    </filter>
  </defs>
  <rect width="200" height="200" fill="#090D1A" rx="32"/>

  <g transform="translate(100,100)">
    <!-- 左翼 (Neural network) -->
    <path d="M-6,0 C-30,-40 -70,-30 -70,5 C-70,35 -40,40 -6,10" fill="url(#wingL)" opacity="0.6"/>
    <!-- 右翼 -->
    <path d="M6,0 C30,-40 70,-30 70,5 C70,35 40,40 6,10" fill="url(#wingR)" opacity="0.6"/>
    
    <!-- 装饰线条：几何声波脉冲 -->
    <path d="M-6,-20 C-40,-50 -65,-10 -6,0" fill="none" stroke="#60A5FA" stroke-width="1.5" stroke-dasharray="3,3"/>
    <path d="M6,-20 C40,-50 65,-10 6,0" fill="none" stroke="#F472B6" stroke-width="1.5" stroke-dasharray="3,3"/>
    
    <!-- 网络节点 -->
    <circle cx="-50" cy="-20" r="3" fill="#60A5FA" filter="url(#glowF)"/>
    <circle cx="-60" cy="15" r="4" fill="#3B82F6" filter="url(#glowF)"/>
    <circle cx="50" cy="-20" r="3" fill="#F472B6" filter="url(#glowF)"/>
    <circle cx="60" cy="15" r="4" fill="#EC4899" filter="url(#glowF)"/>

    <!-- 中轴：沙漏与主体 (Hourglass representing Butler & Time) -->
    <polygon points="-6,-30 6,-30 2,-3 -2,-3" fill="#FFFFFF"/>
    <polygon points="-2,3 2,3 6,30 -6,30" fill="#FFFFFF"/>
    <!-- 连接与发光核心 -->
    <line x1="0" y1="-30" x2="0" y2="30" stroke="#FFFFFF" stroke-width="1.5"/>
    <circle cx="0" cy="0" r="4" fill="#FFFFFF" filter="url(#glowF)"/>
  </g>
</svg>
```

---

## 🛠️ 下一步开发与测试规范

如确认选定，我们可以将首选的 Logo 保存至 `egosync-app/src/assets/logo.svg` 并自动更新相关的登录页或加载页组件，并通过以下命令确保测试正常：
```bash
npm run test:frontend
```
请向我们反馈您最中意哪一个版本的创意！包含：
1. **多面人格棱镜**（现代感、玻璃折射）
2. **无限轨与管家星**（平滑感、心流动力）
3. **神经网络双翼**（科技硬核、认知蜕变）
