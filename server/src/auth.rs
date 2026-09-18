//! 单用户认证（Story 15.4，架构决策 #5）：env/首访 setup、Argon2id、
//! httpOnly Cookie 会话、401 中间件、IP 滑动窗口限流。
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
use axum::http::{header, HeaderValue, StatusCode};
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
pub async fn rate_limit(
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    State(state): State<Arc<AppState>>,
    req: Request,
    next: Next,
) -> Response {
    if !state.auth.rate.check(addr.ip()) {
        return (
            StatusCode::TOO_MANY_REQUESTS,
            Json(json!({"error": "rate limit exceeded"})),
        )
            .into_response();
    }
    next.run(req).await
}

/// `GET /api/auth/status`：认证前发现端点（setupRequired / authenticated）。
///
/// setupRequired = 无 env 且库内无哈希（首访引导）；authenticated = 当前
/// Cookie 会话有效。不泄露令牌存在性之外的信息。
pub async fn auth_status(State(state): State<Arc<AppState>>, req: Request) -> Response {
    // 注意：Body 为 !Sync——Request 只能按值跨 await（引用会使 future !Send）
    let session_token = extract_session_token(&req);
    let authenticated = session_cookie_valid(&state, session_token.as_deref()).await;
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
/// Strict）。body 反序列化失败同样 401。
pub async fn login(State(state): State<Arc<AppState>>, body: Bytes) -> Response {
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
    let cookie = format!(
        "{}={}; Path=/; HttpOnly; SameSite=Strict",
        SESSION_COOKIE, session_token
    );
    let mut response = (StatusCode::OK, Json(json!({"ok": true}))).into_response();
    response.headers_mut().insert(
        header::SET_COOKIE,
        HeaderValue::from_str(&cookie).expect("会话 Cookie 头构造失败"),
    );
    response
}

/// 认证中间件：业务端点（含 SSE）守门——会话 Cookie 无效 ⇒ 401 统一形状。
pub async fn require_auth(
    State(state): State<Arc<AppState>>,
    req: Request,
    next: Next,
) -> Response {
    // 注意：Body 为 !Sync——先同步提取 Cookie 再入异步校验
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

/// 解析 `{token}` body（缺失/非 JSON ⇒ None——login 面统一 401）。
fn parse_token_body(body: &Bytes) -> Option<String> {
    serde_json::from_slice::<serde_json::Value>(body)
        .ok()?
        .get("token")?
        .as_str()
        .map(|s| s.to_string())
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
pub fn app_error_to_response(err: AppError) -> Response {
    (StatusCode::OK, Json(err)).into_response()
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
}
