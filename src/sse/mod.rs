mod broker;

use actix_web_lab::sse;
pub use broker::SseBroker;
use serde::Serialize;
use std::time::SystemTime;

#[derive(Clone, Debug, Serialize)]
#[serde(untagged)]
pub enum SseEvent {
    InventoryUpdate {
        // to both user and canteen
        item_id: i32,
        stock: i32,
        is_available: bool,
        price: i32,
    },
    UserOrderUpdate {
        // only to user
        order_id: i32,
        status: String, // "delivered" or "cancelled" or "placed"
    },
    CanteenAggregatedOrderUpdate {
        // only to canteen
        time_band: String,
        item_id: i32,
        num_ordered: i32,
    },
}

impl SseEvent {
    pub fn to_sse_event(&self) -> sse::Event {
        let (event_name, json_str) = match self {
            SseEvent::InventoryUpdate { .. } => {
                ("inventory_update", serde_json::to_string(self).unwrap())
            }
            SseEvent::UserOrderUpdate { .. } => {
                ("user_order_update", serde_json::to_string(self).unwrap())
            }
            SseEvent::CanteenAggregatedOrderUpdate { .. } => (
                "canteen_aggregated_order_update",
                serde_json::to_string(self).unwrap(),
            ),
        };
        let now = SystemTime::now();
        let epoch = now.duration_since(SystemTime::UNIX_EPOCH).unwrap();
        sse::Data::new(json_str)
            .event(event_name)
            .id(epoch.as_millis().to_string())
            .into()
    }
}
