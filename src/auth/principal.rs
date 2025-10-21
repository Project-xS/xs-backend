#[derive(Clone, Debug)]
pub enum Principal {
    User {
        user_id: i32,
        firebase_uid: String,
        email: Option<String>,
    },
    Admin {
        canteen_id: i32,
    },
}

