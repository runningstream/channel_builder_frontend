use serde::{Deserialize, Serialize};

#[derive(Debug,Deserialize,Serialize,Clone)]
pub struct AuthForm {
    pub username: String,
    pub password: String,
}

#[derive(Debug,Deserialize,Serialize,Clone)]
pub struct CreateAcctForm {
    pub username: String,
    pub password: String,
}

#[derive(Debug,Deserialize,Serialize,Clone)]
pub struct SetChannelListForm {
    pub listname: String,
    pub listdata: String,
}

#[derive(Debug,Deserialize,Serialize,Clone)]
pub struct RenameChannelForm {
    pub listname: String,
    pub newlistname: String,
}

#[derive(Debug,Deserialize,Serialize,Clone)]
pub struct CreateChannelListForm {
    pub listname: String,
}

#[derive(Debug,Deserialize,Serialize,Clone)]
pub struct SetActiveChannelForm {
    pub listname: String,
}

#[derive(Debug,Deserialize,Serialize,Clone)]
pub struct DeleteChannelQuery {
    pub listname: String,
}

#[derive(Debug,Deserialize,Serialize,Clone)]
pub struct GetChannelListQuery {
    pub list_name: String,
}

#[derive(Debug,Deserialize,Serialize,Clone)]
pub struct ValidateAccountRequest {
    pub val_code: String,
}
