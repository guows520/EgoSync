use std::collections::HashMap;
use std::sync::Arc;

use tokio::sync::{mpsc, Mutex};

use crate::models::agent::BusEvent;
use crate::services::agent_bridge::AgentBridge;

/// Routes opencode bus events from the global `GET /event` stream to per-session
/// subscribers. One conversation = one subscription = one channel.
///
/// Why a router exists: opencode's `/event` stream is a single global SSE that
/// multiplexes ALL session events. Each EgoSync conversation cares about a
/// specific `sessionID`, so the router demultiplexes by inspecting each event's
/// `properties.sessionID` and forwarding to the matching subscriber.
#[derive(Clone, Default)]
pub struct EventRouter {
    subscribers: Arc<Mutex<HashMap<String, mpsc::Sender<BusEvent>>>>,
}

impl EventRouter {
    pub fn new() -> Self {
        Self::default()
    }

    /// Register a subscriber for `session_id`. Returns the receiver end the
    /// caller drains until it sees a terminal event (`session.idle` /
    /// `session.error`).
    pub async fn subscribe(&self, session_id: &str) -> mpsc::Receiver<BusEvent> {
        let (tx, rx) = mpsc::channel::<BusEvent>(64);
        let mut subs = self.subscribers.lock().await;
        subs.insert(session_id.to_string(), tx);
        tracing::debug!(session_id, total = subs.len(), "bus: subscribed");
        rx
    }

    /// Drop the subscription for `session_id`. Call when one prompt round
    /// finishes so the channel and its future receivers don't leak.
    pub async fn unsubscribe(&self, session_id: &str) {
        let mut subs = self.subscribers.lock().await;
        let removed = subs.remove(session_id).is_some();
        tracing::debug!(session_id, removed, total = subs.len(), "bus: unsubscribed");
    }

    /// Dispatch one bus event to the matching subscriber, if any.
    async fn dispatch(&self, event: BusEvent) {
        let session_id = event
            .properties
            .get("sessionID")
            .and_then(|v| v.as_str())
            .map(str::to_string);
        tracing::trace!(event_type = %event.event_type, ?session_id, "bus: dispatch");
        let Some(session_id) = session_id else {
            return;
        };
        let subs = self.subscribers.lock().await;
        if let Some(tx) = subs.get(&session_id) {
            // Best-effort: if the subscriber's channel is full or dropped,
            // we drop the event rather than block other sessions.
            let _ = tx.try_send(event);
        }
    }

    /// Run the long-running event pump. Subscribes to opencode's global
    /// event stream and dispatches each event by sessionID. Reconnects with
    /// backoff when the stream closes. This is an async fn so the caller
    /// chooses the runtime (Tokio vs `tauri::async_runtime::spawn`).
    pub async fn run_pump(self: Arc<Self>, bridge: AgentBridge) {
        loop {
            let (tx, mut rx) = mpsc::channel::<BusEvent>(128);
            let bridge_clone = bridge.clone();
            let pump = tokio::spawn(async move {
                let _ = bridge_clone.subscribe_events(tx).await;
            });

            while let Some(event) = rx.recv().await {
                self.dispatch(event).await;
            }

            let _ = pump.await;
            tracing::warn!("opencode event stream disconnected, reconnecting in 2s");
            tokio::time::sleep(std::time::Duration::from_secs(2)).await;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn make_event(session_id: &str) -> BusEvent {
        BusEvent {
            event_type: "session.idle".into(),
            properties: json!({ "sessionID": session_id }),
        }
    }

    #[tokio::test]
    async fn dispatch_routes_event_to_matching_session_subscriber() {
        // WHY: this is the whole reason the router exists — without correct
        // routing, every conversation would receive every other conversation's
        // tokens, corrupting all UI streams.
        let router = EventRouter::new();
        let mut rx_a = router.subscribe("ses-A").await;
        let mut rx_b = router.subscribe("ses-B").await;

        router.dispatch(make_event("ses-A")).await;

        let received_a = rx_a.try_recv().ok();
        let received_b = rx_b.try_recv().ok();
        assert!(
            received_a.is_some(),
            "ses-A subscriber must receive its event"
        );
        assert!(
            received_b.is_none(),
            "ses-B subscriber must not see ses-A's event"
        );
    }

    #[tokio::test]
    async fn dispatch_drops_event_when_session_id_missing() {
        // WHY: malformed events from opencode must not panic or crash the
        // pump — they should be silently dropped so streaming for other
        // sessions keeps working.
        let router = EventRouter::new();
        let mut rx = router.subscribe("ses-A").await;

        let event = BusEvent {
            event_type: "server.heartbeat".into(),
            properties: json!({}),
        };
        router.dispatch(event).await;

        assert!(
            rx.try_recv().is_err(),
            "heartbeats without sessionID should not reach session subscribers"
        );
    }

    #[tokio::test]
    async fn unsubscribe_stops_receiving_further_events() {
        // WHY: leaked subscribers would cause unbounded memory growth and
        // potential cross-conversation contamination if session IDs are
        // ever reused.
        let router = EventRouter::new();
        let _rx = router.subscribe("ses-A").await;

        router.unsubscribe("ses-A").await;
        router.dispatch(make_event("ses-A")).await;

        // No assertion target beyond the fact this completes without
        // dispatch panicking; map must allow removal.
        let subs = router.subscribers.lock().await;
        assert!(!subs.contains_key("ses-A"));
    }
}
