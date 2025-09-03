use crate::models::admin::{CanteenDetails, MenuItem};
use crate::models::common::OrderItems;
use crate::models::user::PastOrderItem;

pub trait PicKey {
    fn has_pic(&self) -> bool;
    fn pic_key(&self) -> String;
}

impl PicKey for MenuItem {
    fn has_pic(&self) -> bool {
        self.has_pic
    }
    fn pic_key(&self) -> String {
        format!("items/{}", self.item_id)
    }
}

impl PicKey for CanteenDetails {
    fn has_pic(&self) -> bool {
        self.has_pic
    }
    fn pic_key(&self) -> String {
        format!("canteens/{}", self.canteen_id)
    }
}

impl PicKey for OrderItems {
    fn has_pic(&self) -> bool {
        self.has_pic
    }
    fn pic_key(&self) -> String {
        format!("items/{}", self.item_id)
    }
}

impl PicKey for PastOrderItem {
    fn has_pic(&self) -> bool {
        self.has_pic
    }
    fn pic_key(&self) -> String {
        format!("items/{}", self.item_id)
    }
}
