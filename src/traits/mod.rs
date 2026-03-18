use crate::models::admin::{CanteenDetails, MenuItem};
use crate::models::common::OrderItems;
use crate::models::user::PastOrderItem;

pub trait PicKey {
    fn pic_key(&self) -> Option<String>;
}

impl PicKey for MenuItem {
    fn pic_key(&self) -> Option<String> {
        self.pic_key.as_ref().map(|key| format!("items/{key}"))
    }
}

impl PicKey for CanteenDetails {
    fn pic_key(&self) -> Option<String> {
        self.pic_key.as_ref().map(|key| format!("canteens/{key}"))
    }
}

impl PicKey for OrderItems {
    fn pic_key(&self) -> Option<String> {
        self.pic_key.as_ref().map(|key| format!("items/{key}"))
    }
}

impl PicKey for PastOrderItem {
    fn pic_key(&self) -> Option<String> {
        self.pic_key.as_ref().map(|key| format!("items/{key}"))
    }
}
