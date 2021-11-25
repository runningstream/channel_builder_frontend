/// Manages all the database operations

use crate::{db_models, helpers};
use db_models::SessKeyCommon;
use helpers::SessType;
use diesel::pg::PgConnection;
use diesel::Connection;
use diesel::RunQueryDsl;
use diesel::result::Error::NotFound;
use std::{error, fmt};
use std::sync::mpsc;
use tokio::task;
use tokio::sync::oneshot;
use chrono::Utc;
use crate::diesel::{QueryDsl, ExpressionMethods};
use crate::schema::{user_data, channel_list, front_end_sess_keys, roku_sess_keys};
use crate::schema::user_data::dsl::user_data as ud_dsl;
use crate::schema::channel_list::dsl::channel_list as cl_dsl;
use crate::schema::front_end_sess_keys::dsl::front_end_sess_keys as fesk_dsl;
use crate::schema::roku_sess_keys::dsl::roku_sess_keys as rosk_dsl;

embed_migrations!();

#[derive(Debug)]
pub enum DBError {
    PassHashError,
    InvalidDBResponse,
    InvalidValidationCode,
    InvalidUsername,
    JSONConversionError,
    EntryAlreadyExists,
    NoEntryReturned,
    ThreadResponseFailure,
}

impl fmt::Display for DBError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "Error: {:?}", *self)
    }
}

// TODO - implement sources for these if/where appropriate
impl error::Error for DBError {
    fn source(&self) -> Option<&(dyn error::Error + 'static)> {
        match *self {
            _ => None,
        }
    }
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
            Ok(_) => {
                match rx.await {
                    Ok(result) => result,
                    Err(err) => {
                        error!("Error getting result from database thread: {}", err);
                        Err(DBError::ThreadResponseFailure)
                    }
                }
            },
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
        match diesel::insert_into(user_data::table)
            .values(&new_user)
            .returning(user_data::id)
            .get_results(&dat.db_conn)
        {
            //Ok(1) => Ok(Response::Empty), // From .execute, TODO delete
            Ok(user_ids) => {
                if user_ids.len() != 1 {
                    warn!("Adding user returned other-than 1 row: {:?}", user_ids);
                    Err(DBError::InvalidDBResponse)
                } else {
                    Ok(Response::UserID(user_ids[0]))
                }
            },
            Err(err) => {
                warn!("Error {:?}", err);
                Err(DBError::InvalidDBResponse)},
        }
    }

    fn validate_account(dat: &InThreadData, val_code: String)
        -> Result<Response, DBError>
    {
        // Find the user_data that matches the val_code if there is one
        let results = match ud_dsl.filter(user_data::validation_code.eq(val_code))
            .limit(5)
            .load::<db_models::QueryUserData>(&dat.db_conn)
        {
            Ok(vals) => vals,
            Err(err) => {
                warn!("Error getting validation code: {}", err);
                return Err(DBError::InvalidValidationCode);},
        };

        // Make sure the returned values make a little sense
        match results.len() {
            0 => {
                return Err(DBError::InvalidValidationCode);
            },
            1 => {},
            _ => {
                warn!("Error with validate account db results: {}", results.len());
                return Err(DBError::InvalidDBResponse);
            },
        };

        // Grab the ID
        let id = results[0].id;

        // Update it
        match diesel::update(ud_dsl.find(id))
            .set((
                user_data::validation_status.eq(true),
                user_data::validation_code.eq::<Option<String>>(None),
            ))
            .execute(&dat.db_conn)
        {
            Ok(1) => Ok(Response::Bool(true)),
            Ok(val) => {
                warn!("Updating status returned other-than 1: {}", val);
                Err(DBError::InvalidDBResponse)},
            Err(err) => {
                warn!("Error {:?}", err);
                Err(DBError::InvalidDBResponse)},
        }
    }

    fn add_session_key(dat: &InThreadData, user: String,
            sess_type: SessType, sess_key: String)
        -> Result<Response, DBError>
    {
        // Generate current time
        let time_now = Utc::now();

        // Find the user_data that matches the username if there is one
        let results = match ud_dsl.filter(user_data::username.eq(user))
            .limit(5)
            .load::<db_models::QueryUserData>(&dat.db_conn)
        {
            Ok(vals) => vals,
            Err(err) => {
                warn!("Error getting username: {}", err);
                return Err(DBError::InvalidUsername);},
        };

        // Make sure the returned values make a little sense
        match results.len() {
            0 => {
                return Err(DBError::InvalidUsername);
            },
            1 => {},
            _ => {
                warn!(
                    "Error with add session key account db results: {}",
                    results.len());
                return Err(DBError::InvalidDBResponse);
            },
        };

        // Build the sess key entry 
        let result = match sess_type {
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
        };

        match result
        {
            Ok(1) => Ok(Response::Empty),
            Ok(val) => {
                warn!("Adding sess key other-than 1: {}", val);
                Err(DBError::InvalidDBResponse)},
            Err(err) => {
                warn!("Error {:?}", err);
                Err(DBError::InvalidDBResponse)},
        }
    }

    fn validate_session_key(dat: &InThreadData, sess_type: SessType,
            sess_key: String)
        -> Result<Response, DBError>
    {
        fn process_filt_result<T: SessKeyCommon>
            (filt_results: Result<Vec<T>, diesel::result::Error>)
            -> Result<db_models::SessKeyComponents, DBError>
        {
            let results = match filt_results
            {
                Ok(vals) => vals,
                Err(err) => {
                    warn!("Error getting session key: {}", err);
                    return Err(DBError::InvalidDBResponse);
                },
            };
            
            if results.len() != 1 {
                warn!("Error with session key db results: {}", results.len());
                return Err(DBError::InvalidDBResponse);
            }

            Ok(results[0].get_common())
        }

        // Do the database filter, then run process_filt_result
        // That will result in match legs with the same type...
        // Without processing to a common type before returning,
        // the code won't compile because of different leg types.
        let processed_result = match sess_type {
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
        };
        let result = match processed_result {
            Ok(val) => val,
            Err(err) => return Err(err),
        };

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
            return match del_result {
                // Return failed session key
                Ok(1) => Ok(Response::ValidatedKey(false, 0)),
                Ok(val) => {
                    warn!("Updating lastusedtime returned other-than 1: {}", val);
                    Err(DBError::InvalidDBResponse)},
                Err(err) => {
                    warn!("Error updating lastusedtime {:?}", err);
                    Err(DBError::InvalidDBResponse)},
            };
        }

        // Update last used time
        let upd_res = match sess_type {
            SessType::Frontend =>
                diesel::update(fesk_dsl.find(result.id))
                    .set((front_end_sess_keys::lastusedtime.eq(time_now),))
                    .execute(&dat.db_conn),
            SessType::Roku =>
                diesel::update(rosk_dsl.find(result.id))
                    .set((roku_sess_keys::lastusedtime.eq(time_now),))
                    .execute(&dat.db_conn),
        };

        match upd_res {
            Ok(1) => Ok(Response::ValidatedKey(true, result.userid)),
            Ok(val) => {
                warn!("Updating lastusedtime returned other-than 1: {}", val);
                Err(DBError::InvalidDBResponse)},
            Err(err) => {
                warn!("Error updating lastusedtime {:?}", err);
                Err(DBError::InvalidDBResponse)},
        }
    }

    fn logout_session_key(dat: &InThreadData, sess_type: SessType,
            sess_key: String)
        -> Result<Response, DBError>
    {
        let result = match sess_type {
            SessType::Frontend => 
                diesel::delete(fesk_dsl.filter(
                        front_end_sess_keys::sesskey.eq(sess_key)
                )).execute(&dat.db_conn),
            SessType::Roku => 
                diesel::delete(rosk_dsl.filter(
                        roku_sess_keys::sesskey.eq(sess_key)
                )).execute(&dat.db_conn),
        };
        match result {
            // Return failed session key
            Ok(_) => Ok(Response::Empty),
            Err(err) => {
                warn!("Error updating lastusedtime {:?}", err);
                Err(DBError::InvalidDBResponse)},
        }
    }

    fn get_user_passhash(dat: &InThreadData, user: String)
        -> Result<Response, DBError>
    {
        let results = match ud_dsl.filter(user_data::username.eq(user))
            .limit(5)
            .load::<db_models::QueryUserData>(&dat.db_conn)
        {
            Ok(vals) => vals,
            Err(err) => {
                warn!("Error getting user pass hash: {}", err);
                return Err(DBError::InvalidDBResponse);},
        };
        
        if results.len() != 1 {
            warn!("Error with user pass hash db results: {}", results.len());
            return Err(DBError::InvalidDBResponse);
        }

        Ok(Response::UserPassHash(
            results[0].pass_hash.clone(),
            results[0].pass_hash_type,
            results[0].validation_status
        ))
    }

    fn get_channel_lists(dat: &InThreadData, user_id: i32)
        -> Result<Response, DBError>
    {
        let results = match cl_dsl
            .filter(channel_list::userid.eq(user_id))
            .load::<db_models::QueryChannelList>(&dat.db_conn)
        {
            Ok(vals) => vals,
            Err(err) => {
                warn!("Error getting channel lists: {}", err);
                return Err(DBError::InvalidDBResponse);
            },
        };
        
        let channel_names: Vec<String> = results.iter().map(|result| {
            result.name.clone()
        }).collect();

        match serde_json::to_string(&channel_names) {
            Ok(val) => Ok(Response::StringResp(val)),
            Err(err) => {
                warn!("Error converting channel_names to JSON: {}", err);
                return Err(DBError::JSONConversionError);
            },
        }
    }

    fn get_channel_list(dat: &InThreadData, user_id: i32, list_name: String)
        -> Result<Response, DBError>
    {
        // Get the channel
        let results = match cl_dsl
            .filter(channel_list::userid.eq(user_id))
            .filter(channel_list::name.eq(&list_name))
            .limit(5)
            .load::<db_models::QueryChannelList>(&dat.db_conn)
        {
            Ok(vals) => vals,
            Err(err) => {
                warn!("Error getting channel list: {}", err);
                return Err(DBError::InvalidDBResponse);
            },
        };

        // Make sure we got only one
        if results.len() != 1 {
            warn!(
                concat!("Error with channel list db results: ",
                    "user {}, list {}, result count {}"),
                user_id, list_name, results.len()
            );
            return Err(DBError::InvalidDBResponse);
        }
        
        Ok(Response::StringResp(results[0].data.clone()))
    }

    fn set_channel_list(dat: &InThreadData, user_id: i32, list_name: String,
        list_data: String)
        -> Result<Response, DBError>
    {
        match diesel::update(cl_dsl
                .filter(channel_list::userid.eq(user_id))
                .filter(channel_list::name.eq(&list_name))
            )
            .set(channel_list::data.eq(list_data))
            .execute(&dat.db_conn)
        {
            Ok(1) => Ok(Response::Empty),
            Ok(val) => {
                warn!(concat!(
                        "Updating channel list returned other-than 1: ",
                        "userid {} list {} count {}"),
                    user_id, list_name, val
                );
                Err(DBError::InvalidDBResponse)},
            Err(err) => {
                warn!(concat!("Error updating channel list ",
                        "userid {} list {} err {:?}"),
                    user_id, list_name, err
                );
                Err(DBError::InvalidDBResponse)},
        }
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
            Ok(_) => {
                info!("Error creating channel - already exists",);
                return Err(DBError::EntryAlreadyExists);
            },
            Err(NotFound) => {},
            Err(err) => {
                warn!("Error creating channel: {}", err);
                return Err(DBError::InvalidDBResponse);
            },
        };

        // If it doesn't, create it
        let new_channel = db_models::InsertChannelList {
            userid: user_id,
            name: &list_name,
            data: "{\"entries\": []}",
        };

        // Insert it
        match diesel::insert_into(channel_list::table)
            .values(&new_channel)
            .execute(&dat.db_conn)
        {
            Ok(1) => Ok(Response::Empty),
            Ok(val) => {
                warn!("Adding channel returned other-than 1: {}", val);
                Err(DBError::InvalidDBResponse)},
            Err(err) => {
                warn!("Error {:?}", err);
                Err(DBError::InvalidDBResponse)},
        }
    }

    fn get_active_channel(dat: &InThreadData, user_id: i32)
        -> Result<Response, DBError>
    {
        joinable!(user_data -> channel_list (active_channel) );

        let results = match channel_list::table
            .filter(channel_list::userid.eq(user_id))
            .inner_join(user_data::table)
            //.filter(user_data::id.eq(user_id))
            .select(channel_list::data)
            .limit(5)
            .load::<String>(&dat.db_conn)
        {
            Ok(vals) => vals,
            Err(err) => {
                info!("Error getting user pass hash: {}", err);
                return Err(DBError::InvalidDBResponse);},
        };

        // Make sure the returned values make a little sense
        match results.len() {
            0 => {
                return Err(DBError::NoEntryReturned);
            },
            1 => {},
            _ => {
                warn!("Error with validate account db results: {}", results.len());
                return Err(DBError::InvalidDBResponse);
            },
        };

        Ok(Response::StringResp(results[0].clone()))
    }

    fn set_active_channel(dat: &InThreadData, user_id: i32, list_name: String)
        -> Result<Response, DBError>
    {
        // Get the channel
        let results = match cl_dsl
            .filter(channel_list::userid.eq(user_id))
            .filter(channel_list::name.eq(&list_name))
            .limit(5)
            .load::<db_models::QueryChannelList>(&dat.db_conn)
        {
            Ok(vals) => vals,
            Err(err) => {
                warn!("Error getting channel: {}", err);
                return Err(DBError::InvalidDBResponse);
            },
        };

        // Make sure we got only one
        if results.len() != 1 {
            warn!(
                concat!("Error with channel list db results: ",
                    "user {}, list {}, result count {}"),
                user_id, list_name, results.len()
            );
            return Err(DBError::InvalidDBResponse);
        }

        // Update the user to reflect the id
        match diesel::update(ud_dsl.find(user_id))
            .set(user_data::active_channel.eq(results[0].id))
            .execute(&dat.db_conn)
        {
            Ok(1) => Ok(Response::Empty),
            Ok(val) => {
                warn!(concat!(
                        "Updating active channel returned other-than 1: ",
                        "userid {} list {} count {}"),
                    user_id, list_name, val
                );
                Err(DBError::InvalidDBResponse)},
            Err(err) => {
                warn!(concat!("Error active channel ",
                        "userid {} list {} err {:?}"),
                    user_id, list_name, err
                );
                Err(DBError::InvalidDBResponse)},
        }
    }
}
