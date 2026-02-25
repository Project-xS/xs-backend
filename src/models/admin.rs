use chrono::{DateTime, NaiveTime, Utc};
use diesel::{AsChangeset, Identifiable, Insertable, Queryable, Selectable};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

#[derive(Queryable, Debug, Identifiable, Serialize, Deserialize)]
#[diesel(table_name = crate::db::schema::canteens)]
#[diesel(primary_key(canteen_id))]
pub struct Canteen {
    pub canteen_id: i32,
    pub canteen_name: String,
    pub location: String,
    pub username: String,
    pub password: String,
    pub has_pic: bool,
    pub pic_etag: Option<String>,
    pub opening_time: Option<NaiveTime>,
    pub closing_time: Option<NaiveTime>,
    pub is_open: bool,
    pub last_opened_at: Option<DateTime<Utc>>,
}

#[derive(Queryable, Debug, Identifiable, Selectable, Serialize, Deserialize)]
#[diesel(table_name = crate::db::schema::canteens)]
#[diesel(primary_key(canteen_id))]
pub struct CanteenDetails {
    pub canteen_id: i32,
    pub canteen_name: String,
    pub location: String,
    pub has_pic: bool,
    pub pic_etag: Option<String>,
    pub opening_time: Option<NaiveTime>,
    pub closing_time: Option<NaiveTime>,
    pub is_open: bool,
}

#[derive(Insertable, Debug, Serialize, Deserialize, ToSchema)]
#[diesel(table_name = crate::db::schema::canteens)]
pub struct NewCanteen {
    pub canteen_name: String,
    pub location: String,
    pub has_pic: bool,
    #[schema(value_type = String, format = "time")]
    pub opening_time: Option<NaiveTime>,
    #[schema(value_type = String, format = "time")]
    pub closing_time: Option<NaiveTime>,
}

#[derive(Insertable, Debug)]
#[diesel(table_name = crate::db::schema::canteens)]
pub struct NewCanteenInsert {
    pub canteen_name: String,
    pub location: String,
    pub has_pic: bool,
    pub opening_time: Option<NaiveTime>,
    pub closing_time: Option<NaiveTime>,
    pub is_open: bool,
    pub last_opened_at: Option<DateTime<Utc>>,
}

#[derive(Queryable, Debug, Identifiable, Serialize, Deserialize, ToSchema, Selectable)]
#[diesel(table_name = crate::db::schema::menu_items)]
#[diesel(primary_key(item_id))]
pub struct MenuItem {
    pub item_id: i32,
    pub canteen_id: i32,
    pub name: String,
    pub is_veg: bool,
    pub price: i32,
    pub stock: i32,
    pub is_available: bool,
    pub description: Option<String>,
    pub has_pic: bool,
    pub pic_etag: Option<String>,
}

#[derive(Insertable, Debug, Serialize, Deserialize, Selectable)]
#[diesel(table_name = crate::db::schema::menu_items)]
pub struct NewMenuItem {
    pub canteen_id: i32,
    pub name: String,
    pub is_veg: bool,
    pub price: i32,
    pub stock: i32,
    pub is_available: bool,
    pub description: Option<String>,
    pub has_pic: bool,
}

#[derive(Debug, Selectable, Queryable)]
#[diesel(table_name = crate::db::schema::menu_items)]
#[diesel(primary_key(item_id))]
pub struct MenuItemCheck {
    pub item_id: i32,
    pub canteen_id: i32,
    pub name: String,
    pub stock: i32,
    pub price: i32,
    pub is_available: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, AsChangeset, ToSchema)]
#[diesel(table_name = crate::db::schema::menu_items)]
pub struct UpdateMenuItem {
    pub name: Option<String>,
    pub is_veg: Option<bool>,
    pub price: Option<i32>,
    pub stock: Option<i32>,
    pub is_available: Option<bool>,
    pub description: Option<String>,
}

pub const MENU_ITEM_NAME_MAX_LEN: usize = 120;
pub const MENU_ITEM_DESC_MAX_LEN: usize = 500;

fn sanitize_name(name: &str) -> Result<String, String> {
    let trimmed = name.trim();
    if trimmed.is_empty() {
        return Err("name must not be empty".to_string());
    }
    if trimmed.chars().count() > MENU_ITEM_NAME_MAX_LEN {
        return Err(format!(
            "name must be at most {MENU_ITEM_NAME_MAX_LEN} characters"
        ));
    }
    Ok(trimmed.to_string())
}

fn sanitize_description(description: &Option<String>) -> Result<Option<String>, String> {
    match description.as_deref() {
        None => Ok(None),
        Some(desc) => {
            let trimmed = desc.trim();
            if trimmed.is_empty() {
                return Ok(None);
            }
            if trimmed.chars().count() > MENU_ITEM_DESC_MAX_LEN {
                return Err(format!(
                    "description must be at most {MENU_ITEM_DESC_MAX_LEN} characters"
                ));
            }
            Ok(Some(trimmed.to_string()))
        }
    }
}

fn validate_price(price: i32) -> Result<(), String> {
    if price <= 0 {
        return Err("price must be greater than 0".to_string());
    }
    Ok(())
}

fn validate_stock(stock: i32) -> Result<(), String> {
    if stock < -1 {
        return Err("stock must be greater than or equal to -1".to_string());
    }
    Ok(())
}

impl NewMenuItem {
    pub fn sanitize_and_validate(mut self) -> Result<Self, String> {
        self.name = sanitize_name(&self.name)?;
        self.description = sanitize_description(&self.description)?;
        validate_price(self.price)?;
        validate_stock(self.stock)?;
        Ok(self)
    }
}

impl UpdateMenuItem {
    pub fn sanitize_and_validate(mut self) -> Result<Self, String> {
        if let Some(name) = self.name.as_ref() {
            self.name = Some(sanitize_name(name)?);
        }
        if let Some(price) = self.price {
            validate_price(price)?;
        }
        if let Some(stock) = self.stock {
            validate_stock(stock)?;
        }
        self.description = sanitize_description(&self.description)?;
        Ok(self)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sanitize_name_trims_and_validates_length() {
        assert_eq!(sanitize_name("  Burger ").unwrap(), "Burger");
        assert!(sanitize_name("   ").is_err());

        let long_name = "a".repeat(MENU_ITEM_NAME_MAX_LEN + 1);
        assert!(sanitize_name(&long_name).is_err());
    }

    #[test]
    fn sanitize_description_trims_and_handles_empty() {
        assert_eq!(sanitize_description(&None).unwrap(), None);
        assert_eq!(
            sanitize_description(&Some("  tasty  ".to_string())).unwrap(),
            Some("tasty".to_string())
        );
        assert_eq!(
            sanitize_description(&Some("   ".to_string())).unwrap(),
            None
        );

        let long_desc = "a".repeat(MENU_ITEM_DESC_MAX_LEN + 1);
        assert!(sanitize_description(&Some(long_desc)).is_err());
    }

    #[test]
    fn validate_price_rejects_non_positive() {
        assert!(validate_price(1).is_ok());
        assert!(validate_price(0).is_err());
        assert!(validate_price(-5).is_err());
    }

    #[test]
    fn validate_stock_rejects_below_negative_one() {
        assert!(validate_stock(0).is_ok());
        assert!(validate_stock(-1).is_ok());
        assert!(validate_stock(-2).is_err());
    }

    #[test]
    fn update_menu_item_sanitize_all_none_passes() {
        let update = UpdateMenuItem {
            name: None,
            is_veg: None,
            price: None,
            stock: None,
            is_available: None,
            description: None,
        };
        let result = update.sanitize_and_validate();
        assert!(result.is_ok());
        let validated = result.unwrap();
        assert!(validated.name.is_none());
        assert!(validated.price.is_none());
        assert!(validated.stock.is_none());
    }

    #[test]
    fn update_menu_item_sanitize_rejects_bad_price() {
        let update = UpdateMenuItem {
            name: None,
            is_veg: None,
            price: Some(0),
            stock: None,
            is_available: None,
            description: None,
        };
        assert!(update.sanitize_and_validate().is_err());
    }

    #[test]
    fn update_menu_item_sanitize_rejects_bad_stock() {
        let update = UpdateMenuItem {
            name: None,
            is_veg: None,
            price: None,
            stock: Some(-2),
            is_available: None,
            description: None,
        };
        assert!(update.sanitize_and_validate().is_err());
    }

    #[test]
    fn update_menu_item_sanitize_trims_name() {
        let update = UpdateMenuItem {
            name: Some("  Burger  ".to_string()),
            is_veg: None,
            price: None,
            stock: None,
            is_available: None,
            description: None,
        };
        let result = update.sanitize_and_validate().unwrap();
        assert_eq!(result.name, Some("Burger".to_string()));
    }

    #[test]
    fn new_menu_item_sanitize_rejects_empty_name() {
        let item = NewMenuItem {
            canteen_id: 1,
            name: "".to_string(),
            is_veg: true,
            price: 100,
            stock: 10,
            is_available: true,
            description: None,
            has_pic: false,
        };
        assert!(item.sanitize_and_validate().is_err());
    }

    #[test]
    fn new_menu_item_sanitize_rejects_long_description() {
        let item = NewMenuItem {
            canteen_id: 1,
            name: "Valid Name".to_string(),
            is_veg: true,
            price: 100,
            stock: 10,
            is_available: true,
            description: Some("a".repeat(MENU_ITEM_DESC_MAX_LEN + 1)),
            has_pic: false,
        };
        assert!(item.sanitize_and_validate().is_err());
    }
}

#[derive(Debug, Selectable, Queryable, Serialize, ToSchema)]
#[diesel(table_name = crate::db::schema::canteens)]
pub struct CanteenLoginSuccess {
    pub canteen_id: i32,
    pub canteen_name: String,
}
