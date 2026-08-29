//! 内存注册表（AC2、AC3）：`relay_id → {desktop 槽, phone 槽}`。
//!
//! 零持久化纪律（AC3）：注册表只存在于本进程内存，无 DB、无磁盘写，
//! 断线即丢（不做离线投递）；进程重启后注册表为空（集成测试以真子进程断言）。
//!
//! 生命周期语义：
//! - **单槽替换**：同角色二次注册，验证通过后替换旧槽并关闭旧连接
//!   （镜像 12.2 评审 P6「新连接取代僵尸会话」）；
//! - **phone 槽公钥绑定**（评审裁决 D1-a）：phone 槽首次注册绑定静态公钥，
//!   此后同 relay_id 的 phone 重复注册须持同一公钥，不一致即拒绝——
//!   堵死第三方仅凭 relay_id 稳定抢占 phone 槽的通道（desktop 槽无需此检查：
//!   `hash(pubkey)==relay_id` 已保证同 relay_id 必同密钥）；
//! - **断线清理**：任一端 WS 断开 → 清其槽位，并对端连接发送 close
//!   （让对端感知并走重连）；
//! - **条目回收**：双槽均空 → 移除该 `relay_id` 条目（防 Map 无界增长）；
//! - **优雅退出广播**：`shutdown_all` 经 watch 通道通知全部活跃连接
//!   发送 Close 后退出（Docker stop 场景，进程不得挂到 SIGKILL）。

use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::Instant;

use axum::extract::ws::Message;
use tokio::sync::mpsc::Sender;
use tokio::sync::{watch, Mutex};

/// 连接角色（relay 控制协议 `register.role` 的解析结果）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Role {
    Desktop,
    Phone,
}

/// 单个已注册连接占用的槽位。
pub struct Slot {
    /// 连接唯一 ID（单槽替换后，旧连接的清理不得误删新槽）。
    pub conn_id: u64,
    /// 向该连接推送消息的通道（对端 → 本连接方向；有界——满即断连，不做无界积压）。
    pub sender: Sender<Message>,
    /// 注册时刻。
    pub registered_at: Instant,
    /// 对端静态公钥（XX 握手验证所得；relay_id 归属桌面公钥哈希，见 auth.rs；
    /// phone 槽公钥绑定判据）。
    pub remote_static_pubkey: Vec<u8>,
}

/// 一个 relay_id 下的双槽位。
#[derive(Default)]
pub struct RelayEntry {
    pub desktop: Option<Slot>,
    pub phone: Option<Slot>,
}

/// 注册拒绝类别（phone 槽公钥绑定，评审裁决 D1-a）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RegisterError {
    /// phone 槽已被不同静态公钥占用（合法手机重配对须 desktop 侧重新配对）。
    PhoneKeyMismatch,
}

/// 全局连接 ID 生成器（进程内单调递增）。
static NEXT_CONN_ID: AtomicU64 = AtomicU64::new(1);

/// 分配一个连接唯一 ID。
pub fn next_conn_id() -> u64 {
    NEXT_CONN_ID.fetch_add(1, Ordering::Relaxed)
}

/// 内存注册表（`Arc<Mutex<HashMap<...>>>` + 优雅退出 watch 广播）。
#[derive(Clone)]
pub struct Registry {
    inner: Arc<Mutex<HashMap<String, RelayEntry>>>,
    shutdown: Arc<watch::Sender<bool>>,
}

impl Default for Registry {
    fn default() -> Self {
        let (shutdown, _rx) = watch::channel(false);
        Self {
            inner: Arc::new(Mutex::new(HashMap::new())),
            shutdown: Arc::new(shutdown),
        }
    }
}

impl Registry {
    pub fn new() -> Self {
        Self::default()
    }

    /// 登记槽位。同角色二次注册 = 单槽替换：旧槽 sender 被 drop，
    /// 旧连接任务由此感知通道关闭并退出（P6）。
    /// phone 槽已有不同公钥时拒绝（防抢占，D1-a）。
    /// 返回对端 sender（对端在线则转发即通，否则 `None`）。
    pub async fn register(
        &self,
        relay_id: &str,
        role: Role,
        slot: Slot,
    ) -> Result<Option<Sender<Message>>, RegisterError> {
        let mut map = self.inner.lock().await;
        let entry = map.entry(relay_id.to_string()).or_default();
        // 抢占防御（D1-a）：phone 槽已绑定其他公钥 → 拒绝且不登记。
        // desktop 槽无需检查——hash==relay_id 已保证同 relay_id 必同密钥。
        if role == Role::Phone {
            if let Some(existing) = entry.phone.as_ref() {
                if existing.remote_static_pubkey != slot.remote_static_pubkey {
                    return Err(RegisterError::PhoneKeyMismatch);
                }
            }
        }
        let peer = Self::peer_of(entry, role).map(|s| s.sender.clone());
        *Self::slot_of_mut(entry, role) = Some(slot); // 旧槽（若有）在此被 drop → 旧连接关闭
        Ok(peer)
    }

    /// 断线清理：仅当槽位仍属该连接（conn_id 匹配）时清除；
    /// 双槽均空 → 移除该 `relay_id` 条目。
    /// 返回对端 sender（调用方向其发送 close，让对端感知断线并走重连）。
    pub async fn disconnect(
        &self,
        relay_id: &str,
        role: Role,
        conn_id: u64,
    ) -> Option<Sender<Message>> {
        let mut map = self.inner.lock().await;
        let entry = map.get_mut(relay_id)?;
        let slot = Self::slot_of_mut(entry, role);
        if !slot.as_ref().is_some_and(|s| s.conn_id == conn_id) {
            return None; // 槽位已被新连接接管，旧连接的清理到此为止
        }
        *slot = None;
        let peer = Self::peer_of(entry, role).map(|s| s.sender.clone());
        if entry.desktop.is_none() && entry.phone.is_none() {
            map.remove(relay_id);
        }
        peer
    }

    /// 获取对端 sender（转发用；对端未上线返回 `None`——无离线投递）。
    pub async fn peer_sender(
        &self,
        relay_id: &str,
        role: Role,
    ) -> Option<Sender<Message>> {
        let map = self.inner.lock().await;
        let entry = map.get(relay_id)?;
        Self::peer_of(entry, role).map(|s| s.sender.clone())
    }

    /// 当前条目数（防 Map 无界增长的回归观测点）。
    pub async fn len(&self) -> usize {
        self.inner.lock().await.len()
    }

    pub async fn is_empty(&self) -> bool {
        self.len().await == 0
    }

    /// 广播优雅退出信号：全部活跃连接发送 Close 后退出（进程不得挂到 SIGKILL）。
    pub fn shutdown_all(&self) {
        let _ = self.shutdown.send(true);
    }

    /// 订阅优雅退出信号（每连接一份）。
    pub fn shutdown_rx(&self) -> watch::Receiver<bool> {
        self.shutdown.subscribe()
    }

    fn slot_of_mut(entry: &mut RelayEntry, role: Role) -> &mut Option<Slot> {
        match role {
            Role::Desktop => &mut entry.desktop,
            Role::Phone => &mut entry.phone,
        }
    }

    fn peer_of(entry: &RelayEntry, role: Role) -> Option<&Slot> {
        match role {
            Role::Desktop => entry.phone.as_ref(),
            Role::Phone => entry.desktop.as_ref(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::sync::mpsc;

    fn make_slot(conn_id: u64, pubkey: &[u8]) -> (Slot, mpsc::Receiver<Message>) {
        let (tx, rx) = mpsc::channel(16);
        (
            Slot {
                conn_id,
                sender: tx,
                registered_at: Instant::now(),
                remote_static_pubkey: pubkey.to_vec(),
            },
            rx,
        )
    }

    /// 空构造：新注册表必须为空（零持久化——无任何预置状态）。
    #[tokio::test]
    async fn empty_registry_starts_empty() {
        // WHY: 中继对转发内容零知识、对历史零记忆——「空启动」是零持久化的
        // 起点，若构造即携带状态则无从谈起断线即丢。
        let registry = Registry::new();
        assert_eq!(registry.len().await, 0);
        assert!(registry.is_empty().await);
    }

    /// 登记：先 desktop 后 phone；phone 登记时应拿到对端（desktop）sender。
    #[tokio::test]
    async fn register_fills_slot_and_returns_peer_when_both_present() {
        // WHY: 转发即通的前提是后到者能拿到先到者的推送通道——
        // 双槽模型下连接的建立顺序不应对可转发性产生影响。
        let registry = Registry::new();
        let (desktop_slot, _desktop_rx) = make_slot(1, &[1, 2, 3]);
        assert!(
            registry
                .register("aabbccddeeff0011", Role::Desktop, desktop_slot)
                .await
                .expect("desktop 注册不应被拒")
                .is_none(),
            "desktop 先注册时对端必不存在"
        );

        let (phone_slot, _phone_rx) = make_slot(2, &[4, 5, 6]);
        let peer = registry
            .register("aabbccddeeff0011", Role::Phone, phone_slot)
            .await
            .expect("phone 注册不应被拒");
        assert!(peer.is_some(), "phone 后注册时应拿到 desktop sender");

        assert_eq!(registry.len().await, 1, "同一 relay_id 只占一个条目");
    }

    /// 单槽替换：同角色二次注册 → 旧槽被关闭（旧连接收到通道关闭信号）。
    #[tokio::test]
    async fn slot_replacement_closes_old_connection() {
        // WHY: 12.2 评审 P6——僵尸会话不得阻挡新连接；替换必须即时生效，
        // 且旧连接必须能感知自己被取代（通道关闭）从而释放资源。
        let registry = Registry::new();
        let (old_slot, mut old_rx) = make_slot(1, &[1, 2, 3]);
        registry
            .register("aabbccddeeff0011", Role::Desktop, old_slot)
            .await
            .expect("desktop 注册不应被拒");

        let (new_slot, _new_rx) = make_slot(2, &[1, 2, 3]);
        registry
            .register("aabbccddeeff0011", Role::Desktop, new_slot)
            .await
            .expect("desktop 注册不应被拒");

        assert!(
            old_rx.recv().await.is_none(),
            "旧槽 sender 被 drop 后，旧连接通道必须读到关闭（None）"
        );
        assert_eq!(registry.len().await, 1, "替换不新增条目");
    }

    /// phone 槽公钥绑定（D1-a）：不同公钥的重复注册被拒；同公钥替换照常。
    #[tokio::test]
    async fn phone_slot_rejects_different_key_but_replaces_same_key() {
        // WHY: phone 槽若不绑定公钥，任何知道 relay_id 的人都能反复抢占、
        // 把合法手机挤成黑洞——绑定后抢占者连注册都过不去；
        // 而合法手机断线重连（同公钥）的替换语义不得被误伤。
        let registry = Registry::new();
        let key_a = vec![7u8; 32];
        let key_b = vec![8u8; 32];

        // 首次注册绑定 key_a
        let (slot_a, mut rx_a) = make_slot(1, &key_a);
        registry
            .register("aabbccddeeff0011", Role::Phone, slot_a)
            .await
            .expect("phone 首次注册不应被拒");

        // 不同公钥（key_b）→ 拒绝且旧槽不动
        let (slot_b, _rx_b) = make_slot(2, &key_b);
        assert_eq!(
            registry
                .register("aabbccddeeff0011", Role::Phone, slot_b)
                .await
                .unwrap_err(),
            RegisterError::PhoneKeyMismatch,
            "不同公钥的 phone 重复注册必须被拒（防抢占）"
        );
        assert_eq!(
            registry.len().await,
            1,
            "拒绝不得移除既有条目（旧槽不受影响，由下方同公钥替换验证）"
        );

        // 同公钥（key_a）重连 → 替换成功，旧槽关闭
        let (slot_a2, _rx_a2) = make_slot(3, &key_a);
        registry
            .register("aabbccddeeff0011", Role::Phone, slot_a2)
            .await
            .expect("同公钥的 phone 重连（单槽替换）不应被拒");
        assert!(
            rx_a.recv().await.is_none(),
            "同公钥替换后旧槽必须关闭（P6 语义不变）"
        );
        assert_eq!(registry.len().await, 1, "替换不新增条目");
    }

    /// 断线清理：desktop 断开 → 槽位清除、返回对端（phone）sender 供 close 通知。
    #[tokio::test]
    async fn disconnect_cleans_slot_and_returns_peer_sender() {
        // WHY: 一端断线若不清槽，该 relay_id 将永远无法重建会话；
        // 返回对端 sender 是为了让对端收到 close 主动重连，而非等到超时。
        let registry = Registry::new();
        let (desktop_slot, _desktop_rx) = make_slot(1, &[1, 2, 3]);
        registry
            .register("aabbccddeeff0011", Role::Desktop, desktop_slot)
            .await
            .expect("desktop 注册不应被拒");
        let (phone_slot, _phone_rx) = make_slot(2, &[4, 5, 6]);
        registry
            .register("aabbccddeeff0011", Role::Phone, phone_slot)
            .await
            .expect("phone 注册不应被拒");

        let peer = registry
            .disconnect("aabbccddeeff0011", Role::Desktop, 1)
            .await;
        assert!(peer.is_some(), "desktop 断线应返回 phone sender");

        assert_eq!(
            registry.len().await,
            1,
            "phone 槽仍在，条目不得移除"
        );
    }

    /// 双槽空 → 条目移除（防 Map 无界增长）。
    #[tokio::test]
    async fn disconnect_removes_entry_when_both_slots_empty() {
        // WHY: relay_id 是公钥哈希、空间无限——若双槽空后不回收条目，
        // 长寿命进程的注册表将随历史连接数无界增长（AC3 违约）。
        let registry = Registry::new();
        let (desktop_slot, _desktop_rx) = make_slot(1, &[1, 2, 3]);
        registry
            .register("aabbccddeeff0011", Role::Desktop, desktop_slot)
            .await
            .expect("desktop 注册不应被拒");
        let (phone_slot, _phone_rx) = make_slot(2, &[4, 5, 6]);
        registry
            .register("aabbccddeeff0011", Role::Phone, phone_slot)
            .await
            .expect("phone 注册不应被拒");

        registry
            .disconnect("aabbccddeeff0011", Role::Desktop, 1)
            .await;
        registry
            .disconnect("aabbccddeeff0011", Role::Phone, 2)
            .await;

        assert_eq!(registry.len().await, 0, "双槽空后条目必须移除");
    }

    /// 旧连接的断线清理不得误删已被新连接接管的槽位。
    #[tokio::test]
    async fn stale_disconnect_does_not_clear_new_slot() {
        // WHY: 单槽替换后，旧连接的退出路径与新一轮注册并发——
        // 若按角色清槽（而非按 conn_id），旧连接退出会踢掉新连接（P6 回归）。
        let registry = Registry::new();
        let (old_slot, _old_rx) = make_slot(1, &[1, 2, 3]);
        registry
            .register("aabbccddeeff0011", Role::Desktop, old_slot)
            .await
            .expect("desktop 注册不应被拒");
        let (new_slot, _new_rx) = make_slot(2, &[1, 2, 3]);
        registry
            .register("aabbccddeeff0011", Role::Desktop, new_slot)
            .await
            .expect("desktop 注册不应被拒");

        // 旧连接（conn_id=1）的清理：不返回对端、不动新槽
        let peer = registry
            .disconnect("aabbccddeeff0011", Role::Desktop, 1)
            .await;
        assert!(peer.is_none(), "旧连接清理不得触碰新连接的槽位");
        assert_eq!(registry.len().await, 1, "新槽仍在，条目不得移除");

        // 新槽完好：phone 注册仍能拿到 desktop（新连接）sender
        let (phone_slot, _phone_rx) = make_slot(3, &[4, 5, 6]);
        assert!(
            registry
                .register("aabbccddeeff0011", Role::Phone, phone_slot)
                .await
                .expect("phone 注册不应被拒")
                .is_some(),
            "新 desktop 槽必须仍然可配对"
        );
    }

    /// 对端未上线时 peer_sender 返回 None（无离线投递）。
    #[tokio::test]
    async fn peer_sender_none_when_peer_offline() {
        // WHY: 中继不做离线投递（架构硬边界 #3）——对端不在就丢弃，
        // 唯一允许的语义是「现在没有」，绝不能排队等待。
        let registry = Registry::new();
        let (desktop_slot, _desktop_rx) = make_slot(1, &[1, 2, 3]);
        registry
            .register("aabbccddeeff0011", Role::Desktop, desktop_slot)
            .await
            .expect("desktop 注册不应被拒");

        assert!(
            registry
                .peer_sender("aabbccddeeff0011", Role::Desktop)
                .await
                .is_none()
        );
        assert!(
            registry
                .peer_sender("nonexistent", Role::Desktop)
                .await
                .is_none()
        );
    }

    /// 优雅退出广播：shutdown_all 后所有订阅者可感知。
    #[tokio::test]
    async fn shutdown_all_notifies_subscribers() {
        // WHY: Docker stop 时进程只有 10s——若不广播关闭信号，
        // 活跃连接任务会一直挂到被 SIGKILL，客户端永远收不到干净 Close。
        let registry = Registry::new();
        let mut rx1 = registry.shutdown_rx();
        let mut rx2 = registry.shutdown_rx();
        registry.shutdown_all();
        assert!(rx1.changed().await.is_ok(), "订阅者 1 必须感知关闭信号");
        assert!(rx2.changed().await.is_ok(), "订阅者 2 必须感知关闭信号");
        assert!(*rx1.borrow(), "信号值必须为 true");
    }
}
