mod common;

use proj_xs::models::admin::{CanteenDetails, MenuItem};
use proj_xs::traits::PicKey;

#[test]
fn pic_key_menu_item_format() {
    let item = MenuItem {
        item_id: 7,
        canteen_id: 1,
        name: "Test Item".to_string(),
        is_veg: true,
        price: 100,
        stock: 10,
        is_available: true,
        description: None,
        pic_etag: None,
        pic_key: Some("abc-uuid".to_string()),
    };
    assert_eq!(item.pic_key(), Some("items/abc-uuid".to_string()));
}

#[test]
fn pic_key_canteen_details_format() {
    let canteen = CanteenDetails {
        canteen_id: 3,
        canteen_name: "Test Canteen".to_string(),
        location: "Block A".to_string(),
        pic_etag: None,
        pic_key: Some("canteen-uuid".to_string()),
        opening_time: None,
        closing_time: None,
        is_open: true,
    };
    assert_eq!(canteen.pic_key(), Some("canteens/canteen-uuid".to_string()));
}
