use crate::db::errors::RepositoryError;
use crate::db::{AssetOperations, DbConnection};
use crate::enums::common::ItemContainer;
use crate::enums::users::{PastOrderItemContainer, PastOrderItemWithPic};
use crate::models::user::{NewUser, PastOrder, PastOrderItem, User};
use chrono::{DateTime, Utc};
use diesel::prelude::*;
use diesel::r2d2::{ConnectionManager, Pool};
use diesel::result::Error;
use futures::future::join_all;
use log::error;
use std::collections::{HashMap, HashSet};

type MenuItemInfo = (i32, String, bool, bool, Option<String>, Option<String>);

pub struct UserOperations {
    pool: Pool<ConnectionManager<PgConnection>>,
    asset_ops: AssetOperations,
}

impl UserOperations {
    pub async fn new(pool: Pool<ConnectionManager<PgConnection>>) -> Self {
        Self {
            pool,
            asset_ops: AssetOperations::new().await.unwrap(),
        }
    }

    pub fn create_user(&self, new_user: NewUser) -> Result<User, RepositoryError> {
        let mut conn = DbConnection::new(&self.pool).map_err(|e| {
            error!("create_user: failed to acquire DB connection: {}", e);
            e
        })?;

        use crate::db::schema::users::dsl::*;

        diesel::insert_into(users)
            .values(&new_user)
            .get_result(conn.connection())
            .map_err(|e| {
                error!(
                    "create_user: error inserting new user with email '{}': {}",
                    new_user.email, e
                );
                RepositoryError::DatabaseError(e)
            })
    }

    #[allow(dead_code)]
    pub fn get_user_by_rfid(&self, rfid_to_get: i32) -> Result<User, RepositoryError> {
        let mut conn = DbConnection::new(&self.pool).map_err(|e| {
            error!(
                "get_user_by_rfid: failed to acquire DB connection for rfid '{}': {}",
                rfid_to_get, e
            );
            e
        })?;

        use crate::db::schema::users::dsl::*;
        users
            .find(rfid_to_get)
            .limit(1)
            .get_result::<User>(conn.connection())
            .map_err(|e| match e {
                Error::NotFound => RepositoryError::NotFound(rfid_to_get.to_string()),
                other => RepositoryError::DatabaseError(other),
            })
    }

    pub fn get_user_by_email(&self, email_addr: &str) -> Result<User, RepositoryError> {
        let mut conn = DbConnection::new(&self.pool)?;

        use crate::db::schema::users::dsl::*;
        users
            .filter(email.eq(email_addr))
            .limit(1)
            .get_result::<User>(conn.connection())
            .map_err(|e| {
                error!(
                    "get_user_by_email: error fetching user with email '{}': {}",
                    email_addr, e
                );
                match e {
                    Error::NotFound => RepositoryError::NotFound(email_addr.to_string()),
                    other => RepositoryError::DatabaseError(other),
                }
            })
    }

    fn group_order_items(items: Vec<PastOrderItemWithPic>) -> Vec<PastOrderItemContainer> {
        debug!("Ungrouped order items: {:?}", &items);
        let mut grouped: HashMap<i32, (i32, bool, DateTime<Utc>, Vec<ItemContainer>)> =
            HashMap::new();

        for item in items {
            let (_, _, _, new_item) = grouped.entry(item.order_id).or_insert_with(|| {
                (
                    item.total_price,
                    item.order_status,
                    item.ordered_at,
                    Vec::new(),
                )
            });

            new_item.push(ItemContainer {
                canteen_id: item.canteen_id,
                item_id: item.item_id,
                name: item.name,
                quantity: item.quantity,
                is_veg: item.is_veg,
                pic_link: item.pic_link,
                pic_etag: item.pic_etag,
                description: item.description,
            });
        }
        debug!("Grouped order items: {:?}", &grouped);

        grouped
            .into_iter()
            .map(
                |(order_id, (total_price, order_status, ordered_at, items))| {
                    let epoch_date = ordered_at.timestamp();
                    PastOrderItemContainer {
                        order_id,
                        items,
                        total_price,
                        order_status,
                        ordered_at: epoch_date,
                    }
                },
            )
            .collect()
    }

    pub async fn get_past_orders_by_userid(
        &self,
        search_user_id: &i32,
    ) -> Result<Vec<PastOrderItemContainer>, RepositoryError> {
        let mut conn = DbConnection::new(&self.pool).map_err(|e| {
            error!(
                "get_past_orders_by_userid: failed to acquire DB connection for user_id {}: {}",
                search_user_id, e
            );
            e
        })?;

        let orders: Vec<PastOrder>;
        {
            use crate::db::schema::past_orders::dsl::*;
            orders = past_orders
                .filter(user_id.eq(search_user_id))
                .order(ordered_at.desc())
                .select((order_id, user_id, items, order_status, ordered_at, price))
                .load::<PastOrder>(conn.connection())
                .map_err(|e| {
                    error!(
                        "get_past_orders_by_userid: error loading order items for user_id {}: {}",
                        search_user_id, e
                    );
                    match e {
                        Error::NotFound => RepositoryError::NotFound(format!(
                            "get_past_orders_by_userid: {search_user_id}"
                        )),
                        other => RepositoryError::DatabaseError(other),
                    }
                })?;
        }

        let mut all_items: HashSet<i32> = HashSet::new();

        orders.iter().for_each(|item| {
            for &item in &item.items {
                if let Some(item_unwrap) = item {
                    all_items.insert(item_unwrap);
                }
            }
        });

        let mut menu_items_in_orders: HashMap<i32, MenuItemInfo> = HashMap::new();
        {
            use crate::db::schema::menu_items::dsl::*;

            let order_items = menu_items
                .select((
                    item_id,
                    canteen_id,
                    name,
                    is_veg,
                    has_pic,
                    pic_etag,
                    description,
                ))
                .filter(item_id.eq_any(all_items))
                .load::<(i32, i32, String, bool, bool, Option<String>, Option<String>)>(
                    conn.connection(),
                )
                .map_err(|e| {
                    error!(
                        "get_past_orders_by_userid: error loading item details for user_id {}: {}",
                        search_user_id, e
                    );
                    match e {
                        Error::NotFound => RepositoryError::NotFound(format!(
                            "get_past_orders_by_userid: {search_user_id}"
                        )),
                        other => RepositoryError::DatabaseError(other),
                    }
                })?;

            order_items.iter().for_each(|item| {
                menu_items_in_orders.entry(item.0).or_insert_with(|| {
                    (
                        item.1,
                        item.2.clone(),
                        item.3,
                        item.4,
                        item.5.clone(),
                        item.6.clone(),
                    )
                });
            })
        }

        let mut orders_items_with_pic: Vec<PastOrderItem> = Vec::new();
        orders.iter().for_each(|order| {
            let mut items_qty: HashMap<i32, i16> = HashMap::new();
            for item_unwrap in order.items.clone().into_iter().flatten() {
                let qty = items_qty.entry(item_unwrap).or_insert(0);
                *qty += 1;
            }
            for item_unwrap in order.items.clone().into_iter().flatten() {
                if !items_qty.contains_key(&item_unwrap) {
                    continue;
                }
                orders_items_with_pic.push(PastOrderItem {
                    order_id: order.order_id,
                    canteen_id: menu_items_in_orders.get(&item_unwrap).unwrap().0,
                    order_status: order.order_status,
                    ordered_at: order.ordered_at,
                    total_price: order.price,
                    item_id: item_unwrap,
                    name: menu_items_in_orders.get(&item_unwrap).unwrap().1.clone(),
                    quantity: *items_qty.get(&item_unwrap).unwrap(),
                    is_veg: menu_items_in_orders.get(&item_unwrap).unwrap().2,
                    has_pic: menu_items_in_orders.get(&item_unwrap).unwrap().3,
                    pic_etag: menu_items_in_orders.get(&item_unwrap).unwrap().4.clone(),
                    description: menu_items_in_orders.get(&item_unwrap).unwrap().5.clone(),
                });
                items_qty.remove(&item_unwrap);
            }
        });

        let futures = orders_items_with_pic.iter().map(async |item| {
            let mut item_with_pic: PastOrderItemWithPic = item.into();
            item_with_pic
                .populate_pic_link_from(&self.asset_ops, item)
                .await;
            item_with_pic
        });

        let results = join_all(futures).await;

        Ok(Self::group_order_items(results))
    }
}

impl Clone for UserOperations {
    fn clone(&self) -> Self {
        Self {
            pool: self.pool.clone(),
            asset_ops: self.asset_ops.clone(),
        }
    }
}
