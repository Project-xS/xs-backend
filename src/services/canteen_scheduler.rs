use crate::db::CanteenOperations;
use crate::services::canteen_hours::compute_close_at;
use chrono::{DateTime, FixedOffset, Utc};
use std::cmp::Ordering;
use std::sync::Arc;
use tokio::sync::Notify;
use tokio::time::{sleep, Duration};

#[derive(Clone)]
pub struct CanteenSchedulerNotifier {
    notify: Arc<Notify>,
}

impl CanteenSchedulerNotifier {
    pub fn new() -> Self {
        Self {
            notify: Arc::new(Notify::new()),
        }
    }

    pub fn notify(&self) {
        self.notify.notify_one();
    }

    async fn notified(&self) {
        self.notify.notified().await;
    }
}

impl Default for CanteenSchedulerNotifier {
    fn default() -> Self {
        Self::new()
    }
}

pub async fn run_canteen_scheduler(
    canteen_ops: CanteenOperations,
    tz: FixedOffset,
    notifier: CanteenSchedulerNotifier,
) {
    loop {
        let next_close = match tokio::task::spawn_blocking({
            let canteen_ops = canteen_ops.clone();
            move || compute_and_close(canteen_ops, tz)
        })
        .await
        {
            Ok(Ok(next)) => next,
            Ok(Err(e)) => {
                error!("canteen scheduler: error computing close times: {}", e);
                sleep(Duration::from_secs(30)).await;
                continue;
            }
            Err(e) => {
                error!("canteen scheduler: blocking task failed: {}", e);
                sleep(Duration::from_secs(30)).await;
                continue;
            }
        };

        match next_close {
            Some(next) => {
                let now = Utc::now();
                let sleep_for = match next.signed_duration_since(now).to_std() {
                    Ok(d) => d,
                    Err(_) => Duration::from_secs(0),
                };
                tokio::select! {
                    _ = sleep(sleep_for) => {},
                    _ = notifier.notified() => {},
                }
            }
            None => {
                notifier.notified().await;
            }
        }
    }
}

fn compute_and_close(
    canteen_ops: CanteenOperations,
    tz: FixedOffset,
) -> Result<Option<DateTime<Utc>>, crate::db::RepositoryError> {
    let now = Utc::now();
    let mut next_close: Option<DateTime<Utc>> = None;
    let mut due_ids: Vec<i32> = Vec::new();

    let canteens = canteen_ops.list_open_canteens_with_hours()?;
    for canteen in canteens {
        let (opening, closing) = match (canteen.opening_time, canteen.closing_time) {
            (Some(o), Some(c)) => (o, c),
            _ => {
                due_ids.push(canteen.canteen_id);
                continue;
            }
        };

        let opened_at = match canteen.last_opened_at {
            Some(val) => val,
            None => {
                due_ids.push(canteen.canteen_id);
                continue;
            }
        };

        let close_at_local = compute_close_at(opened_at, opening, closing, tz);
        let close_at_utc = close_at_local.with_timezone(&Utc);
        if now >= close_at_utc {
            due_ids.push(canteen.canteen_id);
            continue;
        }

        next_close = match next_close {
            None => Some(close_at_utc),
            Some(existing) => match close_at_utc.cmp(&existing) {
                Ordering::Less => Some(close_at_utc),
                _ => Some(existing),
            },
        };
    }

    if !due_ids.is_empty() {
        let closed = canteen_ops.close_canteens(&due_ids)?;
        info!(
            "canteen scheduler: closed {} canteens (ids={:?})",
            closed, due_ids
        );
    }

    Ok(next_close)
}
