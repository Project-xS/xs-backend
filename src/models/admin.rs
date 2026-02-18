use diesel::{AsChangeset, Identifiable, Insertable, Queryable, Selectable};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

#[derive(Queryable, Debug, Identifiable, Serialize, Deserialize, ToSchema)]
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
}

#[derive(Queryable, Debug, Identifiable, Selectable, Serialize, Deserialize, ToSchema)]
#[diesel(table_name = crate::db::schema::canteens)]
#[diesel(primary_key(canteen_id))]
pub struct CanteenDetails {
    pub canteen_id: i32,
    pub canteen_name: String,
    pub location: String,
    pub has_pic: bool,
    pub pic_etag: Option<String>,
}

#[derive(Insertable, Debug, Serialize, Deserialize, ToSchema)]
#[diesel(table_name = crate::db::schema::canteens)]
pub struct NewCanteen {
    pub canteen_name: String,
    pub location: String,
    pub has_pic: bool,
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
}

#[derive(Debug, Selectable, Queryable, Serialize, ToSchema)]
#[diesel(table_name = crate::db::schema::canteens)]
pub struct CanteenLoginSuccess {
    pub canteen_id: i32,
    pub canteen_name: String,
}
