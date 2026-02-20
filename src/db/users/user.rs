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

type MenuItemInfo = (String, String, bool, bool, Option<String>, Option<String>);

#[derive(Debug)]
struct GroupedPastOrder {
    total_price: i32,
    order_status: bool,
    ordered_at: DateTime<Utc>,
    canteen_name: String,
    items: Vec<ItemContainer>,
}

pub struct UserOperations {
    pool: Pool<ConnectionManager<PgConnection>>,
    asset_ops: AssetOperations,
}

impl UserOperations {
    pub async fn new(
        pool: Pool<ConnectionManager<PgConnection>>,
        asset_ops: AssetOperations,
    ) -> Self {
        Self { pool, asset_ops }
    }

    #[allow(dead_code)]
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

    #[allow(dead_code)]
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
        let mut grouped: HashMap<i32, GroupedPastOrder> = HashMap::new();

        for item in items {
            let new_item = &mut grouped
                .entry(item.order_id)
                .or_insert_with(|| GroupedPastOrder {
                    total_price: item.total_price,
                    order_status: item.order_status,
                    ordered_at: item.ordered_at,
                    canteen_name: item.canteen_name.clone(),
                    items: Vec::new(),
                })
                .items;

            new_item.push(ItemContainer {
                name: item.name,
                quantity: item.quantity,
                price: None,
                is_veg: item.is_veg,
                pic_link: item.pic_link,
                pic_etag: item.pic_etag,
                description: item.description,
            });
        }
        debug!("Grouped order items: {:?}", &grouped);

        grouped
            .into_iter()
            .map(|(order_id, grouped_order)| {
                let epoch_date = grouped_order.ordered_at.timestamp();
                PastOrderItemContainer {
                    order_id,
                    canteen_name: grouped_order.canteen_name,
                    items: grouped_order.items,
                    total_price: grouped_order.total_price,
                    order_status: grouped_order.order_status,
                    ordered_at: epoch_date,
                }
            })
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
                .inner_join(
                    crate::db::schema::canteens::table
                        .on(crate::db::schema::canteens::canteen_id.eq(canteen_id)),
                )
                .select((
                    item_id,
                    crate::db::schema::canteens::canteen_name,
                    name,
                    is_veg,
                    has_pic,
                    pic_etag,
                    description,
                ))
                .filter(item_id.eq_any(all_items))
                .load::<(
                    i32,
                    String,
                    String,
                    bool,
                    bool,
                    Option<String>,
                    Option<String>,
                )>(conn.connection())
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
                        item.1.clone(),
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
                    canteen_name: menu_items_in_orders.get(&item_unwrap).unwrap().0.clone(),
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

    pub fn upsert_firebase_user(
        &self,
        uid: String,
        email_opt: Option<String>,
        display_name_opt: Option<String>,
        photo_url_opt: Option<String>,
        email_verified_flag: bool,
    ) -> Result<User, RepositoryError> {
        let mut conn = DbConnection::new(&self.pool).map_err(|e| {
            error!(
                "upsert_firebase_user: failed to acquire DB connection: {}",
                e
            );
            e
        })?;

        use crate::db::schema::users::dsl as u;

        let email_val = email_opt.ok_or_else(|| {
            RepositoryError::ValidationError(
                "Email missing in Firebase token for google provider".to_string(),
            )
        })?;
        let name_val = display_name_opt
            .clone()
            .unwrap_or_else(|| email_val.clone());

        diesel::insert_into(u::users)
            .values((
                u::firebase_uid.eq(&uid),
                u::rfid.eq::<Option<String>>(None),
                u::name.eq(&name_val),
                u::email.eq(&email_val),
                u::auth_provider.eq("google"),
                u::email_verified.eq(email_verified_flag),
                u::display_name.eq(display_name_opt.clone()),
                u::photo_url.eq(photo_url_opt.clone()),
            ))
            .on_conflict(u::firebase_uid)
            .do_update()
            .set((
                u::name.eq(&name_val),
                u::email.eq(&email_val),
                u::auth_provider.eq("google"),
                u::email_verified.eq(email_verified_flag),
                u::display_name.eq(display_name_opt),
                u::photo_url.eq(photo_url_opt),
            ))
            .get_result::<User>(conn.connection())
            .map_err(RepositoryError::DatabaseError)
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
