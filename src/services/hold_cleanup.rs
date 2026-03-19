use crate::db::HoldOperations;
use crate::sse::{SseBroker, SseEvent};
use actix_web::web;
use tokio::time::{interval, Duration};

pub async fn run_hold_cleanup(hold_ops: HoldOperations, broker: SseBroker) {
    let mut tick = interval(Duration::from_secs(60));
    loop {
        tick.tick().await;
        match web::block({
            let hold_ops = hold_ops.clone();
            move || hold_ops.cleanup_expired_holds()
        })
        .await
        {
            Ok(Ok((count, restored_inventory_updates))) => {
                for (canteen_id, inventory_updates) in restored_inventory_updates {
                    broker.publish_canteen_subscription_event(
                        canteen_id,
                        &SseEvent::InventoryUpdate {
                            items: inventory_updates,
                        },
                    );
                }
                if count > 0 {
                    info!("Background cleanup: released {} expired order holds", count);
                }
            }
            Ok(Err(e)) => {
                error!("Background cleanup error: {}", e);
            }
            Err(e) => {
                error!("Background cleanup blocking error: {}", e);
            }
        }
    }
}
