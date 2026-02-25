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
        has_pic: true,
        pic_etag: None,
    };
    assert_eq!(item.pic_key(), "items/7");
}

#[test]
fn pic_key_canteen_details_format() {
    let canteen = CanteenDetails {
        canteen_id: 3,
        canteen_name: "Test Canteen".to_string(),
        location: "Block A".to_string(),
        has_pic: true,
        pic_etag: None,
        opening_time: None,
        closing_time: None,
        is_open: true,
    };
    assert_eq!(canteen.pic_key(), "canteens/3");
}
