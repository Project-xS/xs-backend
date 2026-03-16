use crate::sse::SseEvent;
use actix_web_lab::sse;
use dashmap::DashMap;
use std::collections::HashMap;
use tokio::sync::mpsc;
use uuid::Uuid;

#[derive(Clone)]
pub struct SseBroker {
    user_conns: DashMap<i32, HashMap<Uuid, mpsc::Sender<sse::Event>>>, // send user order updates
    canteen_conns: DashMap<i32, HashMap<Uuid, mpsc::Sender<sse::Event>>>, // sends canteen aggregated order updates
    canteen_subs: DashMap<i32, HashMap<Uuid, mpsc::Sender<sse::Event>>>, // canteen id -> tx - sends both inventory updates
}
impl SseBroker {
    pub fn new() -> Self {
        Self {
            user_conns: DashMap::new(),
            canteen_conns: DashMap::new(),
            canteen_subs: DashMap::new(),
        }
    }

    pub fn register_user_connection(
        &self,
        user_id: i32,
        conn_id: Uuid,
        tx: mpsc::Sender<sse::Event>,
    ) {
        self.user_conns
            .entry(user_id)
            .or_default()
            .insert(conn_id, tx);
        debug!(
            "user_order_events: user {} connected with id {}",
            user_id, conn_id
        );
    }

    pub fn register_canteen_connection(
        &self,
        canteen_id: i32,
        conn_id: Uuid,
        tx: mpsc::Sender<sse::Event>,
    ) {
        self.canteen_conns
            .entry(canteen_id)
            .or_default()
            .insert(conn_id, tx);
        debug!(
            "canteen_order_events: canteen {} connected with id {}",
            canteen_id, conn_id
        );
    }

    pub fn register_canteen_subscription(
        &self,
        canteen_id: i32,
        conn_id: Uuid,
        tx: mpsc::Sender<sse::Event>,
    ) {
        self.canteen_subs
            .entry(canteen_id)
            .or_default()
            .insert(conn_id, tx);
        debug!(
            "canteen_subscription_events: canteen {} connected with id {}",
            canteen_id, conn_id
        );
    }

    pub fn unregister_user_connection(&self, user_id: i32, conn_id: Uuid) {
        if let Some(mut conn_map) = self.user_conns.get_mut(&user_id) {
            conn_map.remove(&conn_id);
            if conn_map.is_empty() {
                // drop whole hashmap if user has completely disconnected
                drop(conn_map);
                self.user_conns.remove(&user_id);
            }
        }
        debug!(
            "user_order_events: user {} disconnected with id {}",
            user_id, conn_id
        );
    }

    pub fn unregister_canteen_connection(&self, canteen_id: i32, conn_id: Uuid) {
        if let Some(mut conn_map) = self.canteen_conns.get_mut(&canteen_id) {
            conn_map.remove(&conn_id);
            if conn_map.is_empty() {
                drop(conn_map);
                self.canteen_conns.remove(&canteen_id);
            }
        }
        debug!(
            "canteen_order_events: canteen {} disconnected with id {}",
            canteen_id, conn_id
        );
    }

    pub fn unregister_canteen_subscription(&self, canteen_id: i32, conn_id: Uuid) {
        if let Some(mut conn_map) = self.canteen_subs.get_mut(&canteen_id) {
            conn_map.remove(&conn_id);
            if conn_map.is_empty() {
                drop(conn_map);
                self.canteen_subs.remove(&canteen_id);
            }
        }
        debug!(
            "canteen_subscription_events: canteen {} disconnected with id {}",
            canteen_id, conn_id
        );
    }

    pub fn publish_user_event(&self, user_id: i32, event: &SseEvent) {
        debug!("user_order_events: publishing event to user {}", user_id);
        let sse_event = event.to_sse_event();
        let mut dead_devices: Vec<Uuid> = Vec::new();
        if let Some(conn_map) = self.user_conns.get(&user_id) {
            for (conn_id, tx) in conn_map.iter() {
                if tx.try_send(sse_event.clone()).is_err() {
                    dead_devices.push(*conn_id);
                }
            }
            for dead_device in dead_devices {
                self.unregister_user_connection(user_id, dead_device);
            }
        }
        debug!(
            "user_order_events: finished publishing event to user {}",
            user_id
        );
    }

    pub fn publish_canteen_event(&self, canteen_id: i32, event: &SseEvent) {
        debug!(
            "canteen_order_events: publishing event to canteen {}",
            canteen_id
        );
        let sse_event = event.to_sse_event();
        let mut dead_devices: Vec<Uuid> = Vec::new();
        if let Some(conn_map) = self.canteen_conns.get(&canteen_id) {
            for (conn_id, tx) in conn_map.iter() {
                if tx.try_send(sse_event.clone()).is_err() {
                    dead_devices.push(*conn_id);
                }
            }
            for dead_device in dead_devices {
                self.unregister_canteen_connection(canteen_id, dead_device);
            }
        }
        debug!(
            "canteen_order_events: finished publishing event to canteen {}",
            canteen_id
        );
    }

    pub fn publish_canteen_subscription_event(&self, canteen_id: i32, event: &SseEvent) {
        debug!(
            "canteen_subscription_events: publishing event to canteen {}",
            canteen_id
        );
        let sse_event = event.to_sse_event();
        let mut dead_devices: Vec<Uuid> = Vec::new();
        if let Some(conn_map) = self.canteen_subs.get(&canteen_id) {
            for (conn_id, tx) in conn_map.iter() {
                if tx.try_send(sse_event.clone()).is_err() {
                    dead_devices.push(*conn_id);
                }
            }
            for dead_device in dead_devices {
                self.unregister_canteen_subscription(canteen_id, dead_device);
            }
        }
        debug!(
            "canteen_subscription_events: finished publishing event to canteen {}",
            canteen_id
        );
    }
}
