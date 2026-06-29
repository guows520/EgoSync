/**
 * 60fps 动效静态审计脚本（Story 8.4 AC1）
 *
 * tauri-driver 无 CDP，无法获取 FPS。此脚本通过静态代码审计保证动画属性合规：
 * - 检查所有 animation keyframes 使用 transform/opacity（GPU 加速属性）
 * - 检查无 requestAnimationFrame 用于持续动画循环（允许一次性 scroll 定位）
 * - 检查无 setInterval 驱动的动画
 *
 * 这是 60fps 的必要条件；充分条件由本地人工 DevTools 验证补齐。
 */
import { readFileSync, existsSync, mkdirSync, writeFileSync, readdirSync, statSync } from 'fs';
import { join, resolve, dirname, relative } from 'path';
import { fileURLToPath } from 'url';

const __dirname = dirname(fileURLToPath(import.meta.url));
const srcRoot = resolve(__dirname, '..', '..', '..', 'src');
const reportsDir = resolve(__dirname, '..', 'reports', 'performance');

// 已知的例外：requestAnimationFrame 用于一次性 scroll 定位（非持续动画）
const RAF_EXCEPTIONS = [
  'ChatStream.tsx', // 一次性 scroll 定位（line 716-717）
];

// 已知的例外：setInterval 用于非动画目的（轮询/计时器）
const SETINTERVAL_EXCEPTIONS = [
  'OnboardingView.tsx', // 轮询 LLM 配置完成（非动画）
];

function findFiles(dir, ext, acc = []) {
  if (!existsSync(dir)) return acc;
  for (const entry of readdirSync(dir)) {
    const fullPath = join(dir, entry);
    const stat = statSync(fullPath);
    if (stat.isDirectory()) {
      findFiles(fullPath, ext, acc);
    } else if (entry.endsWith(ext)) {
      acc.push(fullPath);
    }
  }
  return acc;
}

function auditCssAnimations(cssContent, filePath) {
  const issues = [];
  const lines = cssContent.split('\n');

  // 检查 @keyframes 是否使用 transform/opacity
  const keyframePattern = /@keyframes\s+(\w+)\s*\{/g;
  let match;
  while ((match = keyframePattern.exec(cssContent)) !== null) {
    const keyframeName = match[1];
    const startIdx = match.index + match[0].length;
    // 找到对应的闭合括号
    let depth = 1;
    let endIdx = startIdx;
    for (let i = startIdx; i < cssContent.length && depth > 0; i++) {
      if (cssContent[i] === '{') depth++;
      if (cssContent[i] === '}') depth--;
      endIdx = i;
    }
    const keyframeBody = cssContent.slice(startIdx, endIdx);
    const lineOffset = cssContent.slice(0, match.index).split('\n').length;

    // 检查 keyframe 内是否有非 transform/opacity 的动画属性
    const animatedProps = keyframeBody.match(/(?:transform|opacity|rotate|scale|translate|filter|backdrop-filter)\s*:/g);
    const allProps = keyframeBody.match(/(\w[\w-]*)\s*:/g);
    if (allProps) {
      const nonGpuProps = allProps.filter(
        p => !['transform:', 'opacity:', 'rotate:', 'scale:', 'translate:', 'filter:', 'backdrop-filter:'].includes(p)
      );
      // 允许 offset-distance（motion path）和其他非布局属性
      // 但标记可能影响性能的布局属性
      const layoutProps = nonGpuProps.filter(p =>
        ['left:', 'top:', 'right:', 'bottom:', 'width:', 'height:', 'margin:', 'padding:', 'border-width:'].includes(p)
      );
      if (layoutProps.length > 0) {
        issues.push({
          file: relative(srcRoot, filePath),
          line: lineOffset,
          severity: 'warning',
          rule: 'keyframe-uses-layout-property',
          message: `@keyframes ${keyframeName} uses layout property(s): ${layoutProps.join(', ')} — may cause reflow, prefer transform/opacity`,
        });
      }
    }
  }

  return issues;
}

function auditTsxAnimations(content, filePath, fileName) {
  const issues = [];
  const lines = content.split('\n');

  // 检查 requestAnimationFrame 用于持续动画循环
  const rafPattern = /requestAnimationFrame/g;
  let match;
  while ((match = rafPattern.exec(content)) !== null) {
    const lineNum = content.slice(0, match.index).split('\n').length;
    const isException = RAF_EXCEPTIONS.includes(fileName);
    if (!isException) {
      // 检查是否在循环中（递归调用 rAF）
      const surrounding = content.slice(Math.max(0, match.index - 200), match.index + 200);
      const isRecursive = /requestAnimationFrame\s*\(/.test(surrounding) &&
        (surrounding.includes('function') || surrounding.includes('=>'));
      issues.push({
        file: relative(srcRoot, filePath),
        line: lineNum,
        severity: isRecursive ? 'error' : 'info',
        rule: 'requestAnimationFrame-usage',
        message: isRecursive
          ? 'requestAnimationFrame may be used in a recursive loop — verify it is one-shot, not a continuous animation'
          : 'requestAnimationFrame detected — verify it is not used for continuous animation loop',
      });
    }
  }

  // 检查 setInterval 驱动的动画
  const setIntervalPattern = /setInterval/g;
  while ((match = setIntervalPattern.exec(content)) !== null) {
    const lineNum = content.slice(0, match.index).split('\n').length;
    const isException = SETINTERVAL_EXCEPTIONS.includes(fileName);
    if (!isException) {
      issues.push({
        file: relative(srcRoot, filePath),
        line: lineNum,
        severity: 'error',
        rule: 'setInterval-animation',
        message: 'setInterval detected — verify it is not used for animation (use CSS animation or requestAnimationFrame instead)',
      });
    }
  }

  return issues;
}

// ── 主审计流程 ──

if (!existsSync(reportsDir)) {
  mkdirSync(reportsDir, { recursive: true });
}

const allIssues = [];
const filesAudited = [];

// 审计 CSS 文件
const cssFiles = findFiles(srcRoot, '.css');
for (const cssFile of cssFiles) {
  const content = readFileSync(cssFile, 'utf-8');
  filesAudited.push(relative(srcRoot, cssFile));
  allIssues.push(...auditCssAnimations(content, cssFile));
}

// 审计 TSX 文件
const tsxFiles = findFiles(srcRoot, '.tsx');
for (const tsxFile of tsxFiles) {
  const content = readFileSync(tsxFile, 'utf-8');
  const fileName = tsxFile.split(/[\\/]/).pop();
  filesAudited.push(relative(srcRoot, tsxFile));
  allIssues.push(...auditTsxAnimations(content, tsxFile, fileName));
}

const errors = allIssues.filter(i => i.severity === 'error');
const warnings = allIssues.filter(i => i.severity === 'warning');
const infos = allIssues.filter(i => i.severity === 'info');

const report = {
  timestamp: new Date().toISOString(),
  filesAudited: filesAudited.length,
  summary: {
    errors: errors.length,
    warnings: warnings.length,
    infos: infos.length,
  },
  issues: allIssues,
};

const reportPath = join(reportsDir, 'animation-audit.json');
writeFileSync(reportPath, JSON.stringify(report, null, 2));

console.log(`Animation audit complete: ${filesAudited.length} files audited`);
console.log(`  Errors: ${errors.length}, Warnings: ${warnings.length}, Info: ${infos.length}`);
console.log(`  Report: ${reportPath}`);

if (errors.length > 0) {
  console.error('\n❌ Animation audit found errors:');
  for (const err of errors) {
    console.error(`  [${err.rule}] ${err.file}:${err.line} — ${err.message}`);
  }
  // AC #6: 警告不阻断 — exit 0 让 CI 继续
  console.warn('\n⚠️  Animation audit errors detected, but not blocking CI (AC #6: warn not block).');
  process.exit(0);
} else {
  console.log('\n✅ Animation audit passed (no blocking errors).');
}
