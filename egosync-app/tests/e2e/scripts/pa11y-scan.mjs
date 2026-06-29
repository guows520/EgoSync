import { existsSync, mkdirSync, writeFileSync } from 'fs';
import { join, resolve, dirname } from 'path';
import { fileURLToPath } from 'url';
import pa11y from 'pa11y';

const __dirname = dirname(fileURLToPath(import.meta.url));

const BASE_URL = process.env.PA11Y_BASE_URL || 'http://localhost:4173';
const PAGES = [
  { name: 'home', path: '/' },
];

const reportsDir = resolve(__dirname, '..', 'reports', 'accessibility');
if (!existsSync(reportsDir)) {
  mkdirSync(reportsDir, { recursive: true });
}

let failed = false;

for (const page of PAGES) {
  const url = `${BASE_URL}${page.path}`;
  const reportPath = join(reportsDir, `pa11y-${page.name}.json`);

  try {
    const results = await pa11y(url, {
      standard: 'WCAG2AA',
      runners: ['axe', 'htmlcs'],
      ignore: ['color-contrast'],
    });

    const criticalSerious = results.issues.filter(
      issue => issue.type === 'error' || issue.type === 'warning',
    );

    const report = {
      pageName: page.name,
      url,
      timestamp: new Date().toISOString(),
      issues: results.issues.map(issue => ({
        ruleId: issue.code,
        type: issue.type,
        impact: issue.type === 'error' ? 'serious' : issue.type === 'warning' ? 'moderate' : 'minor',
        message: issue.message,
        selector: issue.selector,
        context: issue.context,
      })),
    };

    writeFileSync(reportPath, JSON.stringify(report, null, 2));

    if (criticalSerious.length > 0) {
      failed = true;
      console.error(`pa11y "${page.name}" found ${criticalSerious.length} issue(s):`);
      for (const issue of criticalSerious) {
        console.error(`  [${issue.type}] ${issue.code}: ${issue.message} (${issue.selector})`);
      }
    } else {
      console.log(`pa11y "${page.name}" passed. Report: ${reportPath}`);
    }
  } catch (e) {
    failed = true;
    console.error(`pa11y "${page.name}" failed to run: ${e.message}`);
    writeFileSync(reportPath, JSON.stringify({
      pageName: page.name,
      url,
      timestamp: new Date().toISOString(),
      error: e.message,
    }, null, 2));
  }
}

if (failed) {
  process.exit(1);
}
