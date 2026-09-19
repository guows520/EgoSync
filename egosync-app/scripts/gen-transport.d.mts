// Story 15.5：scripts/gen-transport.mjs 的类型声明（供 vitest 单一解析器同源
// import——测试与生成器共用同一 events.rs 解析逻辑）。
// 实现（零依赖 Node 脚本）见同目录 gen-transport.mjs。

/** events.rs 事件常量条目（常量名 + 事件名）。 */
export interface ParsedEventEntry {
  constName: string;
  eventName: string;
}

/** 解析 events.rs 的事件名常量（`pub const *_EVENT: &str = "..."`，保持声明序）。 */
export function parseEngineEvents(source: string): ParsedEventEntry[];

/** 解析 capabilities.rs 的 `pub const NAME: &[&str] = &[...]` 字符串数组常量。 */
export function parseConstList(source: string, constName: string): string[];

/** 读 commands.json 工件：web-ok 命令名 + replayWhitelist。 */
export function readCommandsArtifact(jsonPath?: string): {
  commands: string[];
  replayWhitelist: string[];
};

/** 生成 capabilities.ts 源文本（确定性输出）。 */
export function generateCapabilitiesModule(input: {
  commands: string[];
  desktopOnly: string[];
  perfTestGated: string[];
  replayWhitelist: string[];
}): string;

/** 生成 events.ts 源文本（确定性输出）。 */
export function generateEventsModule(entries: ParsedEventEntry[]): string;
