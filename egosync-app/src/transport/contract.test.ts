// Story 15.5 评审 G6：AppError 判别头契约串的跨语言耦合门禁。
//
// http.ts 与 routes.rs 各自持有 APP_ERROR_HEADER 常量字面量（「同源约定」
// 此前是纪律不是机制）——server 侧改名后双侧单元测试照绿、真实集成静默
// 断裂。本文件 source-scan 双侧源码断言常量值相等（与 events.rs 正则
// 解析进测试、parity_test 扫 lib.rs 同款范式）。
//
// 状态码与 body 形状的同构由 parity.test.ts（TS 侧）与 api_test.rs
// （server 侧）各自钉死；此处只钉「判别信号本身的契约串」。

import { readFileSync } from 'node:fs';
import { resolve } from 'node:path';
import { describe, expect, it } from 'vitest';

describe('AppError 判别头契约耦合（TS ⇄ server routes.rs）', () => {
  it("http.ts 的 APP_ERROR_HEADER 常量值 == routes.rs 的 APP_ERROR_HEADER 常量值", () => {
    const httpSource = readFileSync(
      resolve(process.cwd(), 'src/transport/http.ts'),
      'utf8'
    );
    const tsMatch = httpSource.match(/const APP_ERROR_HEADER = '([^']+)'/);
    expect(tsMatch, 'http.ts 应含 APP_ERROR_HEADER 字符串常量').not.toBeNull();

    const rustSource = readFileSync(
      resolve(process.cwd(), '..', 'server/src/routes.rs'),
      'utf8'
    );
    const rustMatch = rustSource.match(
      /pub const APP_ERROR_HEADER: &str = "([^"]+)"/
    );
    expect(rustMatch, 'routes.rs 应含 APP_ERROR_HEADER 常量（改名须双侧同步）').not.toBeNull();

    // fetch 的 headers.get 大小写不敏感；钉死双侧均为小写防大小写漂移
    expect(tsMatch![1]!.toLowerCase()).toBe(rustMatch![1]!.toLowerCase());
  });
});
