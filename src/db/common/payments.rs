use crate::db::{DbConnection, RepositoryError};
use crate::models::common::{NewPaymentOrder, PaymentOrder};
use chrono::{DateTime, Utc};
use diesel::prelude::*;
use diesel::r2d2::{ConnectionManager, Pool};
use diesel::result::{DatabaseErrorKind, Error};
use diesel::PgConnection;
use log::{debug, error, warn};

pub const PAYMENT_STATE_COMPLETED: &str = "COMPLETED";
pub const PAYMENT_STATE_FAILED: &str = "FAILED";

#[derive(Debug, Clone)]
pub struct HoldPaymentSnapshot {
    pub hold_id: i32,
    pub user_id: i32,
    pub total_price: i32,
    pub expires_at: DateTime<Utc>,
    pub remaining_secs: i64,
}

#[derive(Clone)]
pub struct PaymentOperations {
    pool: Pool<ConnectionManager<PgConnection>>,
}

impl PaymentOperations {
    pub async fn new(pool: Pool<ConnectionManager<PgConnection>>) -> Self {
        Self { pool }
    }

    pub fn is_terminal_state(state: &str) -> bool {
        state == PAYMENT_STATE_COMPLETED || state == PAYMENT_STATE_FAILED
    }

    pub fn get_hold_snapshot_for_user(
        &self,
        search_hold_id: i32,
        requesting_user_id: i32,
    ) -> Result<HoldPaymentSnapshot, RepositoryError> {
        let mut conn = DbConnection::new(&self.pool).map_err(|e| {
            error!(
                "get_hold_snapshot_for_user: failed to acquire DB connection: {}",
                e
            );
            e
        })?;

        use crate::db::schema::held_orders::dsl::*;
        let (hold_user_id, hold_total_price, hold_expires_at) = held_orders
            .filter(hold_id.eq(search_hold_id))
            .select((user_id, total_price, expires_at))
            .first::<(i32, i32, DateTime<Utc>)>(conn.connection())
            .map_err(|e| match e {
                Error::NotFound => {
                    RepositoryError::NotFound(format!("Hold {} not found", search_hold_id))
                }
                other => RepositoryError::DatabaseError(other),
            })?;

        if hold_user_id != requesting_user_id {
            return Err(RepositoryError::ValidationError(
                "Hold expired or does not belong to user.".to_string(),
            ));
        }

        let now = Utc::now();
        if now > hold_expires_at {
            return Err(RepositoryError::ValidationError(
                "Hold expired or does not belong to user.".to_string(),
            ));
        }

        Ok(HoldPaymentSnapshot {
            hold_id: search_hold_id,
            user_id: hold_user_id,
            total_price: hold_total_price,
            expires_at: hold_expires_at,
            remaining_secs: (hold_expires_at - now).num_seconds().max(0),
        })
    }

    pub fn find_active_mapping_by_hold_id(
        &self,
        search_hold_id: i32,
    ) -> Result<Option<PaymentOrder>, RepositoryError> {
        let mut conn = DbConnection::new(&self.pool).map_err(|e| {
            error!(
                "find_active_mapping_by_hold_id: failed to acquire DB connection: {}",
                e
            );
            e
        })?;

        use crate::db::schema::payment_orders::dsl::*;
        payment_orders
            .filter(hold_id.eq(search_hold_id))
            .filter(payment_state.ne(PAYMENT_STATE_COMPLETED))
            .filter(payment_state.ne(PAYMENT_STATE_FAILED))
            .first::<PaymentOrder>(conn.connection())
            .optional()
            .map_err(RepositoryError::DatabaseError)
    }

    pub fn get_mapping_by_hold_id(
        &self,
        search_hold_id: i32,
    ) -> Result<Option<PaymentOrder>, RepositoryError> {
        let mut conn = DbConnection::new(&self.pool).map_err(|e| {
            error!(
                "get_mapping_by_hold_id: failed to acquire DB connection: {}",
                e
            );
            e
        })?;

        use crate::db::schema::payment_orders::dsl::*;
        payment_orders
            .filter(hold_id.eq(search_hold_id))
            .first::<PaymentOrder>(conn.connection())
            .optional()
            .map_err(RepositoryError::DatabaseError)
    }

    pub fn get_mapping_by_merchant_order_id(
        &self,
        search_merchant_order_id: &str,
    ) -> Result<Option<PaymentOrder>, RepositoryError> {
        let mut conn = DbConnection::new(&self.pool).map_err(|e| {
            error!(
                "get_mapping_by_merchant_order_id: failed to acquire DB connection: {}",
                e
            );
            e
        })?;

        use crate::db::schema::payment_orders::dsl::*;
        payment_orders
            .filter(merchant_order_id.eq(search_merchant_order_id))
            .first::<PaymentOrder>(conn.connection())
            .optional()
            .map_err(RepositoryError::DatabaseError)
    }

    pub fn create_mapping(
        &self,
        mapping: NewPaymentOrder,
    ) -> Result<PaymentOrder, RepositoryError> {
        let mut conn = DbConnection::new(&self.pool).map_err(|e| {
            error!("create_mapping: failed to acquire DB connection: {}", e);
            e
        })?;

        use crate::db::schema::payment_orders::dsl::*;
        match diesel::insert_into(payment_orders)
            .values(&mapping)
            .get_result::<PaymentOrder>(conn.connection())
        {
            Ok(inserted) => {
                debug!(
                    "create_mapping: created payment mapping for hold {} merchant_order_id {}",
                    inserted.hold_id, inserted.merchant_order_id
                );
                Ok(inserted)
            }
            Err(Error::DatabaseError(DatabaseErrorKind::UniqueViolation, _)) => {
                warn!(
                    "create_mapping: unique violation for hold {} merchant_order_id {}",
                    mapping.hold_id, mapping.merchant_order_id
                );
                self.get_mapping_by_hold_id(mapping.hold_id)?
                    .ok_or_else(|| {
                        RepositoryError::ValidationError(
                            "Failed to create payment mapping due to duplicate order.".to_string(),
                        )
                    })
            }
            Err(other) => Err(RepositoryError::DatabaseError(other)),
        }
    }

    pub fn get_mapping_for_user_verify(
        &self,
        search_hold_id: i32,
        requesting_user_id: i32,
        search_merchant_order_id: &str,
    ) -> Result<PaymentOrder, RepositoryError> {
        let mut conn = DbConnection::new(&self.pool).map_err(|e| {
            error!(
                "get_mapping_for_user_verify: failed to acquire DB connection: {}",
                e
            );
            e
        })?;

        use crate::db::schema::payment_orders::dsl::*;
        payment_orders
            .filter(hold_id.eq(search_hold_id))
            .filter(user_id.eq(requesting_user_id))
            .filter(merchant_order_id.eq(search_merchant_order_id))
            .first::<PaymentOrder>(conn.connection())
            .map_err(|e| match e {
                Error::NotFound => {
                    RepositoryError::ValidationError("Hold payment mapping mismatch.".to_string())
                }
                other => RepositoryError::DatabaseError(other),
            })
    }

    pub fn update_mapping_state(
        &self,
        search_merchant_order_id: &str,
        new_state: &str,
        order_id_to_set: Option<i32>,
    ) -> Result<PaymentOrder, RepositoryError> {
        let mut conn = DbConnection::new(&self.pool).map_err(|e| {
            error!(
                "update_mapping_state: failed to acquire DB connection: {}",
                e
            );
            e
        })?;

        use crate::db::schema::payment_orders::dsl::*;
        let now = Utc::now();

        let updated = if let Some(order_id_val) = order_id_to_set {
            diesel::update(payment_orders.filter(merchant_order_id.eq(search_merchant_order_id)))
                .set((
                    payment_state.eq(new_state),
                    app_order_id.eq(Some(order_id_val)),
                    updated_at.eq(now),
                ))
                .get_result::<PaymentOrder>(conn.connection())
        } else {
            diesel::update(payment_orders.filter(merchant_order_id.eq(search_merchant_order_id)))
                .set((payment_state.eq(new_state), updated_at.eq(now)))
                .get_result::<PaymentOrder>(conn.connection())
        };

        updated.map_err(|e| match e {
            Error::NotFound => RepositoryError::NotFound(format!(
                "Payment mapping not found for merchant_order_id {}",
                search_merchant_order_id
            )),
            other => RepositoryError::DatabaseError(other),
        })
    }
}
