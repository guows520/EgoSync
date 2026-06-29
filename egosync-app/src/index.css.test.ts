import { describe, it, expect } from 'vitest';
import { readFileSync } from 'fs';
import { join, dirname } from 'path';
import { fileURLToPath } from 'url';

const __dirname = dirname(fileURLToPath(import.meta.url));
const cssPath = join(__dirname, 'index.css');
const cssContent = readFileSync(cssPath, 'utf-8');

describe('index.css 无障碍修复', () => {
  it('不应全局移除 *:focus 的 outline', () => {
    expect(cssContent).not.toMatch(/\*:focus\s*\{[^}]*outline:\s*none/);
  });

  it('应使用 :focus:not(:focus-visible) 隐藏鼠标焦点', () => {
    expect(cssContent).toContain(':focus:not(:focus-visible)');
  });

  it('应有全局 :focus-visible 焦点环', () => {
    expect(cssContent).toContain(':focus-visible');
  });

  it('reduced-motion 不应保留 breathe 无限循环', () => {
    const reducedMotionSection = cssContent.match(/@media\s*\(prefers-reduced-motion:\s*reduce\)\s*\{([\s\S]*?)\}/);
    expect(reducedMotionSection).not.toBeNull();
    const section = reducedMotionSection![1];
    expect(section).not.toMatch(/breathe.*animation-iteration-count:\s*infinite/);
    expect(section).not.toMatch(/animate-bounce-forever.*animation-iteration-count:\s*infinite/);
    expect(section).not.toMatch(/animate-loading-spin.*infinite/);
  });

  it('reduced-motion 应关闭所有非必要动画', () => {
    const reducedMotionSection = cssContent.match(/@media\s*\(prefers-reduced-motion:\s*reduce\)\s*\{([\s\S]*?)\}/);
    expect(reducedMotionSection).not.toBeNull();
    const section = reducedMotionSection![1];
    expect(section).toContain('animation-iteration-count: 1');
    expect(section).toContain('animation-duration: 0.01ms');
  });
});
