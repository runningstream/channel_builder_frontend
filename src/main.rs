#[macro_use] extern crate diesel;
#[macro_use] extern crate diesel_migrations;

pub mod schema;

#[tokio::main]
async fn main() {

    // Setup DB with arc mutex?
    let db_url = "postgres://postgres:mysecretpassword@localhost/roku_channel_builder";
    let db = db::Db::new(db_url);

    // Setup email handler?
    //let email = 

    let api = api::build_filters(db.clone());
    let server_address = "127.0.0.1:3031";
    let server_sockaddr: std::net::SocketAddr = server_address
        .parse()
        .expect("Unable to parse socket address");
    warp::serve(api).run(server_sockaddr).await;
}

mod db_models {
    use crate::schema::{user_data, front_end_sess_keys};
    use chrono::{DateTime, Utc};

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

    #[derive(Insertable)]
    #[table_name="front_end_sess_keys"]
    pub struct InsertFESessKey<'a> {
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
}

mod db {
    use super::{password_hash_version, db_models, api};
    use diesel::pg::PgConnection;
    use diesel::Connection;
    use diesel::RunQueryDsl;
    use std::fmt;
    use tokio::sync::Mutex;
    use std::sync::Arc;
    use chrono::Utc;
    use crate::diesel::{QueryDsl, ExpressionMethods};
    use crate::schema::{user_data, channel_list};

    embed_migrations!();

    #[derive(Debug)]
    pub enum DBError {
        PassHashError,
        InvalidDBResponse,
        InvalidValidationCode,
        InvalidUsername,
        JSONConversionError,
    }

    impl fmt::Display for DBError {
        fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
            match *self {
                DBError::PassHashError => write!(f, "Error hashing password"),
                DBError::InvalidDBResponse => write!(f, "Invalid DB response"),
                DBError::InvalidValidationCode => write!(f, "Invalid validation code"),
                DBError::InvalidUsername => write!(f, "Invalid username"),
                DBError::JSONConversionError => write!(f, "JSON conversion error"),
            }
        }
    }

    #[derive(Clone)]
    pub struct Db {
        db_arc: Arc<Mutex<PgConnection>>,
    }

    impl Db {
        pub fn new(db_url: &str) -> Self {
            let db_conn = PgConnection::establish(db_url)
                .expect("Unable to connect to database");

            match embedded_migrations::run_with_output(&db_conn,
                &mut std::io::stdout()) 
            {
                Ok(_) => {},
                Err(err) => println!("Error during migrations: {:?}", err),
            };

            let db_arc = Arc::new(Mutex::new(db_conn));

            Self {
                db_arc,
            }
        }

        pub async fn add_user(&self, user: &str, pass: &str, reg_key: &str)
                -> Result<(), DBError>
        {
            // TODO email vaildation

            // Generate the password hash
            let pw_hash = match password_hash_version::hash_pw(user, pass) {
                Ok(val) => val,
                Err(err) => {
                    println!("Error hashing password: {}", err);
                    return Err(DBError::PassHashError)},
            };

            // Build the new user data
            let new_user = db_models::InsertUserData {
                username: user,
                pass_hash: &pw_hash,
                pass_hash_type: password_hash_version::get_pw_ver(),
                validation_status: false,
                validation_code: reg_key,
            };

            // Lock the database connection
            let db_conn = self.db_arc.lock().await;

            // Make the database insert
            match diesel::insert_into(user_data::table)
                .values(&new_user)
                .execute(& *db_conn)
            {
                Ok(1) => Ok(()),
                Ok(val) => {
                    println!("Adding user returned other-than 1: {}", val);
                    Err(DBError::InvalidDBResponse)},
                Err(err) => {
                    println!("Error {:?}", err);
                    Err(DBError::InvalidDBResponse)},
            }
        }

        pub async fn validate_account(&self, val_code: &str)
            -> Result<bool, DBError>
        {
            use crate::schema::user_data::{validation_code, validation_status};

            // Lock the database
            let db_conn = self.db_arc.lock().await;

            // Find the user_data that matches the val_code if there is one
            let results = match user_data::dsl::user_data.filter(validation_code.eq(val_code))
                .limit(5)
                .load::<db_models::QueryUserData>(& *db_conn)
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
            match diesel::update(user_data::dsl::user_data.find(id))
                .set((
                    validation_status.eq(true),
                    validation_code.eq::<Option<String>>(None),
                ))
                .execute(& *db_conn)
            {
                Ok(1) => Ok(true),
                Ok(val) => {
                    println!("Updating status returned other-than 1: {}", val);
                    Err(DBError::InvalidDBResponse)},
                Err(err) => {
                    println!("Error {:?}", err);
                    Err(DBError::InvalidDBResponse)},
            }
        }

        pub async fn add_fe_session_key(&self, user: &str, sess_key: &str)
                -> Result<(), DBError>
        {
            use crate::schema::user_data::username;
            use crate::schema::front_end_sess_keys;

            // Generate current time
            let time_now = Utc::now();

            // Lock the database
            let db_conn = self.db_arc.lock().await;

            // Find the user_data that matches the username if there is one
            let results = match user_data::dsl::user_data.filter(username.eq(user))
                .limit(5)
                .load::<db_models::QueryUserData>(& *db_conn)
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
                    println!("Error with add session key account db results: {}", results.len());
                    return Err(DBError::InvalidDBResponse);
                },
            };

            // Build the sess key entry 
            let new_sess = db_models::InsertFESessKey {
                userid: results[0].id,
                sesskey: sess_key,
                creationtime: time_now,
                lastusedtime: time_now,
            };

            // Make the database insert
            match diesel::insert_into(front_end_sess_keys::table)
                .values(&new_sess)
                .execute(& *db_conn)
            {
                Ok(1) => Ok(()),
                Ok(val) => {
                    println!("Adding sess key other-than 1: {}", val);
                    Err(DBError::InvalidDBResponse)},
                Err(err) => {
                    println!("Error {:?}", err);
                    Err(DBError::InvalidDBResponse)},
            }
        }

        pub async fn validate_fe_session_key(&self, sess_key: &str)
                -> Result<(bool, i32), DBError>
        {
            use crate::schema::front_end_sess_keys::dsl::front_end_sess_keys;
            use crate::schema::front_end_sess_keys::{sesskey, lastusedtime};
            use chrono::Duration;

            let db_conn = self.db_arc.lock().await;
            let results = match front_end_sess_keys.filter(sesskey.eq(sess_key))
                .limit(5)
                .load::<db_models::QueryFESessKey>(& *db_conn)
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

            let result = &results[0];

            // Validate that session key hasn't expired
            let time_now = Utc::now();

            let sess_key_age = time_now.signed_duration_since(
                result.creationtime);
            if sess_key_age > Duration::seconds(api::SESSION_COOKIE_MAX_AGE.into()) {
                // Delete sess key
                return match diesel::delete(front_end_sess_keys.find(result.id))
                    .execute(& *db_conn)
                {
                    // Return failed session key
                    Ok(1) => Ok((false, 0)),
                    Ok(val) => {
                        println!("Updating lastusedtime returned other-than 1: {}", val);
                        Err(DBError::InvalidDBResponse)},
                    Err(err) => {
                        println!("Error updating lastusedtime {:?}", err);
                        Err(DBError::InvalidDBResponse)},
                };
            }

            // Update last used time
            match diesel::update(front_end_sess_keys.find(result.id))
                .set((
                    lastusedtime.eq(time_now),
                ))
                .execute(& *db_conn)
            {
                Ok(1) => Ok((true, result.userid)),
                Ok(val) => {
                    println!("Updating lastusedtime returned other-than 1: {}", val);
                    Err(DBError::InvalidDBResponse)},
                Err(err) => {
                    println!("Error updating lastusedtime {:?}", err);
                    Err(DBError::InvalidDBResponse)},
            }
        }

        pub async fn logout_fe_session_key(&self, sess_key: &str)
                -> Result<(), DBError>
        {
            use crate::schema::front_end_sess_keys::dsl::front_end_sess_keys;
            use crate::schema::front_end_sess_keys::{sesskey};

            let db_conn = self.db_arc.lock().await;

            match diesel::delete(front_end_sess_keys.filter(sesskey.eq(sess_key)))
                .execute(& *db_conn)
            {
                // Return failed session key
                Ok(_) => Ok(()),
                Err(err) => {
                    println!("Error updating lastusedtime {:?}", err);
                    Err(DBError::InvalidDBResponse)},
            }
        }

        pub async fn get_user_passhash(&self, user: &str)
            -> Result<(String, i32, bool), DBError>
        {
            use crate::schema::user_data::username;

            let db_conn = self.db_arc.lock().await;
            let results = match user_data::dsl::user_data.filter(username.eq(user))
                .limit(5)
                .load::<db_models::QueryUserData>(& *db_conn)
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

            Ok((results[0].pass_hash.clone(), results[0].pass_hash_type,
                results[0].validation_status))
        }

        pub async fn get_channel_lists(&self, user_id: i32)
            -> Result<String, DBError>
        {
            let db_conn = self.db_arc.lock().await;
            let results = match channel_list::dsl::channel_list.filter(channel_list::userid.eq(user_id))
                .load::<db_models::QueryChannelList>(& *db_conn)
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
                Ok(val) => Ok(val),
                Err(err) => {
                    println!("Error converting channel_names to JSON: {}", err);
                    return Err(DBError::JSONConversionError);
                },
            }
        }

        pub async fn get_channel_list(&self, user_id: i32, list_name: &str)
            -> Result<String, DBError>
        {
            Ok("asdf".to_string())
        }

        pub async fn set_channel_list(&self, user_id: i32, list_name: &str,
            list_data: &str)
            -> Result<(), DBError>
        {
            Ok(())
        }

        pub async fn create_channel_list(&self, user_id: i32, list_name: &str)
            -> Result<(), DBError>
        {
            Ok(())
        }

        pub async fn set_active_channel(&self, user_id: i32, list_name: &str)
            -> Result<(), DBError>
        {
            Ok(())
        }
    }
}

mod api {
    use super::{api_handlers, models, Rejections, db};
    use warp::{Filter, reject, Rejection, Reply};
    use warp::http::StatusCode;

    pub static SESSION_COOKIE_NAME: &str = "session";
    pub const SESSION_COOKIE_MAX_AGE: u32 = 60 * 24 * 5; // 5 days
    const MAX_AUTH_FORM_LEN: u64 = 4096;

    pub fn build_filters(db: db::Db)
        -> impl Filter<Extract = impl warp::Reply, Error = warp::Rejection> + Clone
    {
        api_authenticate_fe(db.clone())
            .or(api_create_account(db.clone()))
            .or(api_validate_account(db.clone()))
            .or(api_validate_session_fe(db.clone()))
            .or(api_logout_session_fe(db.clone()))
            .or(api_get_channel_lists(db.clone()))
            .or(api_get_channel_list(db.clone()))
            .or(api_set_channel_list(db.clone()))
            .or(api_create_channel_list(db.clone()))
            .or(api_set_active_channel(db.clone()))
            .or(serve_static_index())
            .or(serve_static_files())
            .recover(handle_rejection)
    }

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

    fn api_create_account(db: db::Db)
        -> impl Filter<Extract = impl warp::Reply, Error = warp::Rejection> + Clone
    {
        // TODO Do I return neutral responses when the email already exists - failed?
        api_v1_path("create_account")
            .and(warp::post())
            .and(with_db(db))
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
            .and(validate_fe_session(db))
            .and_then(api_handlers::validate_session_fe)
    }

    fn api_logout_session_fe(db: db::Db)
        -> impl Filter<Extract = impl warp::Reply, Error = warp::Rejection> + Clone
    {
        api_v1_path("logout_session_fe")
            .and(warp::get())
            .and(with_db(db.clone()))
            .and(validate_fe_session(db))
            .and_then(api_handlers::logout_session_fe)
    }

    fn api_get_channel_lists(db: db::Db)
        -> impl Filter<Extract = impl warp::Reply, Error = warp::Rejection> + Clone
    {
        api_v1_path("get_channel_lists")
            .and(warp::get())
            .and(with_db(db.clone()))
            .and(validate_fe_session(db))
            .and_then(api_handlers::get_channel_lists)
    }

    fn api_get_channel_list(db: db::Db)
        -> impl Filter<Extract = impl warp::Reply, Error = warp::Rejection> + Clone
    {
        api_v1_path("get_channel_list")
            .and(warp::get())
            .and(with_db(db.clone()))
            .and(validate_fe_session(db))
            .and(warp::query::<models::GetChannelListQuery>())
            .and_then(api_handlers::get_channel_list)
    }

    fn api_set_channel_list(db: db::Db)
        -> impl Filter<Extract = impl warp::Reply, Error = warp::Rejection> + Clone
    {
        api_v1_path("set_channel_list")
            .and(warp::post())
            .and(with_db(db.clone()))
            .and(validate_fe_session(db))
            .and(get_form::<models::SetChannelListForm>())
            .and_then(api_handlers::set_channel_list)
    }

    fn api_create_channel_list(db: db::Db)
        -> impl Filter<Extract = impl warp::Reply, Error = warp::Rejection> + Clone
    {
        api_v1_path("create_channel_list")
            .and(warp::post())
            .and(with_db(db.clone()))
            .and(validate_fe_session(db))
            .and(get_form::<models::CreateChannelListForm>())
            .and_then(api_handlers::create_channel_list)
    }

    fn api_set_active_channel(db: db::Db)
        -> impl Filter<Extract = impl warp::Reply, Error = warp::Rejection> + Clone
    {
        api_v1_path("set_active_channel")
            .and(warp::post())
            .and(with_db(db.clone()))
            .and(validate_fe_session(db))
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

    fn validate_fe_session(db: db::Db)
        -> impl Filter<Extract = ((String, i32),), Error = warp::Rejection> + Clone
    {
        warp::filters::cookie::cookie::<String>(SESSION_COOKIE_NAME)
            .and(with_db(db.clone()))
            .and_then(|session_id: String, db: db::Db| async move {
                match db.validate_fe_session_key(&session_id).await {
                    Ok((true, user_id)) => Ok((session_id, user_id)),
                    Ok((false, _)) => Err(reject::custom(Rejections::InvalidSession)),
                    Err(err) => {
                        println!("Error validating fe session: {}", err);
                        Err(reject::custom(Rejections::InvalidSession))
                    },
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
        -> impl Filter<Extract = (db::Db,), Error = std::convert::Infallible> + Clone
    {
        warp::any().map(move || db.clone())
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
        } else {
            Err(err)
        }

    }
}

#[derive(Debug)]
enum Rejections { InvalidSession, InvalidUser, InvalidPassword,
    HashValidationError, ErrorCreatingUser, ErrorValidatingAccount,
    ErrorAddingSessionKey, ErrorGettingChannelLists, ErrorGettingChannelList,
    ErrorSettingChannelList, ErrorCreatingChannelList,
    ErrorSettingActiveChannel }

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
    use super::{models, db, password_hash_version, Rejections, api};
    use rand::Rng;
    use warp::http::StatusCode;
    use warp::reject;

    pub async fn authenticate_fe(db: db::Db, form_dat: models::AuthForm)
        -> Result<impl warp::Reply, warp::Rejection>
    {
        let (pass_hash, hash_ver, valid_status) = 
            match db.get_user_passhash(&form_dat.username).await {
                Ok(vals) => vals,
                Err(err) => {println!("Error getting user: {}", err);
                    return Err(reject::custom(Rejections::InvalidUser));},
            };

        if !valid_status {
            println!("Non-validated user attempted login");
            return Err(reject::custom(Rejections::InvalidUser));
        }

        let sess_key = gen_large_rand_str();

        match db.add_fe_session_key(&form_dat.username, &sess_key).await {
            Ok(_) => {},
            Err(err) => {println!("Error adding session key: {}", err);
                return Err(reject::custom(Rejections::ErrorAddingSessionKey))},
        };

        println!("Authenticated: {:?} key {}", form_dat, sess_key);

        match password_hash_version::validate_pw_ver(&form_dat.username,
            &form_dat.password, &pass_hash, hash_ver)
        {
            Ok(true) =>
                Ok(warp::reply::with_header(
                    StatusCode::OK,
                    "Set-Cookie", 
                    format!("{}={}; Max-Age={}", 
                        api::SESSION_COOKIE_NAME, sess_key,
                        api::SESSION_COOKIE_MAX_AGE)
                )),
            Ok(false) => {println!("Wrong password");
                Err(reject::custom(Rejections::InvalidPassword))},
            Err(err) => {println!("Error validating hash: {}", err);
                Err(reject::custom(Rejections::HashValidationError))},
        }
    }

    pub async fn create_account(db: db::Db, form_dat: models::CreateAcctForm)
        -> Result<impl warp::Reply, warp::Rejection>
    {
        // TODO: handle properly when the rand number is already in the DB
        let reg_key = gen_large_rand_str();
        println!("Adding user with reg key {}", reg_key);

        match db.add_user(&form_dat.username, &form_dat.password, &reg_key).await {
            Ok(_) => Ok(StatusCode::OK),
            Err(err) => {println!("Error adding user: {}", err); 
                Err(reject::custom(Rejections::ErrorCreatingUser))},
        }
    }

    pub async fn validate_account(db: db::Db,
        opts: models::ValidateAccountRequest)
        -> Result<impl warp::Reply, warp::Rejection>
    {
        match db.validate_account(&opts.val_code).await {
            Ok(_) => Ok(StatusCode::OK),
            Err(err) => {println!("Error validating account: {}", err); 
                Err(reject::custom(Rejections::ErrorValidatingAccount))},
        }
        
    }

    pub async fn validate_session_fe(_db: db::Db, _sess_info: (String, i32))
        -> Result<impl warp::Reply, warp::Rejection>
    {
        // If we can get to here, we're ok
        // TODO - what's the right response?
        Ok(StatusCode::OK)
    }

    pub async fn logout_session_fe(db: db::Db, sess_info: (String, i32))
        -> Result<impl warp::Reply, warp::Rejection>
    {
        let (sess_key, _user_id) = sess_info;

        // TODO - what's the right response?
        match db.logout_fe_session_key(&sess_key).await {
            Ok(_) => Ok(StatusCode::OK),
            Err(err) => {println!("Error logging out account: {}", err); 
                Err(reject::custom(Rejections::ErrorValidatingAccount))},
        }
    }

    pub async fn get_channel_lists(db: db::Db, sess_info: (String, i32))
        -> Result<impl warp::Reply, warp::Rejection>
    {
        let (_sess_key, user_id) = sess_info;

        match db.get_channel_lists(user_id).await {
            Ok(val) => Ok(warp::reply::html(val)),
            Err(err) => {println!("Error getting channel lists: {}", err); 
                Err(reject::custom(Rejections::ErrorGettingChannelLists))},
        }
    }

    pub async fn get_channel_list(db: db::Db, sess_info: (String, i32), 
        opts: models::GetChannelListQuery)
        -> Result<impl warp::Reply, warp::Rejection>
    {
        let (_sess_key, user_id) = sess_info;

        match db.get_channel_list(user_id, &opts.list_name).await {
            Ok(val) => Ok(warp::reply::html(val)),
            Err(err) => {println!("Error getting channel list: {}", err); 
                Err(reject::custom(Rejections::ErrorGettingChannelList))},
        }
    }

    pub async fn set_channel_list(db: db::Db, sess_info: (String, i32), 
        form_dat: models::SetChannelListForm)
        -> Result<impl warp::Reply, warp::Rejection>
    {
        let (_sess_key, user_id) = sess_info;

        // TODO validate that input is json
        // TODO convert to XML now?

        match db.set_channel_list(user_id, &form_dat.listname,
            &form_dat.listdata).await
        {
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

        match db.create_channel_list(user_id, &form_dat.listname).await {
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

        match db.set_active_channel(user_id, &form_dat.listname).await {
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
