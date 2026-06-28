import { existsSync, unlinkSync } from 'fs';
import { join } from 'path';
import { homedir } from 'os';

const isWindows = process.platform === 'win32';

function getAppDataDir(): string {
  if (isWindows) {
    return join(process.env.APPDATA || join(homedir(), 'AppData', 'Roaming'), 'com.egosync.app');
  }
  return join(homedir(), '.config', 'com.egosync.app');
}

export function cleanDatabase(): void {
  const appDataDir = getAppDataDir();
  const dbFiles = ['egosync.db', 'conversations.db'];
  for (const dbFile of dbFiles) {
    const dbPath = join(appDataDir, dbFile);
    if (existsSync(dbPath)) {
      unlinkSync(dbPath);
    }
  }
}
