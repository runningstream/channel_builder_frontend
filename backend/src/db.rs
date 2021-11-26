/// Manages all the database operations

use crate::{db_models, helpers};
use db_models::SessKeyCommon;
use helpers::SessType;
use diesel::pg::PgConnection;
use diesel::Connection;
use diesel::RunQueryDsl;
use std::sync::mpsc;
use tokio::task;
use tokio::sync::oneshot;
use chrono::Utc;
use thiserror::Error;
use crate::diesel::{QueryDsl, ExpressionMethods};
use crate::schema::{user_data, channel_list, front_end_sess_keys, roku_sess_keys};
use crate::schema::user_data::dsl::user_data as ud_dsl;
use crate::schema::channel_list::dsl::channel_list as cl_dsl;
use crate::schema::front_end_sess_keys::dsl::front_end_sess_keys as fesk_dsl;
use crate::schema::roku_sess_keys::dsl::roku_sess_keys as rosk_dsl;

embed_migrations!();

#[derive(Error, Debug)]
pub enum DBError {
    #[error("entry already exists")]
    EntryAlreadyExists,
    #[error("database returned invalid row count {0}")]
    InvalidRowCount(usize),

    #[error("json conversion error {source}")]
    JSONConversionError {
        #[from]
        source: serde_json::Error,
    },
    #[error("error getting result from database thread {source}")]
    ThreadResponseFailure { 
        #[from]
        source: oneshot::error::RecvError,
    },
    #[error("other database error {source}")]
    OtherErr {
        #[from]
        source: diesel::result::Error,
    },
}

#[derive(Clone)]
pub struct Db {
    db_tx: mpsc::SyncSender<Message>,
}

struct InThreadData {
    db_conn: PgConnection,
}

#[derive(Clone)]
pub enum Action {
    AddUser { user: String, pass_hash: String, pass_hash_ver: i32, reg_key: String },
    ValidateAccount { val_code: String },
    AddSessKey { user: String, sess_type: SessType, sess_key: String },
    ValidateSessKey { sess_type: SessType, sess_key: String },
    LogoutSessKey { sess_type: SessType, sess_key: String },
    GetUserPassHash { user: String },
    GetChannelLists { user_id: i32 },
    GetChannelList { user_id: i32, list_name: String },
    SetChannelList { user_id: i32, list_name: String, list_data: String },
    CreateChannelList { user_id: i32, list_name: String },
    GetActiveChannel { user_id: i32 },
    SetActiveChannel { user_id: i32, list_name: String },
}

#[derive(Clone,Debug)]
pub enum Response {
    Empty,
    Bool(bool),
    StringResp(String),
    UserID(i32),
    ValidatedKey(bool, i32),
    UserPassHash(String, i32, bool),
}

struct Message {
    resp: oneshot::Sender<Result<Response, DBError>>,
    action: Action,
}

trait GetCount { fn count(&self) -> usize; }
impl GetCount for usize { fn count(&self) -> usize { *self } }
impl<T> GetCount for Vec<T> { fn count(&self) -> usize { self.len() } }

// Error when the query result is an error, or has a number of rows other than 1
fn allow_only_one<T: GetCount>(qr: diesel::result::QueryResult<T>) ->
        Result<T, DBError>
{
    let unwrapped = qr?;
    match unwrapped.count() {
        1 => Ok(unwrapped),
        val => Err(DBError::InvalidRowCount(val)),
    }
}

impl Db {
    pub fn new(db_url: &str) -> Self {
        let db_conn = match helpers::retry_on_err(5, std::time::Duration::new(5, 0), || {
            PgConnection::establish(db_url)
            },)
        {
            Ok(ret) => ret,
            Err(err) => panic!("Unable to connect to database: {:?}", err),
        };

        match embedded_migrations::run_with_output(&db_conn,
            &mut std::io::stdout()) 
        {
            Ok(_) => {},
            Err(err) => error!("Error during migrations: {:?}", err),
        };

        let in_thread_data = InThreadData {
            db_conn,
        };

        let (db_tx, db_rx) = mpsc::sync_channel(100*1024);

        let _handler_thread = task::spawn_blocking(move || {
            Self::handle_db_calls(in_thread_data, db_rx);
        });

        Self {
            db_tx
        }
    }

    pub async fn please(&self, action: Action) -> Result<Response, DBError> {
        let (tx, rx) = oneshot::channel();

        let msg = Message {
            resp: tx,
            action: action,
        };

        match self.db_tx.send(msg) {
            Ok(_) => rx.await?,
            Err(err) => {panic!("DB request failed! Dying: {}", err);},
        }
    }

    fn handle_db_calls(dat: InThreadData, db_rx: mpsc::Receiver<Message>)
        -> ()
    {
        while let Some(msg) = match db_rx.recv() {
            Ok(msg) => Some(msg),
            Err(_err) => {
                panic!("DB sender disconnected!");
            }
        } {
            let result = match msg.action {
                Action::AddUser { user, pass_hash, pass_hash_ver, reg_key }=>
                    Self::add_user(&dat, user, pass_hash, pass_hash_ver, reg_key),
                Action::ValidateAccount { val_code } =>
                    Self::validate_account(&dat, val_code),
                Action::AddSessKey { user, sess_type, sess_key } =>
                    Self::add_session_key(&dat, user, sess_type, sess_key),
                Action::ValidateSessKey { sess_type, sess_key } =>
                    Self::validate_session_key(&dat, sess_type, sess_key),
                Action::LogoutSessKey { sess_type, sess_key } =>
                    Self::logout_session_key(&dat, sess_type, sess_key),
                Action::GetUserPassHash { user } =>
                    Self::get_user_passhash(&dat, user),
                Action::GetChannelLists { user_id } =>
                    Self::get_channel_lists(&dat, user_id),
                Action::GetChannelList { user_id, list_name } =>
                    Self::get_channel_list(&dat, user_id, list_name),
                Action::SetChannelList { user_id, list_name, list_data } =>
                    Self::set_channel_list(&dat, user_id, list_name, list_data),
                Action::CreateChannelList { user_id, list_name } =>
                    Self::create_channel_list(&dat, user_id, list_name),
                Action::GetActiveChannel { user_id } =>
                    Self::get_active_channel(&dat, user_id),
                Action::SetActiveChannel { user_id, list_name } =>
                    Self::set_active_channel(&dat, user_id, list_name),
            };
            match msg.resp.send(result) {
                Ok(_) => {},
                Err(_) => {
                    error!("Failed to send database response to requestor...");
                },
            };
        }
    }

    fn add_user(dat: &InThreadData, user: String, pass_hash: String,
            pass_hash_ver: i32, reg_key: String)
        -> Result<Response, DBError>
    {
        // Build the new user data
        let new_user = db_models::InsertUserData {
            username: &user,
            pass_hash: &pass_hash,
            pass_hash_type: pass_hash_ver,
            validation_status: false,
            validation_code: &reg_key,
        };

        // Make the database insert
        let user_ids = allow_only_one(
            diesel::insert_into(user_data::table)
                .values(&new_user)
                .returning(user_data::id)
                .get_results(&dat.db_conn)
        )?;

        Ok(Response::UserID(user_ids[0]))
    }

    fn validate_account(dat: &InThreadData, val_code: String)
        -> Result<Response, DBError>
    {
        // Find the user_data that matches the val_code if there is one
        let results = allow_only_one(
            ud_dsl.filter(user_data::validation_code.eq(val_code))
                .limit(5)
                .load::<db_models::QueryUserData>(&dat.db_conn)
        )?;

        // Update it
        allow_only_one(
            diesel::update(ud_dsl.find(results[0].id))
            .set((
                user_data::validation_status.eq(true),
                user_data::validation_code.eq::<Option<String>>(None),
            ))
            .execute(&dat.db_conn))?;

        Ok(Response::Bool(true))
    }

    fn add_session_key(dat: &InThreadData, user: String,
            sess_type: SessType, sess_key: String)
        -> Result<Response, DBError>
    {
        // Generate current time
        let time_now = Utc::now();

        // Find the user_data that matches the username if there is one
        let results = allow_only_one(
            ud_dsl.filter(user_data::username.eq(user))
            .limit(5)
            .load::<db_models::QueryUserData>(&dat.db_conn)
        )?;

        // Build the sess key entry 
        allow_only_one(
            match sess_type {
                SessType::Frontend => {
                    let new_sess = db_models::InsertFESessKey {
                        userid: results[0].id,
                        sesskey: &sess_key,
                        creationtime: time_now,
                        lastusedtime: time_now,
                    };
                    diesel::insert_into(front_end_sess_keys::table)
                        .values(&new_sess)
                        .execute(&dat.db_conn)
                },
                SessType::Roku => {
                    let new_sess = db_models::InsertROSessKey {
                        userid: results[0].id,
                        sesskey: &sess_key,
                        creationtime: time_now,
                        lastusedtime: time_now,
                    };
                    diesel::insert_into(roku_sess_keys::table)
                        .values(&new_sess)
                        .execute(&dat.db_conn)
                },
            }
        )?;

        Ok(Response::Empty)
    }

    fn validate_session_key(dat: &InThreadData, sess_type: SessType,
            sess_key: String)
        -> Result<Response, DBError>
    {
        fn process_filt_result<T: SessKeyCommon>
            (filt_results: Result<Vec<T>, diesel::result::Error>)
            -> Result<db_models::SessKeyComponents, DBError>
        {
            Ok(( allow_only_one(filt_results)? )[0].get_common())
        }

        // Do the database filter, then run process_filt_result
        // That will result in match legs with the same type...
        // Without processing to a common type before returning,
        // the code won't compile because of different leg types.
        let result = match sess_type {
            SessType::Frontend =>
                process_filt_result(
                    fesk_dsl.filter(front_end_sess_keys::sesskey.eq(sess_key))
                        .limit(5)
                        .load::<db_models::QueryFESessKey>(&dat.db_conn)
                ),
            SessType::Roku =>
                process_filt_result(
                    rosk_dsl.filter(roku_sess_keys::sesskey.eq(sess_key))
                        .limit(5)
                        .load::<db_models::QueryROSessKey>(&dat.db_conn)
                ),
        }?;

        // Validate that session key hasn't expired
        let time_now = Utc::now();

        let sess_key_age = time_now.signed_duration_since(
            result.creationtime);
        if sess_key_age > sess_type.get_max_age()
        {
            // Delete sess key
            let del_result = match sess_type {
                SessType::Frontend => 
                    diesel::delete(fesk_dsl.find(result.id))
                        .execute(&dat.db_conn),
                SessType::Roku => 
                    diesel::delete(rosk_dsl.find(result.id))
                        .execute(&dat.db_conn),
            };

            allow_only_one(del_result)?;

            return Ok(Response::ValidatedKey(false, 0));
        }

        // Update last used time
        allow_only_one(
            match sess_type {
                SessType::Frontend =>
                    diesel::update(fesk_dsl.find(result.id))
                        .set((front_end_sess_keys::lastusedtime.eq(time_now),))
                        .execute(&dat.db_conn),
                SessType::Roku =>
                    diesel::update(rosk_dsl.find(result.id))
                        .set((roku_sess_keys::lastusedtime.eq(time_now),))
                        .execute(&dat.db_conn),
            }
        )?;

        Ok(Response::ValidatedKey(true, result.userid))
    }

    fn logout_session_key(dat: &InThreadData, sess_type: SessType,
            sess_key: String)
        -> Result<Response, DBError>
    {
        match sess_type {
            SessType::Frontend => 
                diesel::delete(fesk_dsl.filter(
                        front_end_sess_keys::sesskey.eq(sess_key)
                )).execute(&dat.db_conn),
            SessType::Roku => 
                diesel::delete(rosk_dsl.filter(
                        roku_sess_keys::sesskey.eq(sess_key)
                )).execute(&dat.db_conn),
        }?;

        Ok(Response::Empty)
    }

    fn get_user_passhash(dat: &InThreadData, user: String)
        -> Result<Response, DBError>
    {
        let results = allow_only_one(
            ud_dsl.filter(user_data::username.eq(user))
                .limit(5)
                .load::<db_models::QueryUserData>(&dat.db_conn)
        )?;
        
        Ok(Response::UserPassHash(
            results[0].pass_hash.clone(),
            results[0].pass_hash_type,
            results[0].validation_status
        ))
    }

    fn get_channel_lists(dat: &InThreadData, user_id: i32)
        -> Result<Response, DBError>
    {
        let results = cl_dsl
            .filter(channel_list::userid.eq(user_id))
            .load::<db_models::QueryChannelList>(&dat.db_conn)?;
        
        let channel_names: Vec<String> = results.iter().map(|result| {
            result.name.clone()
        }).collect();

        Ok(Response::StringResp(serde_json::to_string(&channel_names)?))
    }

    fn get_channel_list(dat: &InThreadData, user_id: i32, list_name: String)
        -> Result<Response, DBError>
    {
        // Get the channel
        let results = allow_only_one(
            cl_dsl
                .filter(channel_list::userid.eq(user_id))
                .filter(channel_list::name.eq(&list_name))
                .limit(5)
                .load::<db_models::QueryChannelList>(&dat.db_conn)
        )?;

        Ok(Response::StringResp(results[0].data.clone()))
    }

    fn set_channel_list(dat: &InThreadData, user_id: i32, list_name: String,
        list_data: String)
        -> Result<Response, DBError>
    {
        allow_only_one(
            diesel::update(cl_dsl
                .filter(channel_list::userid.eq(user_id))
                .filter(channel_list::name.eq(&list_name))
            )
            .set(channel_list::data.eq(list_data))
            .execute(&dat.db_conn)
        )?;

        Ok(Response::Empty)
    }

    fn create_channel_list(dat: &InThreadData, user_id: i32, list_name: String)
        -> Result<Response, DBError>
    {
        // See if the channel already exists
        match cl_dsl
            .filter(channel_list::userid.eq(user_id))
            .filter(channel_list::name.eq(&list_name))
            .first::<db_models::QueryChannelList>(&dat.db_conn)
        {
            Ok(_) => Err(DBError::EntryAlreadyExists),
            Err(diesel::result::Error::NotFound) => Ok(()),
            Err(err) => Err(err.into()),
        }?;

        // If it doesn't, create it
        let new_channel = db_models::InsertChannelList {
            userid: user_id,
            name: &list_name,
            data: "{\"entries\": []}",
        };

        // Insert it
        allow_only_one(
            diesel::insert_into(channel_list::table)
                .values(&new_channel)
                .execute(&dat.db_conn)
        )?;

        Ok(Response::Empty)
    }

    fn get_active_channel(dat: &InThreadData, user_id: i32)
        -> Result<Response, DBError>
    {
        joinable!(user_data -> channel_list (active_channel) );

        let results = allow_only_one(
            channel_list::table
                .filter(channel_list::userid.eq(user_id))
                .inner_join(user_data::table)
                .select(channel_list::data)
                .limit(5)
                .load::<String>(&dat.db_conn)
        )?;

        Ok(Response::StringResp(results[0].clone()))
    }

    fn set_active_channel(dat: &InThreadData, user_id: i32, list_name: String)
        -> Result<Response, DBError>
    {
        // Get the channel
        let results = allow_only_one(
            cl_dsl
                .filter(channel_list::userid.eq(user_id))
                .filter(channel_list::name.eq(&list_name))
                .limit(5)
                .load::<db_models::QueryChannelList>(&dat.db_conn)
        )?;

        // Update the user to reflect the id
        allow_only_one(
            diesel::update(ud_dsl.find(user_id))
                .set(user_data::active_channel.eq(results[0].id))
                .execute(&dat.db_conn)
        )?;

        Ok(Response::Empty)
    }
}
