//! 单用户认证（Story 15.4，架构决策 #5）：env/首访 setup、Argon2id、
//! httpOnly Cookie 会话、401 中间件、IP 滑动窗口限流。
//!
//! Story 16.3：叠加 Bearer 通道——桌面远程客户端（webview 跨源，Cookie
//! 无法携带）经 `Authorization: Bearer <token>` 认证，校验引导主令牌
//! （env 常时比对 / 库态 Argon2id），**非会话表**；Cookie 路径零变化
//! （无 Authorization 头的请求行为与 15.4 逐语义一致）。
//!
//! [T5 修订]（人工裁决 2026-09-21）Bearer 失败面限流：`require_auth` 的
//! Bearer 验证失败按 IP 计数（**仅计失败**——成功访问不计数、不限流，
//! 令牌正确者不受他人失败影响），阈值/窗口沿用登录面口径（5 次/分钟/IP
//! 滑动窗口，超限 429 统一形状）；计数器 [`AppState::bearer_rate`] 独立
//! 于 auth 路由限流器；`/api/cmd/*` 与 `/api/events/ticket` 均覆盖，
//! SSE 流端点（`require_auth_with_sse_ticket` 的 Bearer 分支）同计数器
//! ——同一令牌空间的失败面不因换端点而旁路（与登录面防盗器对等的
//! 防爆破姿态，Argon2id 态每试还烧 CPU）。
//!
//! 初始化优先级冻结：
//! - env 存在 `EGOSYNC_TOKEN` ⇒ `/api/setup` 不挂载（请求 404）、login 仅
//!   常时比对 env；
//! - env 不存在 ⇒ 以库内 Argon2id 哈希为准（首访 `/api/setup` 写入
//!   `app_settings` kv `server_token_hash`；成功后本进程随即 404）；
//! - 两态切换须重启进程；已发 Cookie 不随切换失效（`auth_sessions` 表
//!   持久化，验证只看会话表——不关心引导凭据形态）；
//! - 无任何「任一通过即可」fail-open 组合读法：login 分支互斥。
//!
//! 会话设计（架构未指定会话存储，设计裁量）：login 校验一次 → 换发随机
//! 会话令牌（uuid v4）→ SHA256 哈希入库；每请求校验 = 主键查找 + 哈希
//! 比对（亚毫秒；Argon2 仅 login 执行）。

use std::collections::{HashMap, VecDeque};
use std::net::{IpAddr, SocketAddr};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use argon2::password_hash::{PasswordHash, PasswordHasher, PasswordVerifier, SaltString};
use argon2::Argon2;
use axum::body::Bytes;
use axum::extract::{ConnectInfo, Request, State};
use axum::http::{header, HeaderMap, HeaderValue, StatusCode};
use axum::middleware::Next;
use axum::response::{IntoResponse, Json, Response};
use egosync_engine::db::app_settings;
use egosync_engine::db::pool::DbPool;
use egosync_engine::error::AppError;
use serde_json::json;
use sha2::{Digest, Sha256};
use subtle::ConstantTimeEq;
use uuid::Uuid;

use crate::AppState;

/// 会话 Cookie 名。
pub const SESSION_COOKIE: &str = "egosync_session";
/// 令牌哈希在 app_settings kv 的键（令牌本体哈希按架构数据边界表落点）。
///
/// 单一事实源：engine `app_settings` 的保留键名单（评审修复 #6——
/// app_get_setting/app_set_setting 命中保留键拒绝读/写，防离线爆破
/// 与库态凭据覆写）。
pub const TOKEN_HASH_SETTING_KEY: &str =
    egosync_engine::db::app_settings::RESERVED_SETTING_KEYS[0];
/// 限流窗口（I/O 矩阵：5 次/分钟/IP）。
pub const RATE_LIMIT_WINDOW: Duration = Duration::from_secs(60);
/// 窗口内上限。
pub const RATE_LIMIT_MAX: usize = 5;
/// setup 令牌最短长度（首访引导下限；env 令牌为运维自担，不校验）。
pub const SETUP_TOKEN_MIN_LEN: usize = 8;

/// 认证态：引导凭据形态 + setup 可用位 + 会话表 + 限流器。
pub struct AuthState {
    /// env 态令牌（`EGOSYNC_TOKEN` 原值；None ⇒ 库态 Argon2id）。
    pub env_token: Option<String>,
    /// `/api/setup` 是否可用（env 存在或库哈希已写入 ⇒ false）。
    /// 首访 setup 成功后置 false（随即 404）；两态切换须重启进程。
    pub setup_available: AtomicBool,
    /// setup 串行化锁（check-then-write 原子化：锁内重检-写入-翻转，
    /// 防并发双 setup 后写覆盖前令牌）。
    pub setup_lock: tokio::sync::Mutex<()>,
    /// 会话表所在主库。
    pub pool: DbPool,
    /// 认证面限流器（`/api/auth/*` + `/api/setup`）。
    pub rate: RateLimiter,
}

impl AuthState {
    /// 启动期装配：读 env + 查库内哈希，决定引导形态与 setup 可用位。
    ///
    /// 库哈希读失败 ⇒ `Err`（沿 bootstrap 上抛拒启）——绝不把 DB 错误
    /// 吞成「无哈希」误挂 /api/setup（fail-open：已初始化实例的凭据
    /// 将可被覆写）。
    pub async fn new(pool: DbPool, env_token: Option<String>) -> Result<Self, String> {
        let setup_available = if env_token.is_some() {
            false
        } else {
            !db_has_token_hash(&pool).await?
        };
        Ok(Self {
            env_token,
            setup_available: AtomicBool::new(setup_available),
            setup_lock: tokio::sync::Mutex::new(()),
            pool,
            rate: RateLimiter::default(),
        })
    }

    pub fn setup_available(&self) -> bool {
        self.setup_available.load(Ordering::SeqCst)
    }
}

/// IP 滑动窗口限流器（内存计数；单实例单用户量级，不引 tower-governor）。
pub struct RateLimiter {
    inner: Mutex<HashMap<IpAddr, VecDeque<Instant>>>,
}

impl Default for RateLimiter {
    fn default() -> Self {
        Self {
            inner: Mutex::new(HashMap::new()),
        }
    }
}

impl RateLimiter {
    /// 记录一次请求并判定是否放行（窗口外记录剔除；超限 ⇒ false）。
    ///
    /// 窗口清空后驱逐该 IP 条目——防扫描源 IP 使 HashMap 无限增长
    /// （评审修复 #5：条目泄漏与键空间同阶）。
    fn check(&self, ip: IpAddr) -> bool {
        let now = Instant::now();
        let mut inner = self.inner.lock().expect("限流器锁中毒");
        let Some(window) = inner.get_mut(&ip) else {
            inner.insert(ip, VecDeque::from([now]));
            return true;
        };
        while window
            .front()
            .is_some_and(|t| now.duration_since(*t) > RATE_LIMIT_WINDOW)
        {
            window.pop_front();
        }
        if window.is_empty() {
            // 窗口已过期清空：驱逐条目（下次请求重新插入）
            inner.remove(&ip);
            inner.insert(ip, VecDeque::from([now]));
            return true;
        }
        if window.len() >= RATE_LIMIT_MAX {
            return false;
        }
        window.push_back(now);
        true
    }
}

/// 限流中间件：`/api/auth/*` 与 `/api/setup` 按 IP 5 次/分钟（超 ⇒ 429）。
///
/// 连接信息载体为 [`crate::idle_timeout::RemoteAddr`]（自定义 Listener 的
/// `Connected` 实现需本地类型——见其文档）。
pub async fn rate_limit(
    ConnectInfo(addr): ConnectInfo<crate::idle_timeout::RemoteAddr>,
    State(state): State<Arc<AppState>>,
    req: Request,
    next: Next,
) -> Response {
    if !state.auth.rate.check(addr.0.ip()) {
        // 二轮评审修复 #6：限流拒绝对运维可见（不含任何用户输入——
        // 冻结只禁密钥/载荷入日志，不禁事件）
        tracing::warn!(ip = %addr.0.ip(), "认证面限流拒绝（5 次/分钟/IP）");
        return too_many_requests();
    }
    next.run(req).await
}

/// `GET /api/auth/status`：认证前发现端点（setupRequired / authenticated）。
///
/// setupRequired = 无 env 且库内无哈希（首访引导）；authenticated = 当前
/// Cookie 会话有效 **或** Bearer 主令牌有效（16.3 叠加通道——桌面远程
/// 客户端的连接测试/引导探测走此分支；200 形状不变，不泄露存在性之外
/// 的信息）。
pub async fn auth_status(State(state): State<Arc<AppState>>, req: Request) -> Response {
    // 注意：Body 为 !Sync——Request 只能按值跨 await（引用会使 future !Send）
    let session_token = extract_session_token(&req);
    let bearer_token = extract_bearer_token(&req);
    let authenticated = if let Some(bearer) = bearer_token.as_deref() {
        // Bearer 提供即以 Bearer 判定（不回落 Cookie——客户端显式出示
        // 凭据时结果须确定，免 fail-open 组合读法）
        verify_primary_token(&state, bearer).await
    } else {
        session_cookie_valid(&state, session_token.as_deref()).await
    };
    (
        StatusCode::OK,
        Json(json!({
            "setupRequired": state.auth.setup_available(),
            "authenticated": authenticated,
        })),
    )
        .into_response()
}

/// `POST /api/setup {token}`：首访初始化（仅库态首访可写）。
///
/// env 态：路由不挂载（404）；已初始化：随即 404；成功后 setup_available
/// 置 false。空令牌 → 401 统一形状（错误白名单冻结口径：非 200 仅
/// 401/429/404/5xx 四类，认证失败一律 401 不泄露存在性）。
///
/// 并发防护：`setup_lock` 串行化——锁内重检 setup_available 再写入再
/// 翻转，并发双发只有首个成功（后到者 404，不覆盖已写入令牌）。
/// 令牌下限：trim 后最短 8 字符（首访引导凭据强度下限；env 令牌为
/// 运维自担，不在此校验）。
pub async fn setup(State(state): State<Arc<AppState>>, body: Bytes) -> Response {
    if !state.auth.setup_available() {
        return (
            StatusCode::NOT_FOUND,
            Json(json!({"error": "not found"})),
        )
            .into_response();
    }

    let token = match parse_token_body(&body) {
        Some(t) => t.trim().to_string(),
        _ => return unauthorized(),
    };
    if token.len() < SETUP_TOKEN_MIN_LEN {
        // 过短令牌视同认证失败（401 统一形状——白名单口径）
        return unauthorized();
    }

    // 串行化：锁内重检-写入-翻转（check-then-write 原子化）
    let _guard = state.auth.setup_lock.lock().await;
    if !state.auth.setup_available() {
        // 并发双发的后到者：已被首个请求初始化 ⇒ 404（不覆盖）
        return (
            StatusCode::NOT_FOUND,
            Json(json!({"error": "not found"})),
        )
            .into_response();
    }

    let phc = match hash_token_argon2id(&token) {
        Ok(phc) => phc,
        Err(e) => {
            tracing::error!("Argon2id 哈希失败: {}", e);
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({"error": "internal server error"})),
            )
                .into_response();
        }
    };

    if let Err(e) = app_settings::set_setting(&state.auth.pool, TOKEN_HASH_SETTING_KEY, &phc).await
    {
        tracing::error!("令牌哈希写入失败: {}", e);
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({"error": "internal server error"})),
        )
            .into_response();
    }

    // 路由随即不再可用（后续请求 404；不依赖重启）
    state.auth.setup_available.store(false, Ordering::SeqCst);
    tracing::info!("单用户令牌已初始化（Argon2id 哈希落 app_settings）");
    (StatusCode::OK, Json(json!({"ok": true}))).into_response()
}

/// `POST /api/auth/login {token}`：校验引导凭据，换发会话 Cookie。
///
/// 统一 401（不泄露存在性）：env 态常时比对（SHA256 摘要常时比较，长度
/// 不敏感）；库态 Argon2id 校验。成功 ⇒ Set-Cookie（httpOnly SameSite=
/// Strict + Max-Age=2592000——30 天持久会话，boss 2026-09-20 裁决；关闭
/// 浏览器不再登出是该裁决接受的体验取舍，TTL 清扫/多端吊销归 17.x）；
/// 反代 TLS 面（BEHIND_PROXY=1 且 XFP=https）追加 `Secure`（17.1）。
/// body 反序列化失败同样 401。
pub async fn login(
    ConnectInfo(addr): ConnectInfo<crate::idle_timeout::RemoteAddr>,
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    body: Bytes,
) -> Response {
    let Some(token) = parse_token_body(&body) else {
        return unauthorized();
    };

    let verified = match state.auth.env_token.as_deref() {
        // env 态：仅常时比对 env（库内哈希忽略——优先级冻结，无 fail-open）
        Some(env_token) => constant_time_eq_token(&token, env_token),
        // 库态：Argon2id 校验（哈希缺失 ⇒ 未初始化 ⇒ 拒绝）
        None => match app_settings::get_setting(&state.auth.pool, TOKEN_HASH_SETTING_KEY).await {
            Ok(Some(phc)) => verify_token_argon2id(&token, &phc),
            _ => false,
        },
    };

    if !verified {
        // 二轮评审修复 #6：失败登录对运维可见（公网单用户服务器的爆破
        // 尝试）——只记来源 IP，不含令牌或任何用户输入（冻结只禁密钥/
        // 载荷入日志，不禁事件）
        tracing::warn!(ip = %addr.0.ip(), "登录失败（令牌不匹配）");
        return unauthorized();
    }

    // 会话令牌：uuid v4（122 bit 熵）；SHA256 哈希入库（防库泄露后劫持）
    let session_token = Uuid::new_v4().to_string();
    let token_hash = sha256_hex(session_token.as_bytes());
    let inserted = sqlx::query("INSERT INTO auth_sessions (token_hash) VALUES (?1)")
        .bind(&token_hash)
        .execute(&state.auth.pool)
        .await
        .map(|_| true)
        .unwrap_or(false);
    if !inserted {
        tracing::error!("会话写入失败");
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({"error": "internal server error"})),
        )
            .into_response();
    }

    tracing::info!("单用户会话建立");
    // Max-Age=2592000（30 天持久会话）：boss 2026-09-20 裁决——auth_sessions
    // 行本就落库跨重启有效，30 天内重开浏览器仍保持登录。
    // Story 17.1：反代 TLS 面（BEHIND_PROXY=1 且 XFP=https）追加 Secure。
    let cookie = format!(
        "{}={}; Path=/; HttpOnly; SameSite=Strict; Max-Age=2592000{}",
        SESSION_COOKIE,
        session_token,
        cookie_secure_flag(state.behind_proxy, &headers)
    );
    let mut response = (StatusCode::OK, Json(json!({"ok": true}))).into_response();
    response.headers_mut().insert(
        header::SET_COOKIE,
        HeaderValue::from_str(&cookie).expect("会话 Cookie 头构造失败"),
    );
    response
}

/// `POST /api/auth/logout`（Story 16.1）：登出——删会话行 + 过期 Cookie。
///
/// - **幂等 200**：无有效会话（无 Cookie / 伪造 / 已删）同样 200——绝不
///   挂 require_auth（过期会话登出若 401 会联动「被踢回登录页」误判路径）；
/// - 携带有效会话 ⇒ 删 `auth_sessions` 当前行（伪造/未知值 DELETE 匹配
///   0 行，结果同幂等）；
/// - Set-Cookie `egosync_session=; Max-Age=0` 同属性（Path/HttpOnly/
///   SameSite=Strict；反代 TLS 面追加 Secure——过期 Cookie 与签发 Cookie
///   属性须对称，否则部分浏览器不覆盖）过期——浏览器立即丢弃；
/// - 会话行删除失败（DB 故障）：仍 200 + 过期 Cookie（客户端态已清，
///   孤儿行不可达——无 Cookie 值即不可劫持；tracing::error 运维可见；
///   TTL 清扫归 17.x）。
pub async fn logout(State(state): State<Arc<AppState>>, req: Request) -> Response {
    let session_token = extract_session_token(&req);
    if let Some(token) = session_token.as_deref() {
        let token_hash = sha256_hex(token.as_bytes());
        match sqlx::query("DELETE FROM auth_sessions WHERE token_hash = ?1")
            .bind(&token_hash)
            .execute(&state.auth.pool)
            .await
        {
            Ok(result) if result.rows_affected() > 0 => {
                tracing::info!("登出：会话行已删除");
            }
            Ok(_) => {} // 无匹配行（无会话/已登出/伪造值）——幂等静默
            Err(e) => {
                tracing::error!("登出：会话行删除失败（孤儿行不可达，TTL 清扫归 17.x）: {}", e);
            }
        }
    }
    let cookie = format!(
        "{}=; Path=/; HttpOnly; SameSite=Strict; Max-Age=0{}",
        SESSION_COOKIE,
        cookie_secure_flag(state.behind_proxy, req.headers())
    );
    let mut response = (StatusCode::OK, Json(json!({"ok": true}))).into_response();
    response.headers_mut().insert(
        header::SET_COOKIE,
        HeaderValue::from_str(&cookie).expect("登出 Cookie 头构造失败"),
    );
    response
}

/// 认证中间件：业务端点（含 SSE）守门——会话 Cookie 无效 ⇒ 401 统一形状。
///
/// Story 16.3 叠加通道：`Authorization: Bearer <token>` 有效 ⇒ 放行（校验
/// 引导主令牌，非会话表）。Bearer 提供但无效 ⇒ 401（不回落 Cookie——
/// 客户端显式出示凭据时结果须确定）。无 Authorization 头 ⇒ Cookie 路径
/// 零变化（与 15.4 逐语义一致）。
///
/// [T5 修订]：Bearer 验证失败按 IP 计数限流（仅计失败；5 次/分钟/IP
/// 滑动窗口；超限 ⇒ 429）——计数先于限流判定之外还有一层语义：**成功
/// 访问不计数也不受超限影响**（有效令牌必须始终放行，故校验先行、仅
/// 失败路径计数）。
pub async fn require_auth(
    ConnectInfo(addr): ConnectInfo<crate::idle_timeout::RemoteAddr>,
    State(state): State<Arc<AppState>>,
    req: Request,
    next: Next,
) -> Response {
    // 注意：Body 为 !Sync——先同步提取凭据再入异步校验
    if let Some(bearer) = extract_bearer_token(&req) {
        if !verify_primary_token(&state, &bearer).await {
            if !state.bearer_rate.check(addr.0.ip()) {
                tracing::warn!(ip = %addr.0.ip(), "Bearer 通道限流拒绝（5 次/分钟/IP）");
                return too_many_requests();
            }
            return unauthorized();
        }
        return next.run(req).await;
    }
    let session_token = extract_session_token(&req);
    if !session_cookie_valid(&state, session_token.as_deref()).await {
        return unauthorized();
    }
    next.run(req).await
}

/// SSE 事件流专用认证（Story 16.3）：三通道——Bearer 主令牌 / 一次性
/// 票据（`?ticket=`，EventSource 无法带自定义头）/ 会话 Cookie。
///
/// 票据通道：存在即校验并**消费**（单次使用）——有效放行、无效/过期 ⇒
/// 401（不回落其他通道：票据是显式出示的凭据）。票据只在 `/api/events`
/// 路由接受（本中间件不挂业务命令面——命令通道无 EventSource 约束）。
///
/// [T5 修订]：Bearer 分支同计数器（`/api/events` 的 Bearer 失败与
/// `/api/cmd/*`、`/api/events/ticket` 共享 5 次/分钟/IP 失败面——同一
/// 主令牌空间不因换端点旁路；票据/Cookie 分支不计数——票据单次使用
/// 本身即限速，Cookie 面失败语义归 15.4 既有路径）。
pub async fn require_auth_with_sse_ticket(
    ConnectInfo(addr): ConnectInfo<crate::idle_timeout::RemoteAddr>,
    State(state): State<Arc<AppState>>,
    req: Request,
    next: Next,
) -> Response {
    if let Some(bearer) = extract_bearer_token(&req) {
        if !verify_primary_token(&state, &bearer).await {
            if !state.bearer_rate.check(addr.0.ip()) {
                tracing::warn!(ip = %addr.0.ip(), "Bearer 通道限流拒绝（5 次/分钟/IP）");
                return too_many_requests();
            }
            return unauthorized();
        }
        return next.run(req).await;
    }
    if let Some(ticket) = extract_query_ticket(&req) {
        if !state.sse_tickets.consume(&ticket) {
            return unauthorized();
        }
        return next.run(req).await;
    }
    // Cookie 路径（浏览器 EventSource 既有语义零变化）
    let session_token = extract_session_token(&req);
    if !session_cookie_valid(&state, session_token.as_deref()).await {
        return unauthorized();
    }
    next.run(req).await
}

/// 同步提取会话 Cookie 值（Body 为 !Sync——Request 引用不得跨 await）。
fn extract_session_token(req: &Request) -> Option<String> {
    let cookie_header = req.headers().get(header::COOKIE)?;
    // 手写 Cookie 解析（单实例单用户量级，不引 axum-extra）
    let header_str = cookie_header.to_str().ok()?;
    header_str
        .split(';')
        .map(|pair| pair.trim())
        .filter_map(|pair| pair.split_once('='))
        .find(|(name, _)| *name == SESSION_COOKIE)
        .map(|(_, value)| value.to_string())
        .filter(|value| !value.is_empty())
}

/// 同步提取 `Authorization: Bearer <token>` 值（Story 16.3 叠加通道）。
///
/// scheme 大小写不敏感（RFC 9110）；无 Authorization 头 / 非 Bearer /
/// 空令牌 ⇒ None（调用方回落 Cookie 通道）。
fn extract_bearer_token(req: &Request) -> Option<String> {
    let value = req.headers().get(header::AUTHORIZATION)?.to_str().ok()?;
    let (scheme, token) = value.split_once(' ')?;
    if !scheme.eq_ignore_ascii_case("bearer") {
        return None;
    }
    let token = token.trim();
    if token.is_empty() {
        return None;
    }
    Some(token.to_string())
}

/// 同步提取查询参数 `ticket` 值（SSE 一次性票据通道）。
fn extract_query_ticket(req: &Request) -> Option<String> {
    let query = req.uri().query()?;
    query.split('&').find_map(|pair| {
        let (key, value) = pair.split_once('=')?;
        (key == "ticket" && !value.is_empty()).then(|| value.to_string())
    })
}

/// 引导主令牌校验（Bearer 通道——与 login 同源逻辑，**非会话表**）：
/// env 态常时比对（SHA256 摘要常时比较）；库态 Argon2id（哈希缺失 ⇒
/// 未初始化 ⇒ false）。
///
/// 性能口径：env 态亚毫秒；库态每请求一次 Argon2id（桌面远程单用户
/// 低 QPS，规格钉死「校验 env/Argon2id 主令牌，非会话表」——不走
/// Cookie 会话换发通道）。
async fn verify_primary_token(state: &Arc<AppState>, token: &str) -> bool {
    match state.auth.env_token.as_deref() {
        // env 态：仅常时比对 env（优先级冻结，无 fail-open）
        Some(env_token) => constant_time_eq_token(token, env_token),
        // 库态：Argon2id 校验（哈希缺失 ⇒ 未初始化 ⇒ 拒绝）
        None => match app_settings::get_setting(&state.auth.pool, TOKEN_HASH_SETTING_KEY).await {
            Ok(Some(phc)) => verify_token_argon2id(token, &phc),
            _ => false,
        },
    }
}

/// 会话 Cookie 校验：SHA256 → auth_sessions 主键查找（跨重启/跨凭据形态
/// 切换均有效——会话生命周期独立于引导凭据）。
///
/// 读路径零写放大（评审修复 #7：YAGNI 简化——无消费者的活跃度列
/// 已随迁移删除，校验保持纯读）。
async fn session_cookie_valid(state: &Arc<AppState>, session_token: Option<&str>) -> bool {
    let Some(session_token) = session_token else {
        return false;
    };
    let token_hash = sha256_hex(session_token.as_bytes());
    sqlx::query_scalar::<_, i32>("SELECT 1 FROM auth_sessions WHERE token_hash = ?1")
        .bind(&token_hash)
        .fetch_optional(&state.auth.pool)
        .await
        .is_ok_and(|row| row.is_some())
}

/// 统一 401 形状（不泄露存在性）。
fn unauthorized() -> Response {
    (
        StatusCode::UNAUTHORIZED,
        Json(json!({"error": "unauthorized"})),
    )
        .into_response()
}

/// 统一 429 形状（限流超限——auth 面与 [T5] Bearer 失败面同款）。
fn too_many_requests() -> Response {
    (
        StatusCode::TOO_MANY_REQUESTS,
        Json(json!({"error": "rate limit exceeded"})),
    )
        .into_response()
}

/// 解析 `{token}` body（缺失/非 JSON ⇒ None——login 面统一 401）。
fn parse_token_body(body: &Bytes) -> Option<String> {
    serde_json::from_slice::<serde_json::Value>(body)
        .ok()?
        .get("token")?
        .as_str()
        .map(|s| s.to_string())
}

/// 反代感知 Cookie `Secure` 附加判定（Story 17.1）。
///
/// `EGOSYNC_BEHIND_PROXY=1` 且 `X-Forwarded-Proto` 首值（逗号分隔代理链
/// 取最左）为 https ⇒ 追加 `; Secure`；其余（门控未启用 / XFP 缺失 /
/// 明文入口 http）返回空串。语义：
/// - 直连（dev/e2e，16.2 web e2e 直连 http://127.0.0.1 不设 env）行为
///   与 15.4 逐字节一致——永不附加；
/// - http 反代入口不附加（Secure Cookie 不得在明文面签发，浏览器会拒收）；
/// - Bearer 通道不受影响（无 Cookie）。
fn cookie_secure_flag(behind_proxy: bool, headers: &HeaderMap) -> &'static str {
    if !behind_proxy {
        return "";
    }
    let https = headers
        .get("x-forwarded-proto")
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.split(',').next())
        .is_some_and(|v| v.trim().eq_ignore_ascii_case("https"));
    if https { "; Secure" } else { "" }
}

/// 库内是否已有令牌哈希。
///
/// 读错误原样上抛（绝不吞成「无哈希」——那是 fail-open：已初始化实例
/// 会误挂 /api/setup，凭据可被覆写）。
async fn db_has_token_hash(pool: &DbPool) -> Result<bool, String> {
    app_settings::get_setting(pool, TOKEN_HASH_SETTING_KEY)
        .await
        .map(|v| v.is_some())
        .map_err(|e| format!("读取令牌哈希失败（拒启——不 fail-open）: {}", e))
}

/// 常时比较令牌：双方 SHA256 摘要后 32 字节常时比对（长度不敏感——
/// 摘要归一化避免长度泄露路径）。
fn constant_time_eq_token(provided: &str, expected: &str) -> bool {
    let a = Sha256::digest(provided.as_bytes());
    let b = Sha256::digest(expected.as_bytes());
    a.ct_eq(&b).into()
}

/// Argon2id 哈希（默认参数 m=19456 KiB / t=2 / p=1，OWASP 基线）。
fn hash_token_argon2id(token: &str) -> Result<String, String> {
    use argon2::password_hash::rand_core::OsRng;
    let salt = SaltString::generate(&mut OsRng);
    Argon2::default()
        .hash_password(token.as_bytes(), &salt)
        .map(|hash| hash.to_string())
        .map_err(|e| format!("{}", e))
}

/// Argon2id 校验（PHC 字符串解析失败 ⇒ false，不区分错误形态）。
fn verify_token_argon2id(token: &str, phc: &str) -> bool {
    let Ok(parsed) = PasswordHash::new(phc) else {
        return false;
    };
    Argon2::default()
        .verify_password(token.as_bytes(), &parsed)
        .is_ok()
}

/// SHA256 十六进制摘要。
fn sha256_hex(bytes: &[u8]) -> String {
    let digest = Sha256::digest(bytes);
    let mut out = String::with_capacity(digest.len() * 2);
    for byte in digest {
        out.push_str(&format!("{:02x}", byte));
    }
    out
}

/// 供测试/校验复用的 AppError 形状探测（错误白名单对等测试消费）。
///
/// Story 15.5：与生产 `cmd_handler` Err 分支同款——200 + 单键 map body +
/// 判别头 `X-Egosync-App-Error: 1`（HTTP 通道错误信号）。
pub fn app_error_to_response(err: AppError) -> Response {
    (
        StatusCode::OK,
        [(crate::routes::APP_ERROR_HEADER, crate::routes::APP_ERROR_HEADER_VALUE)],
        Json(err),
    )
        .into_response()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 评审修复 #2 单测：库哈希读失败（如池已关闭）必须沿 `AuthState::new`
    /// 上抛 `Err` 拒启——绝不吞成「无哈希」（fail-open 误挂 /api/setup，
    /// 已初始化实例的凭据将可被覆写）。
    #[tokio::test]
    async fn db_read_failure_rejects_startup_not_fail_open() {
        // 已关闭的池：任何查询都返回连接错误
        let pool = DbPool::connect("sqlite::memory:")
            .await
            .expect("建池");
        pool.close().await;

        let err = match AuthState::new(pool, None).await {
            Err(e) => e,
            Ok(_) => panic!("读失败必须 Err（拒启）——不得 fail-open"),
        };
        assert!(
            err.contains("读取令牌哈希失败"),
            "错误文案必须指向令牌哈希读失败: {}",
            err
        );
    }

    /// 评审修复 #5 单测：限流器空窗口驱逐——过期窗口清空后条目被移除，
    /// 扫描源 IP 不使 HashMap 无限增长。
    #[test]
    fn rate_limiter_evicts_empty_window_entries() {
        use std::net::Ipv4Addr;
        let limiter = RateLimiter::default();
        let ip = IpAddr::from(Ipv4Addr::new(1, 2, 3, 4));
        // 手工注入一条已过期记录（模拟一分钟前的请求）
        {
            let mut inner = limiter.inner.lock().unwrap();
            let mut stale = VecDeque::new();
            stale.push_back(Instant::now() - RATE_LIMIT_WINDOW - Duration::from_secs(1));
            inner.insert(ip, stale);
        }
        // 新请求：过期记录剔除后窗口为空 ⇒ 驱逐旧条目重插 ⇒ 放行
        assert!(limiter.check(ip), "过期后新请求应放行");
        // 条目数不增长（驱逐+重插，非叠加）
        {
            let inner = limiter.inner.lock().unwrap();
            assert_eq!(
                inner.len(),
                1,
                "同 IP 条目应恰一个（驱逐后重插，不叠加）"
            );
            assert_eq!(inner[&ip].len(), 1, "窗口内应恰一条新记录");
        }
    }

    /// [T5 修订] 单测：Bearer 失败面限流的窗口滑动恢复（60s 窗口不可在
    /// 集成测试内等待——注入跨窗记录驱动滑动语义；bearer_rate 与登录面
    /// 限流器同 `RateLimiter` 实现，窗口语义同源）。
    ///
    /// 场景：3 条已滑出窗口的旧失败 + 2 条仍在窗口内的失败 ⇒ 新失败
    /// 仍被放行计数（窗口内 2 < 5），直至窗口内再满 5 条才超限。
    #[test]
    fn bearer_failure_window_slides_and_recovers() {
        use std::net::Ipv4Addr;
        let limiter = RateLimiter::default();
        let ip = IpAddr::from(Ipv4Addr::new(9, 9, 9, 9));
        {
            let mut inner = limiter.inner.lock().unwrap();
            let mut window = VecDeque::new();
            // 3 条 70s 前的失败（已滑出 60s 窗口）
            for _ in 0..3 {
                window.push_back(Instant::now() - RATE_LIMIT_WINDOW - Duration::from_secs(10));
            }
            // 2 条 10s 前的失败（仍在窗口内）
            for _ in 0..2 {
                window.push_back(Instant::now() - Duration::from_secs(10));
            }
            inner.insert(ip, window);
        }
        // 新失败 #3（窗口内 2 条 ⇒ 记录后 3 条）⇒ 放行（401 而非 429）
        assert!(limiter.check(ip), "窗口滑动后失败计数恢复（旧失败滑出不计）");
        // 新失败 #4（4 条）⇒ 放行
        assert!(limiter.check(ip));
        // 新失败 #5（5 条——窗口内计数满）⇒ 放行（第 5 次失败仍 401）
        assert!(limiter.check(ip), "窗口内第 5 次失败仍属放行档（401）");
        // 第 6 条窗口内失败 ⇒ 超限（429）
        assert!(!limiter.check(ip), "窗口内满 5 条后第 6 次失败 ⇒ 429");
    }

    /// Story 17.1：Cookie Secure 门控判定（I/O 矩阵）——
    /// - 反代 TLS（BEHIND_PROXY=1 + XFP=https）⇒ 追加 `; Secure`；
    /// - 代理链首值语义（`https, http` 取最左）；
    /// - 明文反代入口（XFP=http）⇒ 不附加；
    /// - 直连暴露伪造 XFP=https（门控未启用）⇒ 忽略，不附加
    ///   （16.2 web e2e 直连行为不变）。
    #[test]
    fn cookie_secure_flag_gates_on_behind_proxy_and_xfp() {
        let https_headers = |v: &'static str| {
            let mut h = HeaderMap::new();
            h.insert("x-forwarded-proto", HeaderValue::from_static(v));
            h
        };
        // 反代 TLS ⇒ Secure
        assert_eq!(cookie_secure_flag(true, &https_headers("https")), "; Secure");
        // 代理链取首值
        assert_eq!(
            cookie_secure_flag(true, &https_headers("https, http")),
            "; Secure"
        );
        // 大小写不敏感
        assert_eq!(cookie_secure_flag(true, &https_headers("HTTPS")), "; Secure");
        // 明文入口 ⇒ 不附加
        assert_eq!(cookie_secure_flag(true, &https_headers("http")), "");
        // 反代启用但头缺失 ⇒ 不附加
        assert_eq!(cookie_secure_flag(true, &HeaderMap::new()), "");
        // 直连暴露（门控未启用）：伪造 XFP=https 不得附加
        assert_eq!(cookie_secure_flag(false, &https_headers("https")), "");
        assert_eq!(cookie_secure_flag(false, &HeaderMap::new()), "");
    }
}
