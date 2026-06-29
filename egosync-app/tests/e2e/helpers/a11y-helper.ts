import { browser } from '@wdio/globals';
import { existsSync, mkdirSync, writeFileSync } from 'fs';
import { join, resolve, dirname } from 'path';
import { fileURLToPath } from 'url';

const __dirname = dirname(fileURLToPath(import.meta.url));

const reportsDir = resolve(__dirname, '..', 'reports', 'accessibility');

interface AxeViolation {
  id: string;
  impact: string | null;
  description: string;
  help: string;
  helpUrl: string;
  nodes: Array<{
    html: string;
    target: string[];
    failureSummary: string;
  }>;
}

interface AxeResults {
  violations: AxeViolation[];
  passes: Array<{ id: string; passCount: number }>;
  incomplete: Array<{ id: string; impact: string | null }>;
}

export async function runAxeScan(pageName: string): Promise<void> {
  if (!existsSync(reportsDir)) {
    mkdirSync(reportsDir, { recursive: true });
  }

  const axeBuilder = (await import('@axe-core/webdriverio')).default;
  const results = await axeBuilder({ client: browser })
    .disableRules(['color-contrast'])
    .analyze() as unknown as AxeResults;

  const criticalSerious = results.violations.filter(
    v => v.impact === 'critical' || v.impact === 'serious',
  );

  const report = {
    pageName,
    timestamp: new Date().toISOString(),
    violations: results.violations.map(v => ({
      ruleId: v.id,
      impact: v.impact,
      description: v.description,
      help: v.help,
      helpUrl: v.helpUrl,
      nodes: v.nodes.map(n => ({
        selector: n.target.join(' > '),
        html: n.html,
        failureSummary: n.failureSummary,
      })),
    })),
    passes: results.passes.map(p => ({ ruleId: p.id, passCount: p.passCount })),
    incomplete: results.incomplete.map(i => ({ ruleId: i.id, impact: i.impact })),
  };

  const reportPath = join(reportsDir, `${pageName}.json`);
  writeFileSync(reportPath, JSON.stringify(report, null, 2));

  if (criticalSerious.length > 0) {
    const summary = criticalSerious.map(v =>
      `  [${v.impact}] ${v.id}: ${v.help} (${v.nodes.length} node(s))`,
    ).join('\n');
    throw new Error(
      `axe scan "${pageName}" found ${criticalSerious.length} critical/serious violation(s):\n${summary}\nReport: ${reportPath}`,
    );
  }
}
