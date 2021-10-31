use crate::{api_handlers, db, helpers, email, models};
use helpers::{SessType, SESSION_COOKIE_NAME};
use warp::{Filter, Reply, Rejection};

// TODO is this big enough?
const MAX_AUTH_FORM_LEN: u64 = 1024 * 256;

pub const LOG_KEY: &str = "backend";

pub fn build_filters(db: db::Db, email: email::Email, cors_origin: String)
    -> impl Filter<Extract = impl Reply, Error = Rejection> + Clone
{
    // Setup warp's built in CORS
    let cors = warp::cors()
        .allow_origin(cors_origin.as_str())
        .allow_methods(vec!["GET", "POST"])
        .allow_credentials(true);

    // Permit Roku endpoints to avoid origin_referer_filt
    // Require browser endpoints to meet that filter requirement
    api_authenticate_ro(db.clone())
        .or(api_validate_session_ro(db.clone()))
        .or(api_get_channel_xml_ro(db.clone()))
        .or(
            origin_referer_filt(cors_origin.clone()).and(
                api_authenticate_fe(db.clone())
                    .or(api_create_account(db.clone(), email.clone()))
                    .or(api_validate_account(db.clone()))
                    .or(api_logout_session_fe(db.clone()))
                    .or(api_get_channel_lists(db.clone()))
                    .or(api_get_channel_list(db.clone()))
                    .or(api_set_channel_list(db.clone()))
                    .or(api_create_channel_list(db.clone()))
                    .or(api_set_active_channel(db.clone()))
                    .or(api_validate_session_fe(db.clone()))
                    //.or(serve_static_index())
                    //.or(serve_static_files())
            )
        )
        .recover(api_handlers::handle_rejection)
        .with(cors)
        .with(warp::log(LOG_KEY))
}

/*
fn serve_static_index()
    -> impl Filter<Extract = impl Reply, Error = Rejection> + Clone
{
    warp::path::end()
        .and(warp::fs::file("frontend/content/index.html"))
}

fn serve_static_files()
    -> impl Filter<Extract = impl Reply, Error = Rejection> + Clone
{
    warp::fs::dir("frontend/content")
}
*/

fn api_authenticate_fe(db: db::Db)
    -> impl Filter<Extract = impl Reply, Error = Rejection> + Clone
{
    // TODO do I return neutral responses when email doesn't exist vs
    // bad auth?
    api_v1_path("authenticate_fe")
        .and(warp::post())
        .and(add_in(db))
        .and(get_form::<models::AuthForm>())
        .and_then(api_handlers::authenticate_fe)
}

// Make sure that either the origin or referer headers are present
// Validate that it starts with the expected cors_origin
// This provides CSRF protection for modern browsers
fn origin_referer_filt(cors_origin: String)
    -> impl Filter<Extract = (), Error = Rejection> + Clone
{
    warp::header("origin")
        .or(warp::header("referer"))
        .unify()
        .and(add_in(cors_origin))
        .and_then(api_handlers::validate_origin_or_referer)
        .untuple_one()
}

fn api_authenticate_ro(db: db::Db)
    -> impl Filter<Extract = impl Reply, Error = Rejection> + Clone
{
    // TODO do I return neutral responses when email doesn't exist vs
    // bad auth?
    api_v1_path("authenticate_ro")
        .and(warp::post())
        .and(add_in(db))
        .and(get_form::<models::AuthForm>())
        .and_then(api_handlers::authenticate_ro)
}

fn api_create_account(db: db::Db, email: email::Email)
    -> impl Filter<Extract = impl Reply, Error = Rejection> + Clone
{
    // TODO Do I return neutral responses when the email already exists - failed?
    api_v1_path("create_account")
        .and(warp::post())
        .and(add_in(db))
        .and(add_in(email))
        .and(get_form::<models::CreateAcctForm>())
        .and_then(api_handlers::create_account)
}

fn api_validate_account(db: db::Db)
    -> impl Filter<Extract = impl Reply, Error = Rejection> + Clone
{
    api_v1_path("validate_account")
        .and(warp::get())
        .and(add_in(db))
        .and(warp::query::<models::ValidateAccountRequest>())
        .and_then(api_handlers::validate_account)
}

fn api_validate_session_fe(db: db::Db)
    -> impl Filter<Extract = impl Reply, Error = Rejection> + Clone
{
    api_v1_path("validate_session_fe")
        .and(warp::get())
        .and(add_in(db.clone()))
        .and(validate_session(SessType::Frontend, db))
        .and_then(api_handlers::validate_session_fe)
}

fn api_validate_session_ro(db: db::Db)
    -> impl Filter<Extract = impl Reply, Error = Rejection> + Clone
{
    api_v1_path("validate_session_ro")
        .and(warp::get())
        .and(add_in(db.clone()))
        .and(validate_session(SessType::Roku, db))
        .and_then(api_handlers::validate_session_ro)
}

fn api_logout_session_fe(db: db::Db)
    -> impl Filter<Extract = impl Reply, Error = Rejection> + Clone
{
    api_v1_path("logout_session_fe")
        .and(warp::get())
        .and(add_in(db.clone()))
        .and(validate_session(SessType::Frontend, db))
        .and_then(api_handlers::logout_session_fe)
}

fn api_get_channel_lists(db: db::Db)
    -> impl Filter<Extract = impl Reply, Error = Rejection> + Clone
{
    api_v1_path("get_channel_lists")
        .and(warp::get())
        .and(add_in(db.clone()))
        .and(validate_session(SessType::Frontend, db))
        .and_then(api_handlers::get_channel_lists)
}

fn api_get_channel_list(db: db::Db)
    -> impl Filter<Extract = impl Reply, Error = Rejection> + Clone
{
    api_v1_path("get_channel_list")
        .and(warp::get())
        .and(add_in(db.clone()))
        .and(validate_session(SessType::Frontend, db))
        .and(warp::query::<models::GetChannelListQuery>())
        .and_then(api_handlers::get_channel_list)
}

fn api_get_channel_xml_ro(db: db::Db)
    -> impl Filter<Extract = impl Reply, Error = Rejection> + Clone
{
    api_v1_path("get_channel_xml_ro")
        .and(warp::get())
        .and(add_in(db.clone()))
        .and(validate_session(SessType::Roku, db))
        .and_then(api_handlers::get_channel_xml_ro)
}

fn api_set_channel_list(db: db::Db)
    -> impl Filter<Extract = impl Reply, Error = Rejection> + Clone
{
    api_v1_path("set_channel_list")
        .and(warp::post())
        .and(add_in(db.clone()))
        .and(validate_session(SessType::Frontend, db))
        .and(get_form::<models::SetChannelListForm>())
        .and_then(api_handlers::set_channel_list)
}

fn api_create_channel_list(db: db::Db)
    -> impl Filter<Extract = impl Reply, Error = Rejection> + Clone
{
    api_v1_path("create_channel_list")
        .and(warp::post())
        .and(add_in(db.clone()))
        .and(validate_session(SessType::Frontend, db))
        .and(get_form::<models::CreateChannelListForm>())
        .and_then(api_handlers::create_channel_list)
}

fn api_set_active_channel(db: db::Db)
    -> impl Filter<Extract = impl Reply, Error = Rejection> + Clone
{
    api_v1_path("set_active_channel")
        .and(warp::post())
        .and(add_in(db.clone()))
        .and(validate_session(SessType::Frontend, db))
        .and(get_form::<models::SetActiveChannelForm>())
        .and_then(api_handlers::set_active_channel)
}

fn get_form<T>()
    -> impl Filter<Extract = (T,), Error = Rejection> + Clone
    where
        T: Send,
        T: for<'de> serde::Deserialize<'de>
{
    warp::body::content_length_limit(MAX_AUTH_FORM_LEN)
        .and(warp::body::form())
}

fn validate_session(sess_type: SessType, db: db::Db)
    -> impl Filter<Extract = ((String, i32),), Error = Rejection> + Clone
{
    warp::filters::cookie::cookie::<String>(SESSION_COOKIE_NAME)
        .and(add_in(db))
        .and(add_in(sess_type))
        .and_then(
            api_handlers::retrieve_session_dat
        )
}

fn api_v1_path(api_tail: &str)
    -> impl Filter<Extract = (), Error = Rejection> + Clone + '_
{
    warp::path("api")
        .and(warp::path("v1"))
        .and(warp::path(api_tail))
        .and(warp::path::end())
}

fn add_in<THING>(thing: THING)
    -> impl Filter<Extract = (THING,), Error = std::convert::Infallible>
        + Clone
where
    THING: Clone + Send
{
    warp::any().map(move || thing.clone())
}
