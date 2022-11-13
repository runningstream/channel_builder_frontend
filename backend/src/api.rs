//! This module implements the API interface, and this documentation describes that interface.

use crate::{api_handlers, helpers, models, LOG_KEY};
pub use api_handlers::{APIParams, orderly_shutdown};
use chrono::prelude::{DateTime, Utc};
use helpers::{SessType};
use warp::{Filter, Reply, Rejection};

#[doc(hidden)]
#[macro_export]
macro_rules! APICORS {
    (true) => { "Requires correct origin/referer" };
    (false) => { "Not required or supported" };
}

#[doc(hidden)]
#[macro_export]
macro_rules! APIDATA {
    ( $str:literal ) => { concat!(
        "\n\nData: ", $str, "\n\nContent-Type: x-www-form-urlencoded"
    )};
    () => { "" };
}

#[doc(hidden)]
#[macro_export]
macro_rules! APIPathV1 {
    ( $tail:literal ) => { concat!(".../api/v1/", $tail) };
}

#[doc(hidden)]
#[macro_export]
macro_rules! APIMethod {
    (POST) => { "POST" };
    (GET) => { "GET" };
}

#[doc(hidden)]
#[macro_export]
macro_rules! APISessType {
    (FRONTEND) => {" - frontend sessions"};
    (ROKU) => {" - roku sessions"};
    (DISPLAY) => {" - display sessions"};
    () => {""};
}

#[doc(hidden)]
#[macro_export]
macro_rules! APIDocs {
    (
        Desc:$desc:literal,
        URL:$url:expr,
        Sess:$sess:expr,
        Method:$method:expr,
        CORS:$cors:expr,
        Data:$data:expr,
        $($tt:tt)*
    ) => {
        #[doc=concat!(
            "# ", $desc, $sess,
            "\n\nURL endpoint: ", $url,
            "\n\nMethod: ", $method,
            $data,
            "\n\nCORS: ", $cors,
        )]
        $($tt)*
    };
}

// TODO is this big enough?
#[doc(hidden)]
const MAX_AUTH_FORM_LEN: u64 = 1024 * 256;

/// Not part of the public API - creates the full filter string to permit Warp framework to kick-off
pub fn build_filters(params: APIParams, cors_origins: Vec<String>, startup_time: DateTime<Utc>)
    -> impl Filter<Extract = impl Reply, Error = Rejection> + Clone
{
    // Setup warp's built in CORS
    let cors = warp::cors()
        .allow_origins(cors_origins.iter().map(|s| s.as_ref()))
        .allow_methods(vec!["GET", "POST"])
        //.allow_headers(vec!["content-type"]) // Generally not required
        .allow_credentials(true);

    // Permit Roku endpoints and status to avoid origin_referer_filt
    // Require browser endpoints to meet that filter requirement
    api_get_status_report(startup_time, params.clone())
        .or(api_authenticate_ro(params.clone()))
        .or(api_validate_session_ro(params.clone()))
        .or(api_logout_session_ro(params.clone()))
        .or(api_get_active_channel_ro(params.clone()))
        .or(api_get_channel_xml_ro(params.clone()))
        .or(api_refresh_session_ro(params.clone()))
        .or(
            origin_referer_filt(cors_origins.clone()).and(
                api_create_account(params.clone())
                    .or(api_validate_account(params.clone()))
                    .or(api_authenticate_fe(params.clone()))
                    .or(api_authenticate_di(params.clone()))
                    .or(api_validate_session_fe(params.clone()))
                    .or(api_validate_session_di(params.clone()))
                    .or(api_logout_session_fe(params.clone()))
                    .or(api_logout_session_di(params.clone()))
                    .or(api_get_active_channel_fe(params.clone()))
                    .or(api_get_active_channel_di(params.clone()))
                    .or(api_get_channel_list_fe(params.clone()))
                    .or(api_get_channel_list_di(params.clone()))
                    .or(api_get_channel_lists_fe(params.clone()))
                    .or(api_set_channel_list_fe(params.clone()))
                    .or(api_create_channel_list_fe(params.clone()))
                    .or(api_set_active_channel_fe(params.clone()))
                    .or(api_get_active_channel_name_fe(params.clone()))
                    .or(api_refresh_session_di(params.clone()))
            )
        )
        .recover(api_handlers::handle_rejection)
        .with(cors)
        .with(warp::log(LOG_KEY))
}

APIDocs!{
    Desc: "Create an account",
    URL: APIPathV1!("create_account"),
    Sess: APISessType!(),
    Method: APIMethod!(POST),
    CORS: APICORS!(true),
    Data: APIDATA!("username and password strings"),

    fn api_create_account(params: APIParams)
        -> impl Filter<Extract = impl Reply, Error = Rejection> + Clone
    {
        // TODO Do I return neutral responses when the email already exists - failed?
        api_v1_path("create_account")
            .and(warp::post())
            .and(add_in(params))
            .and(get_form::<models::CreateAcctForm>())
            .and_then(api_handlers::create_account)
    }
}

APIDocs!{
    Desc: "Validate an account via email link",
    URL: APIPathV1!("validate_account"),
    Sess: APISessType!(),
    Method: APIMethod!(GET),
    CORS: APICORS!(true),
    Data: APIDATA!("val_code string"),

    fn api_validate_account(params: APIParams)
        -> impl Filter<Extract = impl Reply, Error = Rejection> + Clone
    {
        api_v1_path("validate_account")
            .and(warp::get())
            .and(add_in(params))
            .and(warp::query::<models::ValidateAccountRequest>())
            .and_then(api_handlers::validate_account)
    }
}

APIDocs!{
    Desc: "Retrieve the server status report",
    URL: APIPathV1!("status_report"),
    Sess: APISessType!(),
    Method: APIMethod!(GET),
    CORS: APICORS!(false),
    Data: APIDATA!(),

    fn api_get_status_report(startup_time: DateTime<Utc>, params: APIParams)
        -> impl Filter<Extract = impl Reply, Error = Rejection> + Clone
    {
        api_v1_path("status_report")
            .and(warp::get())
            .and(add_in(startup_time))
            .and(add_in(params))
            .and_then(api_handlers::get_status_report)
    }
}

APIDocs!{
    Desc: "Authenticate users",
    URL: APIPathV1!("authenticate_fe"),
    Sess: APISessType!(FRONTEND),
    Method: APIMethod!(POST),
    CORS: APICORS!(true),
    Data: APIDATA!("username and password strings"),

    fn api_authenticate_fe(params: APIParams)
        -> impl Filter<Extract = impl Reply, Error = Rejection> + Clone
    {
        // TODO do I return neutral responses when email doesn't exist vs
        // bad auth?
        api_v1_path("authenticate_fe")
            .and(warp::post())
            .and(add_in(SessType::Frontend))
            .and(add_in(params))
            .and(get_form::<models::AuthForm>())
            .and_then(api_handlers::authenticate)
    }
}

APIDocs!{
    Desc: "Authenticate users",
    URL: APIPathV1!("authenticate_di"),
    Sess: APISessType!(DISPLAY),
    Method: APIMethod!(POST),
    CORS: APICORS!(true),
    Data: APIDATA!("username and password strings"),

    fn api_authenticate_di(params: APIParams)
        -> impl Filter<Extract = impl Reply, Error = Rejection> + Clone
    {
        // TODO do I return neutral responses when email doesn't exist vs
        // bad auth?
        api_v1_path("authenticate_di")
            .and(warp::post())
            .and(add_in(SessType::Display))
            .and(add_in(params))
            .and(get_form::<models::AuthForm>())
            .and_then(api_handlers::authenticate)
    }
}

APIDocs!{
    Desc: "Authenticate users",
    URL: APIPathV1!("authenticate_ro"),
    Sess: APISessType!(ROKU),
    Method: APIMethod!(POST),
    CORS: APICORS!(false),
    Data: APIDATA!("username and password strings"),

    fn api_authenticate_ro(params: APIParams)
        -> impl Filter<Extract = impl Reply, Error = Rejection> + Clone
    {
        // TODO do I return neutral responses when email doesn't exist vs
        // bad auth?
        api_v1_path("authenticate_ro")
            .and(warp::post())
            .and(add_in(SessType::Roku))
            .and(add_in(params))
            .and(get_form::<models::AuthForm>())
            .and_then(api_handlers::authenticate)
    }
}

APIDocs!{
    Desc: "Validate a session",
    URL: APIPathV1!("validate_session_fe"),
    Sess: APISessType!(FRONTEND),
    Method: APIMethod!(GET),
    CORS: APICORS!(true),
    Data: APIDATA!(),

    fn api_validate_session_fe(params: APIParams)
        -> impl Filter<Extract = impl Reply, Error = Rejection> + Clone
    {
        api_v1_path("validate_session_fe")
            .and(warp::get())
            .and(add_in(SessType::Frontend))
            .and(add_in(params.clone()))
            .and(validate_session(SessType::Frontend, params))
            .and_then(api_handlers::validate_session)
    }
}

APIDocs!{
    Desc: "Validate a session",
    URL: APIPathV1!("validate_session_ro"),
    Sess: APISessType!(ROKU),
    Method: APIMethod!(GET),
    CORS: APICORS!(false),
    Data: APIDATA!(),

    fn api_validate_session_ro(params: APIParams)
        -> impl Filter<Extract = impl Reply, Error = Rejection> + Clone
    {
        api_v1_path("validate_session_ro")
            .and(warp::get())
            .and(add_in(SessType::Roku))
            .and(add_in(params.clone()))
            .and(validate_session(SessType::Roku, params))
            .and_then(api_handlers::validate_session)
    }
}

APIDocs!{
    Desc: "Validate a session",
    URL: APIPathV1!("validate_session_di"),
    Sess: APISessType!(DISPLAY),
    Method: APIMethod!(GET),
    CORS: APICORS!(true),
    Data: APIDATA!(),

    fn api_validate_session_di(params: APIParams)
        -> impl Filter<Extract = impl Reply, Error = Rejection> + Clone
    {
        api_v1_path("validate_session_di")
            .and(warp::get())
            .and(add_in(SessType::Display))
            .and(add_in(params.clone()))
            .and(validate_session(SessType::Display, params))
            .and_then(api_handlers::validate_session)
    }
}

APIDocs!{
    Desc: "Logout a session",
    URL: APIPathV1!("logout_session_fe"),
    Sess: APISessType!(FRONTEND),
    Method: APIMethod!(GET),
    CORS: APICORS!(true),
    Data: APIDATA!(),

    fn api_logout_session_fe(params: APIParams)
        -> impl Filter<Extract = impl Reply, Error = Rejection> + Clone
    {
        api_v1_path("logout_session_fe")
            .and(warp::get())
            .and(add_in(SessType::Frontend))
            .and(add_in(params.clone()))
            .and(validate_session(SessType::Frontend, params))
            .and_then(api_handlers::logout_session)
    }
}

APIDocs!{
    Desc: "Logout a session",
    URL: APIPathV1!("logout_session_ro"),
    Sess: APISessType!(ROKU),
    Method: APIMethod!(GET),
    CORS: APICORS!(false),
    Data: APIDATA!(),

    fn api_logout_session_ro(params: APIParams)
        -> impl Filter<Extract = impl Reply, Error = Rejection> + Clone
    {
        api_v1_path("logout_session_ro")
            .and(warp::get())
            .and(add_in(SessType::Roku))
            .and(add_in(params.clone()))
            .and(validate_session(SessType::Roku, params))
            .and_then(api_handlers::logout_session)
    }
}

APIDocs!{
    Desc: "Logout a session",
    URL: APIPathV1!("logout_session_di"),
    Sess: APISessType!(DISPLAY),
    Method: APIMethod!(GET),
    CORS: APICORS!(true),
    Data: APIDATA!(),

    fn api_logout_session_di(params: APIParams)
        -> impl Filter<Extract = impl Reply, Error = Rejection> + Clone
    {
        api_v1_path("logout_session_di")
            .and(warp::get())
            .and(add_in(SessType::Display))
            .and(add_in(params.clone()))
            .and(validate_session(SessType::Display, params))
            .and_then(api_handlers::logout_session)
    }
}

APIDocs!{
    Desc: "List all the channel lists",
    URL: APIPathV1!("get_channel_lists_fe"),
    Sess: APISessType!(FRONTEND),
    Method: APIMethod!(GET),
    CORS: APICORS!(true),
    Data: APIDATA!(),

    fn api_get_channel_lists_fe(params: APIParams)
        -> impl Filter<Extract = impl Reply, Error = Rejection> + Clone
    {
        api_v1_path("get_channel_lists_fe")
            .or(api_v1_path("get_channel_lists")) // TODO remove in a future version
            .unify() // TODO remove this with the prior line
            .and(warp::get())
            .and(add_in(params.clone()))
            .and(validate_session(SessType::Frontend, params))
            .and_then(api_handlers::get_channel_lists)
    }
}

APIDocs!{
    Desc: "Get a channel list's content",
    URL: APIPathV1!("get_channel_list_fe"),
    Sess: APISessType!(FRONTEND),
    Method: APIMethod!(GET),
    CORS: APICORS!(true),
    Data: APIDATA!("list_name string"),

    fn api_get_channel_list_fe(params: APIParams)
        -> impl Filter<Extract = impl Reply, Error = Rejection> + Clone
    {
        api_v1_path("get_channel_list_fe")
            .or(api_v1_path("get_channel_list")) // TODO remove in a future version
            .unify() // TODO remove this with the prior line
            .and(warp::get())
            .and(add_in(SessType::Frontend))
            .and(add_in(params.clone()))
            .and(validate_session(SessType::Frontend, params))
            .and(warp::query::<models::GetChannelListQuery>())
            .and_then(api_handlers::get_channel_list)
    }
}

APIDocs!{
    Desc: "Get a channel list's content",
    URL: APIPathV1!("get_channel_list_di"),
    Sess: APISessType!(DISPLAY),
    Method: APIMethod!(GET),
    CORS: APICORS!(true),
    Data: APIDATA!("list_name string"),

    fn api_get_channel_list_di(params: APIParams)
        -> impl Filter<Extract = impl Reply, Error = Rejection> + Clone
    {
        api_v1_path("get_channel_list_di")
            .and(warp::get())
            .and(add_in(SessType::Display))
            .and(add_in(params.clone()))
            .and(validate_session(SessType::Display, params))
            .and(warp::query::<models::GetChannelListQuery>())
            .and_then(api_handlers::get_channel_list)
    }
}

APIDocs!{
    Desc: "Get the active channel list's content in XML format",
    URL: APIPathV1!("get_channel_xml_ro"),
    Sess: APISessType!(ROKU),
    Method: APIMethod!(GET),
    CORS: APICORS!(false),
    Data: APIDATA!(),

    fn api_get_channel_xml_ro(params: APIParams)
        -> impl Filter<Extract = impl Reply, Error = Rejection> + Clone
    {
        api_v1_path("get_channel_xml_ro")
            .and(warp::get())
            .and(add_in(SessType::Roku))
            .and(add_in(params.clone()))
            .and(validate_session(SessType::Roku, params))
            .and_then(api_handlers::get_channel_xml)
    }
}

APIDocs!{
    Desc: "Refresh a session key",
    URL: APIPathV1!("refresh_session_di"),
    Sess: APISessType!(DISPLAY),
    Method: APIMethod!(GET),
    CORS: APICORS!(true),
    Data: APIDATA!(),

    fn api_refresh_session_di(params: APIParams)
        -> impl Filter<Extract = impl Reply, Error = Rejection> + Clone
    {
        api_v1_path("refresh_session_di")
            .and(warp::get())
            .and(add_in(SessType::Display))
            .and(add_in(params.clone()))
            .and(validate_session(SessType::Display, params))
            .and_then(api_handlers::refresh_session)
    }
}

APIDocs!{
    Desc: "Refresh a session key",
    URL: APIPathV1!("refresh_session_ro"),
    Sess: APISessType!(ROKU),
    Method: APIMethod!(GET),
    CORS: APICORS!(false),
    Data: APIDATA!(),

    fn api_refresh_session_ro(params: APIParams)
        -> impl Filter<Extract = impl Reply, Error = Rejection> + Clone
    {
        api_v1_path("refresh_session_ro")
            .and(warp::get())
            .and(add_in(SessType::Roku))
            .and(add_in(params.clone()))
            .and(validate_session(SessType::Roku, params))
            .and_then(api_handlers::refresh_session)
    }
}

APIDocs!{
    Desc: "Set a channel list's content",
    URL: APIPathV1!("set_channel_list_fe"),
    Sess: APISessType!(FRONTEND),
    Method: APIMethod!(POST),
    CORS: APICORS!(true),
    Data: APIDATA!("listname and listdata strings (listdata should be JSON)"),

    fn api_set_channel_list_fe(params: APIParams)
        -> impl Filter<Extract = impl Reply, Error = Rejection> + Clone
    {
        api_v1_path("set_channel_list_fe")
            .or(api_v1_path("set_channel_list")) // TODO remove in a future version
            .unify() // TODO remove this with the prior line
            .and(warp::post())
            .and(add_in(params.clone()))
            .and(validate_session(SessType::Frontend, params))
            .and(get_form::<models::SetChannelListForm>())
            .and_then(api_handlers::set_channel_list)
    }
}

APIDocs!{
    Desc: "Create a new channel list",
    URL: APIPathV1!("create_channel_list_fe"),
    Sess: APISessType!(FRONTEND),
    Method: APIMethod!(POST),
    CORS: APICORS!(true),
    Data: APIDATA!("listname string"),

    fn api_create_channel_list_fe(params: APIParams)
        -> impl Filter<Extract = impl Reply, Error = Rejection> + Clone
    {
        api_v1_path("create_channel_list_fe")
            .or(api_v1_path("create_channel_list")) // TODO remove in a future version
            .unify() // TODO remove this with the prior line
            .and(warp::post())
            .and(add_in(params.clone()))
            .and(validate_session(SessType::Frontend, params))
            .and(get_form::<models::CreateChannelListForm>())
            .and_then(api_handlers::create_channel_list)
    }
}

APIDocs!{
    Desc: "Set the active channel",
    URL: APIPathV1!("set_active_channel_fe"),
    Sess: APISessType!(FRONTEND),
    Method: APIMethod!(POST),
    CORS: APICORS!(true),
    Data: APIDATA!("listname string"),

    fn api_set_active_channel_fe(params: APIParams)
        -> impl Filter<Extract = impl Reply, Error = Rejection> + Clone
    {
        api_v1_path("set_active_channel_fe")
            .or(api_v1_path("set_active_channel")) // TODO remove in a future version
            .unify() // TODO remove this with the prior line
            .and(warp::post())
            .and(add_in(params.clone()))
            .and(validate_session(SessType::Frontend, params))
            .and(get_form::<models::SetActiveChannelForm>())
            .and_then(api_handlers::set_active_channel)
    }
}

APIDocs!{
    Desc: "Return the content in the active channel",
    URL: APIPathV1!("get_active_channel_fe"),
    Sess: APISessType!(FRONTEND),
    Method: APIMethod!(GET),
    CORS: APICORS!(true),
    Data: APIDATA!(),

    fn api_get_active_channel_fe(params: APIParams)
        -> impl Filter<Extract = impl Reply, Error = Rejection> + Clone
    {
        api_v1_path("get_active_channel_fe")
            .and(warp::get())
            .and(add_in(SessType::Frontend))
            .and(add_in(params.clone()))
            .and(validate_session(SessType::Frontend, params))
            .and_then(api_handlers::get_active_channel)
    }
}

APIDocs!{
    Desc: "Return the content in the active channel",
    URL: APIPathV1!("get_active_channel_di"),
    Sess: APISessType!(DISPLAY),
    Method: APIMethod!(GET),
    CORS: APICORS!(true),
    Data: APIDATA!(),

    fn api_get_active_channel_di(params: APIParams)
        -> impl Filter<Extract = impl Reply, Error = Rejection> + Clone
    {
        api_v1_path("get_active_channel_di")
            .and(warp::get())
            .and(add_in(SessType::Display))
            .and(add_in(params.clone()))
            .and(validate_session(SessType::Display, params))
            .and_then(api_handlers::get_active_channel)
    }
}

APIDocs!{
    Desc: "Return the content in the active channel",
    URL: APIPathV1!("get_active_channel_ro"),
    Sess: APISessType!(ROKU),
    Method: APIMethod!(GET),
    CORS: APICORS!(true),
    Data: APIDATA!(),

    fn api_get_active_channel_ro(params: APIParams)
        -> impl Filter<Extract = impl Reply, Error = Rejection> + Clone
    {
        api_v1_path("get_active_channel_ro")
            .and(warp::get())
            .and(add_in(SessType::Roku))
            .and(add_in(params.clone()))
            .and(validate_session(SessType::Roku, params))
            .and_then(api_handlers::get_active_channel)
    }
}

APIDocs!{
    Desc: "Return the name of the active channel",
    URL: APIPathV1!("get_active_channel_name_fe"),
    Sess: APISessType!(FRONTEND),
    Method: APIMethod!(GET),
    CORS: APICORS!(true),
    Data: APIDATA!(),

    fn api_get_active_channel_name_fe(params: APIParams)
        -> impl Filter<Extract = impl Reply, Error = Rejection> + Clone
    {
        api_v1_path("get_active_channel_name_fe")
            .and(warp::get())
            .and(add_in(SessType::Frontend))
            .and(add_in(params.clone()))
            .and(validate_session(SessType::Frontend, params))
            .and_then(api_handlers::get_active_channel_name)
    }
}

#[doc(hidden)]
fn get_form<T>()
    -> impl Filter<Extract = (T,), Error = Rejection> + Clone
    where
        T: Send,
        T: for<'de> serde::Deserialize<'de>
{
    warp::body::content_length_limit(MAX_AUTH_FORM_LEN)
        .and(warp::body::form())
}

#[doc(hidden)]
fn validate_session(sess_type: SessType, params: APIParams)
    -> impl Filter<Extract = ((String, i32),), Error = Rejection> + Clone
{
    warp::filters::cookie::cookie::<String>(sess_type.get_session_cookie_name())
        .and(add_in(params))
        .and(add_in(sess_type))
        .and_then(
            api_handlers::retrieve_session_dat
        )
}

#[doc(hidden)]
fn api_v1_path(api_tail: &str)
    -> impl Filter<Extract = (), Error = Rejection> + Clone + '_
{
    warp::path("api")
        .and(warp::path("v1"))
        .and(warp::path(api_tail))
        .and(warp::path::end())
}

#[doc(hidden)]
fn add_in<THING>(thing: THING)
    -> impl Filter<Extract = (THING,), Error = std::convert::Infallible>
        + Clone
where
    THING: Clone + Send
{
    warp::any().map(move || thing.clone())
}

// Make sure that either the origin or referer headers are present
// Validate that it starts with the expected cors_origin
// This provides CSRF protection for modern browsers
#[doc(hidden)]
fn origin_referer_filt(cors_origins: Vec<String>)
    -> impl Filter<Extract = (), Error = Rejection> + Clone
{
    warp::header("origin")
        .or(warp::header("referer"))
        .unify()
        .and(add_in(cors_origins))
        .and_then(api_handlers::validate_origin_or_referer)
        .untuple_one()
}

