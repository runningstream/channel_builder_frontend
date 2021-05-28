#[macro_use] extern crate diesel;

pub mod schema;

use diesel::pg::PgConnection;
use diesel::Connection;
use diesel::RunQueryDsl;

#[tokio::main]
async fn main() {

    // Setup DB with arc mutex?
    let db_url = "postgres://postgres:mysecretpassword@localhost/roku_channel_builder";
    let db_conn = PgConnection::establish(&db_url)
        .expect("Unable to connect to database");

    // TODO REMOVE!!!
    add_temp_user(&db_conn, "temp_user", "temp_pw_hash");

    // Setup email handler?
    //let email = 

    let api = api::build_filters();
    let server_address = "127.0.0.1:3031";
    let server_sockaddr: std::net::SocketAddr = server_address
        .parse()
        .expect("Unable to parse socket address");
    warp::serve(api).run(server_sockaddr).await;
}

// TODO REMOVE!!!
fn add_temp_user(db_conn: &PgConnection, user: &str, pw_hsh: &str)
        -> Result<(), String>
{
    use crate::schema::user_data;

    #[derive(Insertable)]
    #[table_name="user_data"]
    struct NewUser<'a> {
        pub username: &'a str,
        pub pass_hash: &'a str,
        pub pass_hash_type: i32,
        pub validation_status: bool,
    }

    let new_user = NewUser {
        username: user,
        pass_hash: pw_hsh,
        pass_hash_type: 1,
        validation_status: true,
    };

    match diesel::insert_into(user_data::table)
        .values(&new_user)
        .execute(db_conn)
    {
        Ok(1) => {println!("Success adding user");},
        val => {println!("Error {:?}", val);},
    }

    Ok(())
}

mod api {
    use super::{api_handlers, models, rejections};
    use warp::{Filter, reject, Rejection, Reply};
    use warp::http::StatusCode;

    static SESSION_COOKIE_NAME: &str = "session";
    const MAX_AUTH_FORM_LEN: u64 = 4096;

    pub fn build_filters()
        -> impl Filter<Extract = impl warp::Reply, Error = warp::Rejection> + Clone
    {
        api_authenticate_fe()
            .or(api_validate_session_fe())
            .or(api_get_channel_lists())
            .or(api_get_channel_list())
            .recover(handle_rejection)
    }

    fn api_authenticate_fe()
        -> impl Filter<Extract = impl warp::Reply, Error = warp::Rejection> + Clone
    {
        api_v1_path("authenticate_fe")
            .and(warp::post())
            .and(auth_form())
            .and_then(api_handlers::authenticate_fe)
    }

    fn api_validate_session_fe()
        -> impl Filter<Extract = impl warp::Reply, Error = warp::Rejection> + Clone
    {
        api_v1_path("validate_session_fe")
            .and(warp::get())
            .and(validate_fe_session())
            .and_then(api_handlers::validate_session_fe)
    }

    fn api_get_channel_lists()
        -> impl Filter<Extract = impl warp::Reply, Error = warp::Rejection> + Clone
    {
        api_v1_path("get_channel_lists")
            .and(warp::get())
            .and(validate_fe_session())
            .and_then(api_handlers::get_channel_lists)
    }

    fn api_get_channel_list()
        -> impl Filter<Extract = impl warp::Reply, Error = warp::Rejection> + Clone
    {
        api_v1_path("get_channel_list")
            .and(warp::get())
            .and(validate_fe_session())
            .and(warp::query::<models::GetChannelListQuery>())
            .and_then(api_handlers::get_channel_list)
    }

    fn auth_form()
        -> impl Filter<Extract = (models::AuthForm,), Error = warp::Rejection> + Clone
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
                    Err(reject::custom(rejections::InvalidSession))
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

    async fn handle_rejection(err: Rejection)
        -> Result<impl Reply, warp::Rejection>
    {
        let code;
        let message: String;

        if let Some(rejections::InvalidSession) = err.find() {
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

mod rejections {
    use warp::reject;

    #[derive(Debug)]
    pub struct InvalidSession;

    impl reject::Reject for InvalidSession {}
}

mod models {
    use serde::{Deserialize, Serialize};

    #[derive(Debug,Deserialize,Serialize,Clone)]
    pub struct AuthForm {
        pub username: String,
        pub password: String,
    }
    
    #[derive(Debug,Deserialize,Serialize,Clone)]
    pub struct GetChannelListQuery {
        pub list_name: String,
    }
}

mod api_handlers {
    use super::models;
    use std::convert::Infallible;
    use warp::http::StatusCode;

    pub async fn authenticate_fe(form_dat: models::AuthForm)
        -> Result<impl warp::Reply, Infallible>
    {
        println!("Authenticate: {:?}", form_dat);
        Ok(StatusCode::OK)
    }

    pub async fn validate_session_fe(session_cookie: String)
        -> Result<impl warp::Reply, Infallible>
    {
        // If we can get to here, we're ok
        Ok(warp::reply::html("Success"))
    }

    pub async fn get_channel_lists(session_cookie: String)
        -> Result<impl warp::Reply, Infallible>
    {
        Ok(warp::reply::html("['channel lists']"))
    }

    pub async fn get_channel_list(session_cookie: String, opts: models::GetChannelListQuery)
        -> Result<impl warp::Reply, Infallible>
    {
        Ok(warp::reply::html("['channel lists']"))
    }
}
