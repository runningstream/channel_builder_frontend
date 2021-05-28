#[macro_use] extern crate diesel;

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

mod db {
    use super::password_hash_version;
    use diesel::pg::PgConnection;
    use diesel::Connection;
    use diesel::RunQueryDsl;
    use std::fmt;
    use tokio::sync::Mutex;
    use std::sync::Arc;

    pub enum DBError {
        PassHashError,
        InvalidDBResponse,
    }

    impl fmt::Display for DBError {
        fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
            match *self {
                DBError::PassHashError => write!(f, "Error hashing password"),
                DBError::InvalidDBResponse => write!(f, "Invalid DB response"),
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
            let db_arc = Arc::new(Mutex::new(db_conn));

            Self {
                db_arc,
            }
        }

        pub async fn add_user(&self, user: &str, pass: &str)
                -> Result<(), DBError>
        {
            // TODO email vaildation
            use crate::schema::user_data;

            #[derive(Insertable)]
            #[table_name="user_data"]
            struct NewUser<'a> {
                pub username: &'a str,
                pub pass_hash: &'a str,
                pub pass_hash_type: i32,
                pub validation_status: bool,
            }

            let pw_hash = match password_hash_version::hash_pw(user, pass) {
                Ok(val) => val,
                Err(err) => {
                    println!("Error hashing password: {}", err);
                    return Err(DBError::PassHashError)},
            };

            let new_user = NewUser {
                username: user,
                pass_hash: &pw_hash,
                pass_hash_type: password_hash_version::get_pw_ver(),
                validation_status: true,
            };

            let db_conn = self.db_arc.lock().await;

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

        pub async fn get_user_passhash(&self, user: &str)
            -> Result<(String, i32), DBError>
        {
            use crate::diesel::{QueryDsl, ExpressionMethods};
            use crate::schema::user_data::dsl::user_data;
            use crate::schema::user_data::username;

            #[derive(Queryable)]
            pub struct UserData {
                pub id: i32,
                pub username: String,
                pub pass_hash: String,
                pub pass_hash_type: i32,
                pub validation_status: bool,
                pub validation_code: Option<String>,
                pub active_channel: Option<i32>,
            }

            let db_conn = self.db_arc.lock().await;
            let results = match user_data.filter(username.eq(user))
                .limit(5)
                .load::<UserData>(& *db_conn)
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

            Ok((results[0].pass_hash.clone(), results[0].pass_hash_type))
        }
    }
}

mod api {
    use super::{api_handlers, models, Rejections, db};
    use warp::{Filter, reject, Rejection, Reply};
    use warp::http::StatusCode;

    static SESSION_COOKIE_NAME: &str = "session";
    const MAX_AUTH_FORM_LEN: u64 = 4096;

    pub fn build_filters(db: db::Db)
        -> impl Filter<Extract = impl warp::Reply, Error = warp::Rejection> + Clone
    {
        api_authenticate_fe(db.clone())
            .or(api_create_account(db.clone()))
            .or(api_validate_session_fe(db.clone()))
            .or(api_get_channel_lists(db.clone()))
            .or(api_get_channel_list(db.clone()))
            .recover(handle_rejection)
    }

    fn api_authenticate_fe(db: db::Db)
        -> impl Filter<Extract = impl warp::Reply, Error = warp::Rejection> + Clone
    {
        // TODO do I return neutral responses when email doesn't exist vs
        // bad auth?
        api_v1_path("authenticate_fe")
            .and(warp::post())
            .and(with_db(db))
            .and(auth_form())
            .and_then(api_handlers::authenticate_fe)
    }

    fn api_create_account(db: db::Db)
        -> impl Filter<Extract = impl warp::Reply, Error = warp::Rejection> + Clone
    {
        // TODO Do I return neutral responses when the email already exists - failed?
        api_v1_path("create_account")
            .and(warp::post())
            .and(with_db(db))
            .and(create_acct_form())
            .and_then(api_handlers::create_account)
    }

    fn api_validate_session_fe(db: db::Db)
        -> impl Filter<Extract = impl warp::Reply, Error = warp::Rejection> + Clone
    {
        api_v1_path("validate_session_fe")
            .and(warp::get())
            .and(with_db(db))
            .and(validate_fe_session())
            .and_then(api_handlers::validate_session_fe)
    }

    fn api_get_channel_lists(db: db::Db)
        -> impl Filter<Extract = impl warp::Reply, Error = warp::Rejection> + Clone
    {
        api_v1_path("get_channel_lists")
            .and(warp::get())
            .and(with_db(db))
            .and(validate_fe_session())
            .and_then(api_handlers::get_channel_lists)
    }

    fn api_get_channel_list(db: db::Db)
        -> impl Filter<Extract = impl warp::Reply, Error = warp::Rejection> + Clone
    {
        api_v1_path("get_channel_list")
            .and(warp::get())
            .and(with_db(db))
            .and(validate_fe_session())
            .and(warp::query::<models::GetChannelListQuery>())
            .and_then(api_handlers::get_channel_list)
    }

    fn auth_form()
        -> impl Filter<Extract = (models::AuthForm,), Error = warp::Rejection> + Clone
    {
        warp::body::content_length_limit(MAX_AUTH_FORM_LEN).and(warp::body::form())
    }

    fn create_acct_form()
        -> impl Filter<Extract = (models::CreateAcctForm,),
            Error = warp::Rejection> + Clone
    {
        warp::body::content_length_limit(MAX_AUTH_FORM_LEN).and(warp::body::form())
    }

    fn validate_fe_session()
        -> impl Filter<Extract = (String,), Error = warp::Rejection> + Clone
    {
        warp::filters::cookie::cookie::<String>(SESSION_COOKIE_NAME)
            .and_then(|session_id: String| async move {
                if session_id == "RIGHT_KEY" {
                    Ok(session_id)
                } else {
                    Err(reject::custom(Rejections::InvalidSession))
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
            //code = StatusCode::INTERNAL_SERVER_ERROR;
            //message = format!("Unhandled error: {:?}", err);
            Err(err)
        }

    }
}

#[derive(Debug)]
enum Rejections { InvalidSession, InvalidUser, InvalidPassword,
    HashValidationError, ErrorCreatingUser }

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
    pub struct GetChannelListQuery {
        pub list_name: String,
    }
}

mod api_handlers {
    use super::{models, db, password_hash_version, Rejections};
    use warp::http::StatusCode;
    use warp::reject;

    pub async fn authenticate_fe(db: db::Db, form_dat: models::AuthForm)
        -> Result<impl warp::Reply, warp::Rejection>
    {
        let (pass_hash, hash_ver) = match db.get_user_passhash(&form_dat.username).await {
            Ok(vals) => vals,
            Err(err) => {println!("Error getting user: {}", err);
                return Err(reject::custom(Rejections::InvalidUser));},
        };
        println!("Authenticate: {:?}", form_dat);
        match password_hash_version::validate_pw_ver(&form_dat.username, &form_dat.password, &pass_hash, hash_ver) {
            Ok(true) => Ok(StatusCode::OK),
            Ok(false) => {println!("Wrong password");
                Err(reject::custom(Rejections::InvalidPassword))},
            Err(err) => {println!("Error validating hash: {}", err);
                Err(reject::custom(Rejections::HashValidationError))},
        }
    }

    pub async fn create_account(db: db::Db, form_dat: models::CreateAcctForm)
        -> Result<impl warp::Reply, warp::Rejection>
    {
        println!("In here");
        match db.add_user(&form_dat.username, &form_dat.password).await {
            Ok(_) => Ok(StatusCode::OK),
            Err(err) => {println!("Error adding user: {}", err); 
                Err(reject::custom(Rejections::ErrorCreatingUser))},
        }
    }

    pub async fn validate_session_fe(_db: db::Db, _session_cookie: String)
        -> Result<impl warp::Reply, warp::Rejection>
    {
        // If we can get to here, we're ok
        Ok(warp::reply::html("Success"))
    }

    pub async fn get_channel_lists(_db: db::Db, _session_cookie: String)
        -> Result<impl warp::Reply, warp::Rejection>
    {
        Ok(warp::reply::html("['channel lists']"))
    }

    pub async fn get_channel_list(_db: db::Db, _session_cookie: String, 
        _opts: models::GetChannelListQuery)
        -> Result<impl warp::Reply, warp::Rejection>
    {
        Ok(warp::reply::html("['channel lists']"))
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
