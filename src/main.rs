#[macro_use] extern crate diesel;
#[macro_use] extern crate diesel_migrations;

pub mod schema;

fn get_env_param(param_name: &str, default_val: Option<&str>) -> String {
    let param_file_name = format!("{}_FILE", param_name.clone());

    match std::env::var(param_name) {
        Ok(val) => val,
        Err(_) => match std::env::var(param_file_name.clone()) {
            Ok(filename) => match std::fs::read_to_string(filename.clone()) {
                Ok(val) => val.trim().to_string(),
                Err(err) => panic!("Error reading {} file {}: {:?}", 
                    param_file_name, filename, err),
            }
            Err(_) => match default_val {
                Some(val) => val.to_string(),
                None => panic!("Value must be specified for env var {}", param_name),
            }
        }
    }
}

#[tokio::main]
async fn main() {

    // Get some parameters from the environment
    let db_password = get_env_param("POSTGRES_PASSWORD", None);
    let db_host = get_env_param("POSTGRES_HOST", Some("localhost:5432"));
    let server_address = get_env_param("CB_LISTEN", Some("127.0.0.1:3031"));
    let frontend_loc = get_env_param("FRONTEND_LOC",
        Some("http://localhost:8080"));
    let smtp_server = get_env_param("SMTP_SERVER", Some("localhost"));
    let smtp_port_str = get_env_param("SMTP_PORT", Some("25"));
    let smtp_username = get_env_param("SMTP_USERNAME", Some("webmaster"));
    let smtp_password = get_env_param("SMTP_PASSWORD", Some(""));
    let email_from = get_env_param("EMAIL_FROM_ADDR", Some("webmaster@localhost"));

    let smtp_port: u16 = match smtp_port_str.parse() {
        Ok(val) => val,
        Err(err) => panic!("Error parsing smtp_port: {}", err),
    };

    // Setup DB with arc mutex
    let db_url = format!("postgres://{}:{}@{}/roku_channel_builder",
        "postgres", db_password, db_host);
    let db = db::Db::new(&db_url);

    // Setup email handler?
    let email = email::Email::new(smtp_server, smtp_port, smtp_username,
        smtp_password, email_from, frontend_loc.clone());

    let api = api::build_filters(db, email, frontend_loc);
    let server_sockaddr: std::net::SocketAddr = server_address
        .parse()
        .expect("Unable to parse socket address");
    warp::serve(api).run(server_sockaddr).await;
}

mod db_models {
    use crate::schema::{user_data, front_end_sess_keys, channel_list, roku_sess_keys};
    use chrono::{DateTime, Utc};

    pub struct SessKeyComponents {
        pub id: i32,
        pub userid: i32,
        pub creationtime: DateTime<Utc>,
    }

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
}

mod email {
    use std::time;
    use std::sync::{Arc, mpsc};
    use tokio::task;
    use tokio::sync::Mutex;
    use lettre::transport::smtp::{authentication, client};
    use lettre::message::{MultiPart, Mailbox};
    use lettre::address::AddressError;
    use lettre::{Address, Transport};

    const EMAIL_PERIOD: u64 = 10; // seconds

    /*
    #[derive(Debug, Clone)]
    pub enum EmailError {
        MessageBuildFailure, MessageSendFailure, InvalidAddress
    }

    impl fmt::Display for EmailError {
        fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
            write!(f, "Error {:?}", *self)
        }
    }
    */

    pub fn parse_addr(addr: &str) -> Result<Mailbox, AddressError> {
        let address: Address = addr.parse()?;
        Ok(Mailbox::new(None, address))
    }

    #[derive(Clone)]
    pub struct Email {
        email_tx: Arc<Mutex<mpsc::Sender<Action>>>,
        //handler_thread: task::JoinHandle<()>,
    }

    #[derive(Clone)]
    struct InThreadData {
        smtp_server: String,
        smtp_port: u16,
        smtp_username: String,
        smtp_password: String,
        base_reg_msg: lettre::message::MessageBuilder,
        frontend_loc_str: String,
    }

    #[derive(Clone)]
    pub struct RegisterData {
        pub dest_addr: String,
        pub reg_key: String,
    }

    #[derive(Clone)]
    pub enum Action {
        SendRegAcct(RegisterData),
    }

    impl Email {
        pub fn new(smtp_server: String, smtp_port: u16, smtp_username: String,
                smtp_password: String, email_from: String, frontend_loc_str: String)
            -> Self
        {
            let from_addr = match parse_addr(&email_from.clone()) {
                Ok(addr) => addr,
                Err(err) => panic!("Failed to parse email addr {} - {}",
                    email_from, err),
            };

            let base_reg_msg = lettre::Message::builder()
                .from(from_addr)
                .subject("Running Stream: Verify Your Account");


            let in_thread_data = InThreadData {
                smtp_server,
                smtp_port,
                smtp_username,
                smtp_password,
                base_reg_msg,
                frontend_loc_str,
            };

            // Create channel
            let (email_tx_base, email_rx) = mpsc::channel();

            // Allow email_tx to work nicely with tokio threads and sync
            let email_tx = Arc::new(Mutex::new(email_tx_base));

            // Spawn blocking task
            let _handler_thread = task::spawn_blocking(move || {
                Self::handle_emails(in_thread_data, email_rx)
            });

            Self {
                email_tx,
                //handler_thread,
            }
        }

        pub async fn please(&self, action: Action) -> () {
            // Send the message to the email handler thread
            match self.email_tx.lock().await.send(action) {
                Ok(_) => (),
                Err(err) => {panic!("Email request failed! Dying: {}", err);},
            }
        }

        fn handle_emails(dat: InThreadData, email_rx: mpsc::Receiver<Action>) -> () {
            let sleep_time = time::Duration::from_secs(EMAIL_PERIOD);

            let creds = authentication::Credentials::
                new(dat.smtp_username.clone(), dat.smtp_password.clone());

            let tls_params = match client::TlsParameters::
                new(dat.smtp_server.clone())
            {
                Ok(params) => params,
                Err(err) => panic!("Failed building tls params: {}", err),
            };

            let smtp = match lettre::SmtpTransport::relay(&dat.smtp_server) {
                Ok(builder) => builder
                    .credentials(creds)
                    .tls(client::Tls::Required(tls_params))
                    .port(dat.smtp_port)
                    .build(),
                Err(err) => panic!("Failed building smtp: {}", err),
            };

            loop {
                std::thread::sleep(sleep_time);
                
                while let Some(msg) = match email_rx.try_recv() {
                    Ok(msg) => Some(msg), 
                    Err(mpsc::TryRecvError::Empty) => None,
                    Err(mpsc::TryRecvError::Disconnected) => { 
                        panic!("Email sender disconnected!");
                    },
                } {
                    match msg {
                        Action::SendRegAcct(reg_dat) => 
                            Self::send_reg_acct(smtp.clone(), dat.clone(), reg_dat),
                    }
                }
            }
        }

        fn send_reg_acct(smtp: lettre::SmtpTransport, dat: InThreadData,
                reg_dat: RegisterData)
            -> ()
        {
            let text_msg = format!("Welcome to Running Stream - build your own Roku channel!  Please paste the following link into your browser to complete registration {}/?val_code={} - if you did not attempt to register at Running Stream please just delete this email.", dat.frontend_loc_str, reg_dat.reg_key);
            let html_msg = format!("<p>Welcome to Running Stream - build your own Roku channel!</p>  <p><a href=\"{}/?val_code={}\">Please click here to complete registration</a></p>  <p>If you did not attempt to register at Running Stream please just delete this email.</p>", dat.frontend_loc_str, reg_dat.reg_key);

            let dest_addr_addr = match parse_addr(&reg_dat.dest_addr) {
                Ok(addr) => addr,
                Err(err) => {
                    println!("Failed to parse email addr {} - {}",
                        reg_dat.dest_addr, err);
                    return;
                },
            };

            let msg = match dat.base_reg_msg.clone()
                .to(dest_addr_addr)
                .multipart(MultiPart::alternative_plain_html(
                    text_msg,
                    html_msg,
                ))
            {
                Ok(val) => val,
                Err(err) => {
                    println!("Failed to build message: {:?}", err);
                    return;
                },
            };

            match smtp.send(&msg) {
                Ok(_) => {
                    println!("Registration email sent successfully");
                },
                Err(e) => {
                    println!("Error sending registration email: {:?}", e);
                },
            };
        }
    }
}

mod db {
    use super::{password_hash_version, db_models, api, helpers};
    use super::api::SessType;
    use super::db_models::SessKeyCommon;
    use diesel::pg::PgConnection;
    use diesel::Connection;
    use diesel::RunQueryDsl;
    use diesel::result::Error::NotFound;
    use std::fmt;
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

    #[derive(Clone)]
    pub struct Db {
        db_tx: mpsc::SyncSender<Message>,
    }

    struct InThreadData {
        db_conn: PgConnection,
    }

    #[derive(Clone)]
    pub enum Action {
        AddUser { user: String, pass: String, reg_key: String },
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

    #[derive(Clone)]
    pub enum Response {
        Empty,
        Bool(bool),
        StringResp(String),
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
                Err(err) => println!("Error during migrations: {:?}", err),
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
                            println!("Error getting result from database thread: {}", err);
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
                    Action::AddUser { user, pass, reg_key }=>
                        Self::add_user(&dat, user, pass, reg_key),
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
                        println!("Failed to send database response to requestor...");
                    },
                };
            }
        }

        fn add_user(dat: &InThreadData, user: String, pass: String, reg_key: String)
            -> Result<Response, DBError>
        {
            // Generate the password hash
            let pw_hash = match password_hash_version::hash_pw(&user, &pass) {
                Ok(val) => val,
                Err(err) => {
                    println!("Error hashing password: {}", err);
                    return Err(DBError::PassHashError)},
            };

            // Build the new user data
            let new_user = db_models::InsertUserData {
                username: &user,
                pass_hash: &pw_hash,
                pass_hash_type: password_hash_version::get_pw_ver(),
                validation_status: false,
                validation_code: &reg_key,
            };

            // Make the database insert
            match diesel::insert_into(user_data::table)
                .values(&new_user)
                .execute(&dat.db_conn)
            {
                Ok(1) => Ok(Response::Empty),
                Ok(val) => {
                    println!("Adding user returned other-than 1: {}", val);
                    Err(DBError::InvalidDBResponse)},
                Err(err) => {
                    println!("Error {:?}", err);
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
                    println!("Error getting validation code: {}", err);
                    return Err(DBError::InvalidValidationCode);},
            };

            // Make sure the returned values make a little sense
            match results.len() {
                0 => {
                    return Err(DBError::InvalidValidationCode);
                },
                1 => {},
                _ => {
                    println!("Error with validate account db results: {}", results.len());
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
                    println!("Updating status returned other-than 1: {}", val);
                    Err(DBError::InvalidDBResponse)},
                Err(err) => {
                    println!("Error {:?}", err);
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
                    println!("Error getting username: {}", err);
                    return Err(DBError::InvalidUsername);},
            };

            // Make sure the returned values make a little sense
            match results.len() {
                0 => {
                    return Err(DBError::InvalidUsername);
                },
                1 => {},
                _ => {
                    println!(
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
                    println!("Adding sess key other-than 1: {}", val);
                    Err(DBError::InvalidDBResponse)},
                Err(err) => {
                    println!("Error {:?}", err);
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
                        println!("Error getting session key: {}", err);
                        return Err(DBError::InvalidDBResponse);
                    },
                };
                
                if results.len() != 1 {
                    println!("Error with session key db results: {}", results.len());
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

            let max_age = match sess_type {
                SessType::Frontend => api::SESSION_COOKIE_FE_MAX_AGE,
                SessType::Roku => api::SESSION_COOKIE_RO_MAX_AGE,
            };

            let sess_key_age = time_now.signed_duration_since(
                result.creationtime);
            if sess_key_age > chrono::Duration::seconds(max_age.into()) {
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
                        println!("Updating lastusedtime returned other-than 1: {}", val);
                        Err(DBError::InvalidDBResponse)},
                    Err(err) => {
                        println!("Error updating lastusedtime {:?}", err);
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
                    println!("Updating lastusedtime returned other-than 1: {}", val);
                    Err(DBError::InvalidDBResponse)},
                Err(err) => {
                    println!("Error updating lastusedtime {:?}", err);
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
                    println!("Error updating lastusedtime {:?}", err);
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
                    println!("Error getting user pass hash: {}", err);
                    return Err(DBError::InvalidDBResponse);},
            };
            
            if results.len() != 1 {
                println!("Error with user pass hash db results: {}", results.len());
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
                    println!("Error getting channel lists: {}", err);
                    return Err(DBError::InvalidDBResponse);
                },
            };
            
            let channel_names: Vec<String> = results.iter().map(|result| {
                result.name.clone()
            }).collect();

            match serde_json::to_string(&channel_names) {
                Ok(val) => Ok(Response::StringResp(val)),
                Err(err) => {
                    println!("Error converting channel_names to JSON: {}", err);
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
                    println!("Error getting channel list: {}", err);
                    return Err(DBError::InvalidDBResponse);
                },
            };

            // Make sure we got only one
            if results.len() != 1 {
                println!(
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
                    println!(concat!(
                            "Updating channel list returned other-than 1: ",
                            "userid {} list {} count {}"),
                        user_id, list_name, val
                    );
                    Err(DBError::InvalidDBResponse)},
                Err(err) => {
                    println!(concat!("Error updating channel list ",
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
                    println!("Error creating channel - already exists",);
                    return Err(DBError::EntryAlreadyExists);
                },
                Err(NotFound) => {},
                Err(err) => {
                    println!("Error creating channel: {}", err);
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
                    println!("Adding channel returned other-than 1: {}", val);
                    Err(DBError::InvalidDBResponse)},
                Err(err) => {
                    println!("Error {:?}", err);
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
                    println!("Error getting user pass hash: {}", err);
                    return Err(DBError::InvalidDBResponse);},
            };

            // Make sure the returned values make a little sense
            match results.len() {
                0 => {
                    return Err(DBError::NoEntryReturned);
                },
                1 => {},
                _ => {
                    println!("Error with validate account db results: {}", results.len());
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
                    println!("Error getting channel: {}", err);
                    return Err(DBError::InvalidDBResponse);
                },
            };

            // Make sure we got only one
            if results.len() != 1 {
                println!(
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
                    println!(concat!(
                            "Updating active channel returned other-than 1: ",
                            "userid {} list {} count {}"),
                        user_id, list_name, val
                    );
                    Err(DBError::InvalidDBResponse)},
                Err(err) => {
                    println!(concat!("Error active channel ",
                            "userid {} list {} err {:?}"),
                        user_id, list_name, err
                    );
                    Err(DBError::InvalidDBResponse)},
            }
        }
    }
}

mod api {
    use super::{api_handlers, models, Rejections, db, email};
    use db::{Action, Response};
    use warp::{Filter, reject, Rejection, Reply};
    use warp::http::StatusCode;

    pub static SESSION_COOKIE_NAME: &str = "session";
    pub const SESSION_COOKIE_FE_MAX_AGE: u32 = 60 * 24 * 5; // 5 days
    pub const SESSION_COOKIE_RO_MAX_AGE: u32 = 60 * 24 * 365; // 365 days
    const MAX_AUTH_FORM_LEN: u64 = 4096;

    #[derive(Debug, Clone)]
    pub enum SessType { Frontend, Roku }

    pub fn build_filters(db: db::Db, email: email::Email, cors_origin: String)
        -> impl Filter<Extract = impl warp::Reply, Error = warp::Rejection> + Clone
    {
        let cors = warp::cors()
            .allow_origin(cors_origin.as_str())
            .allow_headers(vec!["sec-ch-ua"])
            .allow_methods(vec!["GET", "POST"])
            .allow_credentials(true);

        api_authenticate_fe(db.clone())
            .or(api_authenticate_ro(db.clone()))
            .or(api_create_account(db.clone(), email.clone()))
            .or(api_validate_account(db.clone()))
            .or(api_logout_session_fe(db.clone()))
            .or(api_get_channel_lists(db.clone()))
            .or(api_get_channel_list(db.clone()))
            .or(api_get_channel_xml_ro(db.clone()))
            .or(api_set_channel_list(db.clone()))
            .or(api_create_channel_list(db.clone()))
            .or(api_set_active_channel(db.clone()))
            .or(api_validate_session_fe(db.clone()))
            .or(api_validate_session_ro(db.clone()))
            //.or(serve_static_index())
            //.or(serve_static_files())
            .with(cors.clone())
            .recover(handle_rejection)
    }

    /*
    fn serve_static_index()
        -> impl Filter<Extract = impl warp::Reply, Error = warp::Rejection> + Clone
    {
        warp::path::end()
            .and(warp::fs::file("static_files/index.html"))
    }

    fn serve_static_files()
        -> impl Filter<Extract = impl warp::Reply, Error = warp::Rejection> + Clone
    {
        warp::fs::dir("static_files")
    }
    */

    fn api_authenticate_fe(db: db::Db)
        -> impl Filter<Extract = impl warp::Reply, Error = warp::Rejection> + Clone
    {
        // TODO do I return neutral responses when email doesn't exist vs
        // bad auth?
        api_v1_path("authenticate_fe")
            .and(warp::post())
            .and(with_db(db))
            .and(get_form::<models::AuthForm>())
            .and_then(api_handlers::authenticate_fe)
    }

    fn api_authenticate_ro(db: db::Db)
        -> impl Filter<Extract = impl warp::Reply, Error = warp::Rejection> + Clone
    {
        // TODO do I return neutral responses when email doesn't exist vs
        // bad auth?
        api_v1_path("authenticate_ro")
            .and(warp::post())
            .and(with_db(db))
            .and(get_form::<models::AuthForm>())
            .and_then(api_handlers::authenticate_ro)
    }

    fn api_create_account(db: db::Db, email: email::Email)
        -> impl Filter<Extract = impl warp::Reply, Error = warp::Rejection> + Clone
    {
        // TODO Do I return neutral responses when the email already exists - failed?
        api_v1_path("create_account")
            .and(warp::post())
            .and(with_db(db))
            .and(with_email(email))
            .and(get_form::<models::CreateAcctForm>())
            .and_then(api_handlers::create_account)
    }

    fn api_validate_account(db: db::Db)
        -> impl Filter<Extract = impl warp::Reply, Error = warp::Rejection> + Clone
    {
        api_v1_path("validate_account")
            .and(warp::get())
            .and(with_db(db))
            .and(warp::query::<models::ValidateAccountRequest>())
            .and_then(api_handlers::validate_account)
    }

    fn api_validate_session_fe(db: db::Db)
        -> impl Filter<Extract = impl warp::Reply, Error = warp::Rejection> + Clone
    {
        api_v1_path("validate_session_fe")
            .and(warp::get())
            .and(with_db(db.clone()))
            .and(validate_session(SessType::Frontend, db))
            .and_then(api_handlers::validate_session_fe)
    }

    fn api_validate_session_ro(db: db::Db)
        -> impl Filter<Extract = impl warp::Reply, Error = warp::Rejection> + Clone
    {
        api_v1_path("validate_session_ro")
            .and(warp::get())
            .and(with_db(db.clone()))
            .and(validate_session(SessType::Roku, db))
            .and_then(api_handlers::validate_session_ro)
    }

    fn api_logout_session_fe(db: db::Db)
        -> impl Filter<Extract = impl warp::Reply, Error = warp::Rejection> + Clone
    {
        api_v1_path("logout_session_fe")
            .and(warp::get())
            .and(with_db(db.clone()))
            .and(validate_session(SessType::Frontend, db))
            .and_then(api_handlers::logout_session_fe)
    }

    fn api_get_channel_lists(db: db::Db)
        -> impl Filter<Extract = impl warp::Reply, Error = warp::Rejection> + Clone
    {
        api_v1_path("get_channel_lists")
            .and(warp::get())
            .and(with_db(db.clone()))
            .and(validate_session(SessType::Frontend, db))
            .and_then(api_handlers::get_channel_lists)
    }

    fn api_get_channel_list(db: db::Db)
        -> impl Filter<Extract = impl warp::Reply, Error = warp::Rejection> + Clone
    {
        api_v1_path("get_channel_list")
            .and(warp::get())
            .and(with_db(db.clone()))
            .and(validate_session(SessType::Frontend, db))
            .and(warp::query::<models::GetChannelListQuery>())
            .and_then(api_handlers::get_channel_list)
    }

    fn api_get_channel_xml_ro(db: db::Db)
        -> impl Filter<Extract = impl warp::Reply, Error = warp::Rejection> + Clone
    {
        api_v1_path("get_channel_xml_ro")
            .and(warp::get())
            .and(with_db(db.clone()))
            .and(validate_session(SessType::Roku, db))
            .and_then(api_handlers::get_channel_xml_ro)
    }

    fn api_set_channel_list(db: db::Db)
        -> impl Filter<Extract = impl warp::Reply, Error = warp::Rejection> + Clone
    {
        api_v1_path("set_channel_list")
            .and(warp::post())
            .and(with_db(db.clone()))
            .and(validate_session(SessType::Frontend, db))
            .and(get_form::<models::SetChannelListForm>())
            .and_then(api_handlers::set_channel_list)
    }

    fn api_create_channel_list(db: db::Db)
        -> impl Filter<Extract = impl warp::Reply, Error = warp::Rejection> + Clone
    {
        api_v1_path("create_channel_list")
            .and(warp::post())
            .and(with_db(db.clone()))
            .and(validate_session(SessType::Frontend, db))
            .and(get_form::<models::CreateChannelListForm>())
            .and_then(api_handlers::create_channel_list)
    }

    fn api_set_active_channel(db: db::Db)
        -> impl Filter<Extract = impl warp::Reply, Error = warp::Rejection> + Clone
    {
        api_v1_path("set_active_channel")
            .and(warp::post())
            .and(with_db(db.clone()))
            .and(validate_session(SessType::Frontend, db))
            .and(get_form::<models::SetActiveChannelForm>())
            .and_then(api_handlers::set_active_channel)
    }

    fn get_form<T>()
        -> impl Filter<Extract = (T,), Error = warp::Rejection> + Clone
        where
            T: Send,
            T: for<'de> serde::Deserialize<'de>
    {
        warp::body::content_length_limit(MAX_AUTH_FORM_LEN)
            .and(warp::body::form())
    }

    fn validate_session(sess_type: SessType, db: db::Db)
        -> impl Filter<Extract = ((String, i32),), Error = warp::Rejection> + Clone
    {
        warp::filters::cookie::cookie::<String>(SESSION_COOKIE_NAME)
            .and(with_db(db.clone()))
            .and_then(move |session_id: String, db: db::Db| {
                let sess_type = sess_type.clone();
                async move {
                    match db.please(Action::ValidateSessKey {
                        sess_type: sess_type,
                        sess_key: session_id.clone(),
                    }).await {
                        Ok(Response::ValidatedKey(true, user_id)) =>
                            Ok((session_id, user_id)),
                        Ok(Response::ValidatedKey(false, _)) =>
                            Err(reject::custom(Rejections::InvalidSession)),
                        Ok(_) => {
                            println!(
                                "Invalid response when validating fe session"
                            );
                            Err(reject::custom(Rejections::InvalidSession))
                        },
                        Err(err) => {
                            println!("Error validating fe session: {}", err);
                            Err(reject::custom(Rejections::InvalidSession))
                        },
                    }
                }
            })
    }

    fn api_v1_path(api_tail: &str)
        -> impl Filter<Extract = (), Error = warp::Rejection> + Clone + '_
    {
        warp::path("api")
            .and(warp::path("v1"))
            .and(warp::path(api_tail))
            .and(warp::path::end())
    }

    fn with_db(db: db::Db)
        -> impl Filter<Extract = (db::Db,),
                Error = std::convert::Infallible>
            + Clone
    {
        warp::any().map(move || db.clone())
    }

    fn with_email(email: email::Email)
        -> impl Filter<Extract = (email::Email,),
                Error = std::convert::Infallible>
            + Clone
    {
        warp::any().map(move || email.clone())
    }

    async fn handle_rejection(err: Rejection)
        -> Result<impl Reply, warp::Rejection>
    {
        let code;
        let message: String;

        if let Some(Rejections::InvalidSession) = err.find() {
            code = StatusCode::FORBIDDEN;
            message = "Forbidden".to_string();
            Ok(warp::reply::with_status(message, code))
        } else if let Some(Rejections::InvalidValidationCode) = err.find() {
            code = StatusCode::FORBIDDEN;
            message = "Forbidden".to_string();
            Ok(warp::reply::with_status(message, code))
        } else {
            Err(err)
        }
    }
}

#[derive(Debug)]
enum Rejections { InvalidSession, InvalidUser, InvalidPassword,
    HashValidationError, ErrorCreatingUser, ErrorValidatingAccount,
    ErrorAddingSessionKey, ErrorGettingChannelLists, ErrorGettingChannelList,
    ErrorParsingChannelList, ErrorSettingChannelList, ErrorCreatingChannelList,
    ErrorSettingActiveChannel, InvalidValidationCode }

impl warp::reject::Reject for Rejections {}

mod models {
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
    pub struct CreateChannelListForm {
        pub listname: String,
    }
    
    #[derive(Debug,Deserialize,Serialize,Clone)]
    pub struct SetActiveChannelForm {
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
}

mod api_handlers {
    use super::{models, db, password_hash_version, Rejections, api, helpers, email};
    use super::api::SessType;
    use db::{Action, Response};
    use rand::Rng;
    use warp::http::StatusCode;
    use warp::reject;

    pub async fn authenticate_ro(db: db::Db, form_dat: models::AuthForm)
        -> Result<impl warp::Reply, warp::Rejection>
    {
        println!("Trying to auth roku");
        authenticate_gen(SessType::Roku, db, form_dat).await
    }

    pub async fn authenticate_fe(db: db::Db, form_dat: models::AuthForm)
        -> Result<impl warp::Reply, warp::Rejection>
    {
        authenticate_gen(SessType::Frontend, db, form_dat).await
    }

    async fn authenticate_gen(sess_type: SessType, db: db::Db, form_dat: models::AuthForm)
        -> Result<impl warp::Reply, warp::Rejection>
    {
        let (pass_hash, hash_ver, valid_status) = 
            match db.please(Action::GetUserPassHash {
                user: form_dat.username.clone(),
            }).await {
                Ok(Response::UserPassHash(pass_hash, hash_ver, valid_status)) => 
                    (pass_hash, hash_ver, valid_status),
                Ok(_) => {
                    println!("Invalid type returned by GetUserPassHash");
                    return Err(reject::custom(Rejections::InvalidUser));
                },
                Err(err) => {
                    println!("Error getting user: {}", err);
                    return Err(reject::custom(Rejections::InvalidUser));
                },
            };

        if !valid_status {
            println!("Non-validated user attempted login");
            return Err(reject::custom(Rejections::InvalidUser));
        }

        let sess_key = gen_large_rand_str();

        match db.please(Action::AddSessKey {
            user: form_dat.username.clone(),
            sess_type: sess_type.clone(),
            sess_key: sess_key.clone(),
        }).await {
            Ok(_) => {},
            Err(err) => {
                println!("Error adding session key: {}", err);
                return Err(reject::custom(Rejections::ErrorAddingSessionKey))},
        };

        let max_age = match sess_type {
            SessType::Frontend => api::SESSION_COOKIE_FE_MAX_AGE,
            SessType::Roku => api::SESSION_COOKIE_RO_MAX_AGE,
        };

        println!("Authenticated {:?}: {:?} key {}", sess_type, form_dat, sess_key);

        // Add the session key as content if this is a roku auth
        // TODO: make it so we don't have to do that anymore...
        let base_reply = match sess_type {
            SessType::Roku => warp::reply::html(sess_key.clone()),
            _ => warp::reply::html("".to_string()),
        };

        match password_hash_version::validate_pw_ver(&form_dat.username,
            &form_dat.password, &pass_hash, hash_ver)
        {
            Ok(true) =>
                Ok(warp::reply::with_header(
                    base_reply,
                    "Set-Cookie", 
                    format!("{}={}; Max-Age={}", 
                        api::SESSION_COOKIE_NAME, sess_key,
                        max_age)
                )),
            Ok(false) => {println!("Wrong password");
                Err(reject::custom(Rejections::InvalidPassword))},
            Err(err) => {println!("Error validating hash: {}", err);
                Err(reject::custom(Rejections::HashValidationError))},
        }
    }

    pub async fn create_account(db: db::Db, email_inst: email::Email,
            form_dat: models::CreateAcctForm)
        -> Result<impl warp::Reply, warp::Rejection>
    {
        // Fail early if the username is invalid
        match email::parse_addr(&form_dat.username) {
            Ok(_) => {},
            Err(err) => {
                println!("Requested username is invalid email: {}", err);
                return Err(reject::custom(Rejections::ErrorCreatingUser));
            }
        };

        // TODO: handle properly when the rand number is already in the DB
        let reg_key = gen_large_rand_str();
        println!("Adding user with reg key ?val_code={}", reg_key);

        match db.please(Action::AddUser {
            user: form_dat.username.clone(),
            pass: form_dat.password,
            reg_key: reg_key.clone(),
        }).await {
            Ok(_) => {},
            Err(err) => {
                println!("Error adding user: {}", err); 
                return Err(reject::custom(Rejections::ErrorCreatingUser));
            },
        };

        email_inst.please(email::Action::SendRegAcct(
            email::RegisterData {
                dest_addr: form_dat.username,
                reg_key: reg_key,
            }
        )).await;

        Ok(StatusCode::OK)
    }

    pub async fn validate_account(db: db::Db,
        opts: models::ValidateAccountRequest)
        -> Result<impl warp::Reply, warp::Rejection>
    {
        match db.please(Action::ValidateAccount { val_code: opts.val_code }).await {
            Ok(Response::Bool(true)) => Ok(StatusCode::OK),
            Ok(_) => {
                println!("Invalid validation code received."); 
                Err(reject::custom(Rejections::InvalidValidationCode))
            },
            Err(db::DBError::InvalidValidationCode) => {
                println!("Invalid validation code received."); 
                Err(reject::custom(Rejections::InvalidValidationCode))
            },
            Err(err) => {println!("Error validating account: {}", err); 
                Err(reject::custom(Rejections::ErrorValidatingAccount))
            },
        }
        
    }

    pub async fn validate_session_fe(db: db::Db, sess_info: (String, i32))
        -> Result<impl warp::Reply, warp::Rejection>
    {
        validate_session(SessType::Frontend, db, sess_info).await
    }

    pub async fn validate_session_ro(db: db::Db, sess_info: (String, i32))
        -> Result<impl warp::Reply, warp::Rejection>
    {
        validate_session(SessType::Roku, db, sess_info).await
    }

    async fn validate_session(_sess_type: SessType, _db: db::Db,
        _sess_info: (String, i32))
        -> Result<impl warp::Reply, warp::Rejection>
    {
        // If we can get to here, we're ok
        // TODO - what's the right response?
        //Ok(warp::reply::html("Valid")) // For some reason, Rust won't compile if I use this
        Ok(StatusCode::OK)
    }

    pub async fn logout_session_fe(db: db::Db, sess_info: (String, i32))
        -> Result<impl warp::Reply, warp::Rejection>
    {
        let (sess_key, _user_id) = sess_info;

        // TODO - what's the right response?
        match db.please(Action::LogoutSessKey {
            sess_type: SessType::Frontend,
            sess_key: sess_key,
        }).await {
            Ok(_) => Ok(StatusCode::OK),
            Err(err) => {println!("Error logging out account: {}", err); 
                Err(reject::custom(Rejections::ErrorValidatingAccount))},
        }
    }

    pub async fn get_channel_lists(db: db::Db, sess_info: (String, i32))
        -> Result<impl warp::Reply, warp::Rejection>
    {
        let (_sess_key, user_id) = sess_info;

        match db.please(Action::GetChannelLists {
            user_id: user_id,
        }).await {
            Ok(Response::StringResp(val)) => Ok(warp::reply::html(val)),
            Ok(_) => {
                println!("Invalid return type received from GetChannelLists");
                Err(reject::custom(Rejections::ErrorGettingChannelLists))},
            Err(err) => {println!("Error getting channel lists: {}", err); 
                Err(reject::custom(Rejections::ErrorGettingChannelLists))},
        }
    }

    pub async fn get_channel_list(db: db::Db, sess_info: (String, i32), 
        opts: models::GetChannelListQuery)
        -> Result<impl warp::Reply, warp::Rejection>
    {
        let (_sess_key, user_id) = sess_info;

        match db.please(Action::GetChannelList {
            user_id: user_id,
            list_name: opts.list_name,
        }).await {
            Ok(Response::StringResp(val)) => Ok(warp::reply::html(val)),
            Ok(_) => {
                println!("Invalid return type received from GetChannelList");
                Err(reject::custom(Rejections::ErrorGettingChannelList))},
            Err(err) => {println!("Error getting channel list: {}", err); 
                Err(reject::custom(Rejections::ErrorGettingChannelList))},
        }
    }

    pub async fn get_channel_xml_ro(db: db::Db, sess_info: (String, i32)) 
        -> Result<impl warp::Reply, warp::Rejection>
    {
        let (_sess_key, user_id) = sess_info;

        let channel_list = match db.please(Action::GetActiveChannel {
            user_id: user_id,
        }).await {
            Ok(Response::StringResp(val)) => val,
            Ok(_) => {
                println!("Invalid return type received from GetActiveChannel");
                return Err(reject::custom(Rejections::ErrorGettingChannelList));
            },
            Err(err) => {
                println!("Error getting active channel: {}", err); 
                return Err(reject::custom(Rejections::ErrorGettingChannelList));
            },
        };

        let json: serde_json::Value = match serde_json::from_str(&channel_list) {
            Ok(val) => val,
            Err(err) => {println!("Error parsing channel list: {}", err); 
                return Err(reject::custom(Rejections::ErrorParsingChannelList))},
        };

        let xml_str1 = helpers::build_xml(json.clone());
        Ok(warp::reply::html(xml_str1))
    }

    pub async fn set_channel_list(db: db::Db, sess_info: (String, i32), 
        form_dat: models::SetChannelListForm)
        -> Result<impl warp::Reply, warp::Rejection>
    {
        let (_sess_key, user_id) = sess_info;

        // TODO validate that input is json
        // TODO convert to XML now?

        match db.please(Action::SetChannelList {
            user_id: user_id,
            list_name: form_dat.listname,
            list_data: form_dat.listdata,
        }).await {
            Ok(_) => Ok(StatusCode::OK),
            Err(err) => {println!("Error setting channel list: {}", err); 
                Err(reject::custom(Rejections::ErrorSettingChannelList))},
        }
    }

    pub async fn create_channel_list(db: db::Db, sess_info: (String, i32), 
        form_dat: models::CreateChannelListForm)
        -> Result<impl warp::Reply, warp::Rejection>
    {
        let (_sess_key, user_id) = sess_info;

        match db.please(Action::CreateChannelList {
            user_id: user_id,
            list_name: form_dat.listname,
        }).await {
            Ok(_) => Ok(StatusCode::OK),
            Err(err) => {println!("Error creating channel list: {}", err); 
                Err(reject::custom(Rejections::ErrorCreatingChannelList))},
        }
    }

    pub async fn set_active_channel(db: db::Db, sess_info: (String, i32), 
        form_dat: models::SetActiveChannelForm)
        -> Result<impl warp::Reply, warp::Rejection>
    {
        let (_sess_key, user_id) = sess_info;

        match db.please(Action::SetActiveChannel {
            user_id: user_id,
            list_name: form_dat.listname,
        }).await {
            Ok(_) => Ok(StatusCode::OK),
            Err(err) => {println!("Error setting active channel list: {}", err); 
                Err(reject::custom(Rejections::ErrorSettingActiveChannel))},
        }
    }

    fn gen_large_rand_str() -> String {
        // Generate a 64 character code in ascii hex
        let reg_key_p1 = rand::thread_rng().gen::<u128>();
        let reg_key_p2 = rand::thread_rng().gen::<u128>();
        format!("{:032X}{:032X}", reg_key_p1, reg_key_p2)
    }
}

mod password_hash_version {
    use std::fmt;

    const CUR_HASH_VER: i32 = 1;

    #[derive(Debug, Clone)]
    pub enum CustError { InvalidHashVer, HashingError, HashParseError }

    impl fmt::Display for CustError {
        fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
            match *self {
                CustError::InvalidHashVer => write!(f, "Invalid hash version supplied"),
                CustError::HashingError => write!(f, "Error when hashing..."),
                CustError::HashParseError => write!(f, "Error parsing hash..."),
            }
        }
    }

    // Hash a password as the current best version
    pub fn hash_pw(username: &str, password: &str) -> Result<String, CustError> {
        hash_pw_ver(username, password, CUR_HASH_VER)
    }

    // Get the current best hash version
    pub fn get_pw_ver() -> i32 { CUR_HASH_VER }

    // Hash a password as a specific version
    pub fn hash_pw_ver(_username: &str, password: &str, version: i32)
        -> Result<String, CustError>
    {
        match version {
            1 => hash_pw_ver_1(password),
            _ => Err(CustError::InvalidHashVer),
        }
    }

    // Validate a password against a specific version
    pub fn validate_pw_ver(_username: &str, password: &str,
        password_hash: &str, version: i32) -> Result<bool, CustError>
    {
        match version {
            1 => validate_pw_ver_1(password, password_hash),
            _ => Err(CustError::InvalidHashVer),
        }
    }

    fn hash_pw_ver_1(password: &str) -> Result<String, CustError> {
        use argon2::password_hash::{PasswordHasher, SaltString};
        use argon2::Argon2;

        // Get the salt...
        let salt = SaltString::generate(&mut rand::thread_rng());

        // Argon2 with default params (Argon2id v19)
        let argon2 = Argon2::default();

        // Hash password to PHC string ($argon2id$v=19$...)
        let hash_result = argon2.hash_password_simple(password.as_bytes(),
            salt.as_ref());
        match hash_result {
            Ok(hash) => Ok(hash.to_string()),
            Err(err) => {
                println!("Error in hashing: {}", err);
                Err(CustError::HashingError)
                },
        }
    }

    fn validate_pw_ver_1(password: &str, password_hash: &str)
        -> Result<bool, CustError>
    {
        use argon2::password_hash::{PasswordHash, PasswordVerifier};
        use argon2::Argon2;

        // Parse the hash to grab the salt and properties
        let parsed_hash = match PasswordHash::new(password_hash) {
            Ok(val) => val,
            Err(err) => {
                println!("Error in parsing hash: {}", err);
                return Err(CustError::HashParseError);
                },
        };

        // Argon2 with default params (Argon2id v19)
        let argon2 = Argon2::default();

        // Verify password
        Ok(argon2.verify_password(password.as_bytes(), &parsed_hash).is_ok())
    }
}

mod helpers {
    use std::result::Result;
    use std::fmt::Debug;
    use std::time::Duration;
    use std::thread::sleep;

    #[derive(Debug)]
    pub enum RetryErr {
        RetriesExhausted,
    }

    pub fn retry_on_err<F: Fn() -> Result<T, U>, T, U: Debug>
        ( count: u32, sleep_len: Duration, func: F) -> Result<T, RetryErr>
    {
        if count <= 0 {
            println!("Retries exhausted");
            return Err(RetryErr::RetriesExhausted);
        }
        match func() {
            Ok(val) => Ok(val),
            Err(err) => {
                println!("Error with {} retries remaining: {:?}", count - 1, err);
                sleep(sleep_len);
                retry_on_err(count - 1, sleep_len, func)
            },
        }
    }


    pub fn build_xml(json: serde_json::Value) -> String {
        use serde_json::Value;
        match json {
            Value::Null => "".to_string(),
            Value::Bool(val) => format!("{}", val),
            Value::Number(val) => format!("{}", val),
            Value::String(val) => format!("{}", val),
            Value::Array(arr) => arr.iter()
                .map(|val| {
                    format!("<array_elem>{}</array_elem>", build_xml(val.clone())) 
                }).collect::<String>(),
            Value::Object(map) => format!("<object>{}</object>", 
                map.iter().map(|(key, val)| {
                    format!("<{}>{}</{}>", key, build_xml(val.clone()), key)
                }).collect::<String>()),
        }
    }
}
