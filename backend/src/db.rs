/// Manages all the database operations

use std::fmt;
use std::sync::mpsc;
use crate::{db_models, helpers};
use db_models::SessKeyCommon;
use helpers::SessType;
use diesel::pg::PgConnection;
use diesel::Connection;
use diesel::RunQueryDsl;
use tokio::task;
use tokio::sync::oneshot;
use chrono::Utc;
use thiserror::Error;
use crate::diesel::{QueryDsl, ExpressionMethods};
use crate::schema::{user_data, channel_list, front_end_sess_keys, roku_sess_keys, display_sess_keys};
use crate::schema::user_data::dsl::user_data as ud_dsl;
use crate::schema::channel_list::dsl::channel_list as cl_dsl;
use crate::schema::front_end_sess_keys::dsl::front_end_sess_keys as fesk_dsl;
use crate::schema::roku_sess_keys::dsl::roku_sess_keys as rosk_dsl;
use crate::schema::display_sess_keys::dsl::display_sess_keys as disk_dsl;

embed_migrations!();

#[derive(Error, Debug)]
pub enum DBError {
    #[error("entry already exists")]
    EntryAlreadyExists,
    #[error("database returned invalid row count: {0}")]
    InvalidRowCount(usize),

    #[error("json conversion error: {source}")]
    JSONConversionError {
        #[from]
        source: serde_json::Error,
    },
    #[error("error getting result from database thread: {source}")]
    ThreadResponseFailure { 
        #[from]
        source: oneshot::error::RecvError,
    },
    #[error("other database error: {source}")]
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

#[derive(Debug, Copy, Clone, Default)]
pub struct StatusReport {
    // Counts of times the function was triggered
    add_user: u32,
    validate_account: u32,
    add_session_key: u32,
    validate_session_key: u32,
    logout_session_key: u32,
    get_user_passhash: u32,
    get_channel_lists: u32,
    get_channel_list: u32,
    set_channel_list: u32,
    create_channel_list: u32,
    get_active_channel: u32,
    set_active_channel: u32,
    get_status_report: u32,
    
    // Counts of other things
    add_user_success: u32,
    validate_acct_success: u32,
    add_sess_key_success: u32,
    validate_sess_key_success: u32,
}

impl fmt::Display for StatusReport {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, concat!(
                "Database status report:\n",
                "  Add User: {}\n",
                "    Success: {}\n",
                "  Validate User: {}\n",
                "    Success: {}\n",
                "  Add Session Key: {}\n",
                "    Success: {}\n",
                "  Validate Session Key: {}\n",
                "    Success: {}\n",
                "  Logout: {}\n",
                "  Get User and Pass Hash: {}\n",
                "  Channel List:\n",
                "    List them: {}\n",
                "    Get: {}\n",
                "    Set: {}\n",
                "    Create: {}\n",
                "    Get Active: {}\n",
                "    Set Active: {}\n",
                "  Status Reports: {}",
            ),
            self.add_user, self.add_user_success,
            self.validate_account, self.validate_acct_success,
            self.add_session_key, self.add_sess_key_success,
            self.validate_session_key, self.validate_sess_key_success,
            self.logout_session_key, self.get_user_passhash,
            self.get_channel_lists, self.get_channel_list,
            self.set_channel_list, self.create_channel_list,
            self.get_active_channel, self.set_active_channel,
            self.get_status_report,
        )
    }
}

#[derive(Clone)]
pub enum NameOrID {
    Name(String),
    ID(i32),
}

#[derive(Clone)]
pub enum Action {
    AddUser { user: String, pass_hash: String, pass_hash_ver: i32, reg_key: String },
    ValidateAccount { val_code: String },
    AddSessKey { user: NameOrID, sess_type: SessType, sess_key: String },
    ValidateSessKey { sess_type: SessType, sess_key: String },
    LogoutSessKey { sess_type: SessType, sess_key: String },
    GetUserPassHash { user: String },
    GetChannelLists { user_id: i32 },
    GetChannelList { user_id: i32, list_name: String },
    SetChannelList { user_id: i32, list_name: String, list_data: String },
    CreateChannelList { user_id: i32, list_name: String },
    GetActiveChannel { user_id: i32 },
    SetActiveChannel { user_id: i32, list_name: String },
    GetStatusReport,
    Shutdown,
}

#[derive(Clone,Debug)]
pub enum Response {
    Empty,
    Bool(bool),
    StringResp(String),
    UserID(i32),
    ValidatedKey(bool, i32),
    UserPassHash(String, i32, bool),
    StatusReport(StatusReport),
    Shutdown,
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
        let mut s_r = StatusReport::default();

        let mut keep_going = true;

        while keep_going {
            let msg = db_rx.recv().expect("DB sender disconnected!");

            let result = match msg.action {
                Action::AddUser { user, pass_hash, pass_hash_ver, reg_key }=>
                    Self::add_user(&dat, &mut s_r, user, pass_hash, pass_hash_ver, reg_key),
                Action::ValidateAccount { val_code } =>
                    Self::validate_account(&dat, &mut s_r, val_code),
                Action::AddSessKey { user, sess_type, sess_key } =>
                    Self::add_session_key(&dat, &mut s_r, user, sess_type, sess_key),
                Action::ValidateSessKey { sess_type, sess_key } =>
                    Self::validate_session_key(&dat, &mut s_r, sess_type, sess_key),
                Action::LogoutSessKey { sess_type, sess_key } =>
                    Self::logout_session_key(&dat, &mut s_r, sess_type, sess_key),
                Action::GetUserPassHash { user } =>
                    Self::get_user_passhash(&dat, &mut s_r, user),
                Action::GetChannelLists { user_id } =>
                    Self::get_channel_lists(&dat, &mut s_r, user_id),
                Action::GetChannelList { user_id, list_name } =>
                    Self::get_channel_list(&dat, &mut s_r, user_id, list_name),
                Action::SetChannelList { user_id, list_name, list_data } =>
                    Self::set_channel_list(&dat, &mut s_r, user_id, list_name, list_data),
                Action::CreateChannelList { user_id, list_name } =>
                    Self::create_channel_list(&dat, &mut s_r, user_id, list_name),
                Action::GetActiveChannel { user_id } =>
                    Self::get_active_channel(&dat, &mut s_r, user_id),
                Action::SetActiveChannel { user_id, list_name } =>
                    Self::set_active_channel(&dat, &mut s_r, user_id, list_name),
                Action::GetStatusReport =>
                    Self::get_status_report(&mut s_r),
                Action::Shutdown =>
                    { keep_going = false; Ok(Response::Shutdown) },
            };

            match msg.resp.send(result) {
                Ok(_) => {},
                Err(_) => {
                    error!("Failed to send database response to requestor...");
                },
            };
        }

        info!("Database shutdown received, shutting down");
    }

    fn add_user(dat: &InThreadData, s_r: &mut StatusReport,
            user: String, pass_hash: String,
            pass_hash_ver: i32, reg_key: String
        )
        -> Result<Response, DBError>
    {
        s_r.add_user += 1;

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

        s_r.add_user_success += 1;

        Ok(Response::UserID(user_ids[0]))
    }

    fn validate_account(dat: &InThreadData, s_r: &mut StatusReport, val_code: String)
        -> Result<Response, DBError>
    {
        s_r.validate_account += 1;

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

        s_r.validate_acct_success += 1;

        Ok(Response::Bool(true))
    }

    fn add_session_key(dat: &InThreadData, s_r: &mut StatusReport, user: NameOrID,
            sess_type: SessType, sess_key: String)
        -> Result<Response, DBError>
    {
        s_r.add_session_key += 1;

        // Generate current time
        let time_now = Utc::now();

        // Find the user_data that matches the username or id if there is one
        let results = match user {
            NameOrID::Name(name) =>
                allow_only_one(
                    ud_dsl.filter(user_data::username.eq(name))
                    .limit(5)
                    .load::<db_models::QueryUserData>(&dat.db_conn)),
            NameOrID::ID(id) =>
                allow_only_one(
                    ud_dsl.filter(user_data::id.eq(id))
                    .limit(5)
                    .load::<db_models::QueryUserData>(&dat.db_conn)),
        }?;


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
                SessType::Display => {
                    let new_sess = db_models::InsertDISessKey {
                        userid: results[0].id,
                        sesskey: &sess_key,
                        creationtime: time_now,
                        lastusedtime: time_now,
                    };
                    diesel::insert_into(display_sess_keys::table)
                        .values(&new_sess)
                        .execute(&dat.db_conn)
                },
            }
        )?;

        s_r.add_sess_key_success += 1;

        Ok(Response::Empty)
    }

    fn validate_session_key(dat: &InThreadData, s_r: &mut StatusReport, sess_type: SessType,
            sess_key: String)
        -> Result<Response, DBError>
    {
        s_r.validate_session_key += 1;

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
            SessType::Display =>
                process_filt_result(
                    disk_dsl.filter(display_sess_keys::sesskey.eq(sess_key))
                        .limit(5)
                        .load::<db_models::QueryDISessKey>(&dat.db_conn)
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
                SessType::Display => 
                    diesel::delete(disk_dsl.find(result.id))
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
                SessType::Display =>
                    diesel::update(disk_dsl.find(result.id))
                        .set((display_sess_keys::lastusedtime.eq(time_now),))
                        .execute(&dat.db_conn),
            }
        )?;

        s_r.validate_sess_key_success += 1;

        Ok(Response::ValidatedKey(true, result.userid))
    }

    fn logout_session_key(dat: &InThreadData, s_r: &mut StatusReport, sess_type: SessType,
            sess_key: String)
        -> Result<Response, DBError>
    {
        s_r.logout_session_key += 1;

        match sess_type {
            SessType::Frontend => 
                diesel::delete(fesk_dsl.filter(
                        front_end_sess_keys::sesskey.eq(sess_key)
                )).execute(&dat.db_conn),
            SessType::Roku => 
                diesel::delete(rosk_dsl.filter(
                        roku_sess_keys::sesskey.eq(sess_key)
                )).execute(&dat.db_conn),
            SessType::Display => 
                diesel::delete(disk_dsl.filter(
                        display_sess_keys::sesskey.eq(sess_key)
                )).execute(&dat.db_conn),
        }?;

        Ok(Response::Empty)
    }

    fn get_user_passhash(dat: &InThreadData, s_r: &mut StatusReport, user: String)
        -> Result<Response, DBError>
    {
        s_r.get_user_passhash += 1;

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

    fn get_channel_lists(dat: &InThreadData, s_r: &mut StatusReport, user_id: i32)
        -> Result<Response, DBError>
    {
        s_r.get_channel_lists += 1;

        let results = cl_dsl
            .filter(channel_list::userid.eq(user_id))
            .load::<db_models::QueryChannelList>(&dat.db_conn)?;
        
        let channel_names: Vec<String> = results.iter().map(|result| {
            result.name.clone()
        }).collect();

        Ok(Response::StringResp(serde_json::to_string(&channel_names)?))
    }

    fn get_channel_list(dat: &InThreadData, s_r: &mut StatusReport,
            user_id: i32, list_name: String
        )
        -> Result<Response, DBError>
    {
        s_r.get_channel_list += 1;

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

    fn set_channel_list(dat: &InThreadData, s_r: &mut StatusReport,
            user_id: i32, list_name: String,
            list_data: String
        )
        -> Result<Response, DBError>
    {
        s_r.set_channel_list += 1;

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

    fn create_channel_list(dat: &InThreadData, s_r: &mut StatusReport,
            user_id: i32, list_name: String
        )
        -> Result<Response, DBError>
    {
        s_r.create_channel_list += 1;

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

    fn get_active_channel(dat: &InThreadData, s_r: &mut StatusReport, user_id: i32)
        -> Result<Response, DBError>
    {
        s_r.get_active_channel += 1;

        joinable!(user_data -> channel_list (active_channel));

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

    fn set_active_channel(dat: &InThreadData, s_r: &mut StatusReport,
            user_id: i32, list_name: String
        )
        -> Result<Response, DBError>
    {
        s_r.set_active_channel += 1;

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

    fn get_status_report(s_r: &mut StatusReport)
        -> Result<Response, DBError>
    {
        s_r.get_status_report += 1;

        Ok(Response::StatusReport(*s_r))
    }
}
