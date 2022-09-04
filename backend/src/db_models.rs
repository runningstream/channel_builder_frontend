/// The models employed by Diesel and the db module

use crate::schema::{user_data, front_end_sess_keys, channel_list, roku_sess_keys, display_sess_keys};
use chrono::{DateTime, Utc};

/// The components in common for all session keys (roku and frontend, now)
pub struct SessKeyComponents {
    pub id: i32,
    pub userid: i32,
    pub creationtime: DateTime<Utc>,
}

/// The common interface between all session keys
pub trait SessKeyCommon {
    fn get_common(&self) -> SessKeyComponents;
}

#[derive(Queryable)]
pub struct QueryUserData {
    pub id: i32,
    pub username: String,
    pub pass_hash: String,
    pub pass_hash_type: i32,
    pub validation_status: bool,
    pub validation_code: Option<String>,
    pub active_channel: Option<i32>,
}

#[derive(Insertable)]
#[table_name="user_data"]
pub struct InsertUserData<'a> {
    pub username: &'a str,
    pub pass_hash: &'a str,
    pub pass_hash_type: i32,
    pub validation_status: bool,
    pub validation_code: &'a str,
}

#[derive(Queryable)]
pub struct QueryFESessKey {
    pub id: i32,
    pub userid: i32,
    pub sesskey: String,
    pub creationtime: DateTime<Utc>,
    pub lastusedtime: DateTime<Utc>,
}

/// Implements the common session key interface for frontend session keys
impl SessKeyCommon for QueryFESessKey {
    fn get_common(&self) -> SessKeyComponents {
        SessKeyComponents {
            id: self.id,
            userid: self.userid,
            creationtime: self.creationtime,
        }
    }
}

#[derive(Insertable)]
#[table_name="front_end_sess_keys"]
pub struct InsertFESessKey<'a> {
    pub userid: i32,
    pub sesskey: &'a str,
    pub creationtime: DateTime<Utc>,
    pub lastusedtime: DateTime<Utc>,
}

#[derive(Queryable)]
pub struct QueryROSessKey {
    pub id: i32,
    pub userid: i32,
    pub sesskey: String,
    pub creationtime: DateTime<Utc>,
    pub lastusedtime: DateTime<Utc>,
}

/// Implements the common session key interface for roku session keys
impl SessKeyCommon for QueryROSessKey {
    fn get_common(&self) -> SessKeyComponents {
        SessKeyComponents {
            id: self.id,
            userid: self.userid,
            creationtime: self.creationtime,
        }
    }
}

#[derive(Insertable)]
#[table_name="roku_sess_keys"]
pub struct InsertROSessKey<'a> {
    pub userid: i32,
    pub sesskey: &'a str,
    pub creationtime: DateTime<Utc>,
    pub lastusedtime: DateTime<Utc>,
}

#[derive(Queryable)]
pub struct QueryDISessKey {
    pub id: i32,
    pub userid: i32,
    pub sesskey: String,
    pub creationtime: DateTime<Utc>,
    pub lastusedtime: DateTime<Utc>,
}

/// Implements the common session key interface for display session keys
impl SessKeyCommon for QueryDISessKey {
    fn get_common(&self) -> SessKeyComponents {
        SessKeyComponents {
            id: self.id,
            userid: self.userid,
            creationtime: self.creationtime,
        }
    }
}

#[derive(Insertable)]
#[table_name="display_sess_keys"]
pub struct InsertDISessKey<'a> {
    pub userid: i32,
    pub sesskey: &'a str,
    pub creationtime: DateTime<Utc>,
    pub lastusedtime: DateTime<Utc>,
}

#[derive(Queryable)]
pub struct QueryChannelList {
    pub id: i32,
    pub userid: i32,
    pub name: String,
    pub data: String,
}

#[derive(Insertable)]
#[table_name="channel_list"]
pub struct InsertChannelList<'a> {
    pub userid: i32,
    pub name: &'a str,
    pub data: &'a str,
}
