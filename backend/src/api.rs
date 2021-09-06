use crate::{api_handlers, db, helpers, email, models};
use db::{Action, Response};
use api_handlers::Rejections;
use helpers::{SessType, SESSION_COOKIE_NAME};
use warp::{Filter, reject, Rejection, Reply};
use warp::http::StatusCode;

// TODO fix this, it's too small
const MAX_AUTH_FORM_LEN: u64 = 4096;


pub fn build_filters(db: db::Db, email: email::Email, cors_origin: String)
    -> impl Filter<Extract = impl warp::Reply, Error = warp::Rejection> + Clone
{
    let cors = warp::cors()
        .allow_origin(cors_origin.as_str())
        .allow_methods(vec!["GET", "POST"])
        .allow_credentials(true);

    let origin_referer_filt = warp::header("origin")
            .or(warp::header("referer"))
            .unify()
            .and_then(move |source: String| {
                let cors_origin_dupe = cors_origin.clone();
                async move {
                    if source.starts_with(&cors_origin_dupe) {
                        Ok(())
                    } else {
                        Err(reject::custom(Rejections::InvalidOriginOrReferer))
                    }
                }
            })
            .untuple_one();

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
        .and(origin_referer_filt.clone())
        .recover(handle_rejection)
        .with(cors.clone())
}

/*
fn serve_static_index()
    -> impl Filter<Extract = impl warp::Reply, Error = warp::Rejection> + Clone
{
    warp::path::end()
        .and(warp::fs::file("frontend/content/index.html"))
}

fn serve_static_files()
    -> impl Filter<Extract = impl warp::Reply, Error = warp::Rejection> + Clone
{
    warp::fs::dir("frontend/content")
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
    let message;

    // TODO add in more error handling?

    if err.is_not_found() {
        code = StatusCode::NOT_FOUND;
        message = "Not Found";
    } else {
        match err.find() {
            Some(Rejections::InvalidUserLookup) |
            Some(Rejections::InvalidUserNonValidated) |
            Some(Rejections::InvalidPassword) |
            Some(Rejections::InvalidSession) |
            Some(Rejections::InvalidEmailAddr) |
            Some(Rejections::InvalidValidationCode)
            => {
                code = StatusCode::FORBIDDEN;
                message = "Forbidden";
            },
            Some(Rejections::InvalidOriginOrReferer)
            => {
                code = StatusCode::BAD_REQUEST;
                message = "Bad Request";
            },
            Some(Rejections::ErrorInternal(content))
            => {
                print!("ErrorInternal: {}", content);
                code = StatusCode::INTERNAL_SERVER_ERROR;
                message = "Internal Server Error";
            },
            other => {
                print!("Unhandled error on request: {:?}", other);
                code = StatusCode::INTERNAL_SERVER_ERROR;
                message = "Internal Server Error";
            },
        }
    }

    Ok(warp::reply::with_status(message.to_string(), code))
}
