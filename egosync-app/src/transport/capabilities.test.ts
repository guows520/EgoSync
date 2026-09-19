// Story 15.5：TS 能力清单对等断言——「TS 清单 == 引擎注册表」。
//
// 运行时 fs 读 crates/egosync-engine/commands.json 工件，断言生成清单与
// 工件零漂移（任何单侧契约漂移在 CI 变红：引擎侧增删命令而未再生 TS
// 生成物 ⇒ 此处先红）+ 白名单机械不变量（⊆ web-ok ∧ 参数可缺省）。

import { readFileSync } from 'node:fs';
import * as nodePath from 'node:path';
import { describe, expect, it } from 'vitest';
import {
  DESKTOP_ONLY_COMMANDS,
  PERF_TEST_GATED_COMMANDS,
  REPLAY_WHITELIST,
  TRANSPORT_CAPABILITIES,
  WEB_OK_COMMANDS,
  isDesktopOnly,
  isWebCommand,
} from './capabilities';

// vitest 运行 cwd = egosync-app（jsdom 环境的全局 URL 被 vite 补丁劫持，
// import.meta.url 相对解析不可用——process.cwd 是稳定锚点）
const repoRoot = nodePath.resolve(process.cwd(), '..');

interface ArtifactParam {
  name: string;
  camelName: string;
  type: string;
  kind: 'ctx-injected' | 'client';
}

interface ArtifactCommand {
  name: string;
  capability: string;
  params: ArtifactParam[];
}

/** commands.json 工件（对等断言的机械事实源）。 */
const artifact = JSON.parse(
  readFileSync(`${repoRoot}/crates/egosync-engine/commands.json`, 'utf8')
) as {
  version: number;
  count: number;
  commands: ArtifactCommand[];
  replayWhitelist: string[];
};

describe('TS 能力清单 == 引擎注册表（commands.json 工件对等）', () => {
  it('WEB_OK_COMMANDS 全量等于工件 commands 名单', () => {
    const artifactNames = artifact.commands.map((c) => c.name);
    expect([...WEB_OK_COMMANDS].sort()).toEqual([...artifactNames].sort());
    expect(artifact.count).toBe(artifact.commands.length);
  });

  it('工件全部条目 capability 必须为 web-ok（desktop-only 物理排除）', () => {
    for (const command of artifact.commands) {
      expect(command.capability).toBe('web-ok');
    }
  });

  it('REPLAY_WHITELIST 等于工件顶层 replayWhitelist', () => {
    expect([...REPLAY_WHITELIST].sort()).toEqual([...artifact.replayWhitelist].sort());
    expect(REPLAY_WHITELIST.length).toBe(31);
  });

  it('desktop-only 与 perf-test 门控名单与 web-ok 互斥', () => {
    for (const name of DESKTOP_ONLY_COMMANDS) {
      expect(WEB_OK_COMMANDS).not.toContain(name);
    }
    for (const name of PERF_TEST_GATED_COMMANDS) {
      expect(WEB_OK_COMMANDS).not.toContain(name);
    }
  });

  it('isWebCommand / isDesktopOnly 判定与清单一致（全量循环）', () => {
    for (const name of WEB_OK_COMMANDS) {
      expect(isWebCommand(name)).toBe(true);
      expect(isDesktopOnly(name)).toBe(false);
    }
    for (const name of DESKTOP_ONLY_COMMANDS) {
      expect(isWebCommand(name)).toBe(false);
      expect(isDesktopOnly(name)).toBe(true);
    }
    // 未知命令两侧皆否（探针串非工件成员）
    expect(isWebCommand('no_such_command_xyz')).toBe(false);
    expect(isDesktopOnly('no_such_command_xyz')).toBe(false);
  });

  it('TRANSPORT_CAPABILITIES 对象与清单常量同源', () => {
    expect(TRANSPORT_CAPABILITIES.webCommands).toBe(WEB_OK_COMMANDS);
    expect(TRANSPORT_CAPABILITIES.desktopOnlyCommands).toBe(DESKTOP_ONLY_COMMANDS);
    expect(TRANSPORT_CAPABILITIES.perfTestGatedCommands).toBe(PERF_TEST_GATED_COMMANDS);
    expect(TRANSPORT_CAPABILITIES.replayWhitelist).toBe(REPLAY_WHITELIST);
    expect(TRANSPORT_CAPABILITIES.isWebCommand).toBe(isWebCommand);
    expect(TRANSPORT_CAPABILITIES.isDesktopOnly).toBe(isDesktopOnly);
  });
});

describe('重连重放白名单不变量（与 engine capabilities.rs 单测同口径）', () => {
  it('REPLAY_WHITELIST ⊆ WEB_OK_COMMANDS', () => {
    for (const name of REPLAY_WHITELIST) {
      expect(WEB_OK_COMMANDS).toContain(name);
    }
  });

  it('全部成员客户端参数可缺省（Option 或无客户端参数——`{}` 可重放）', () => {
    const byName = new Map(artifact.commands.map((c) => [c.name, c]));
    for (const name of REPLAY_WHITELIST) {
      const entry = byName.get(name);
      expect(entry).toBeDefined();
      for (const param of entry!.params) {
        if (param.kind === 'client') {
          expect(param.type.startsWith('Option<')).toBe(true);
        }
      }
    }
  });
});
