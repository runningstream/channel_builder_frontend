use std::{error, fmt};
use std::sync::{Arc};
use crate::{db, email, helpers, models, password_hash_version};
use chrono::prelude::{DateTime, Utc};
use helpers::{SessType, MIN_PASSWORD_LEN};
use db::{Action, Response, DBError};
use password_hash_version::PWHashError;
use rand::Rng;
use tokio::sync::Mutex;
use warp::http::StatusCode;
use warp::{reject, Reply, Rejection};


#[derive(Debug)]
pub enum Rejections {
    // User-caused problems
    InvalidUserLookup, InvalidUserNonValidated,
    InvalidPassword, InvalidSession, InvalidEmailAddr,
    InvalidValidationCode, InvalidOriginOrReferer,
    InvalidRefreshSessionType,
    // System Problems
    ErrorInternal(String), ErrorFromPWHash(PWHashError),

    // DB error
    ErrorFromDB(DBError), ErrorDBAPI(String, String)
}

impl reject::Reject for Rejections {}

impl fmt::Display for Rejections {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "Rejections error: ")?;

        match &*self {
            Rejections::ErrorInternal(str) => write!(f, "internal: {}", str),
            Rejections::ErrorFromPWHash(err) => write!(f, "password hash: {}", err),
            Rejections::ErrorFromDB(err) => write!(f, "db: {}", err),
            Rejections::ErrorDBAPI(loc, retval) => write!(f, "db api {}: {}", loc, retval),
            other => write!(f, "{:?}", other),
        }
    }
}

impl error::Error for Rejections {
    fn source(&self) -> Option<&(dyn error::Error + 'static)> {
        match *self {
            Rejections::ErrorFromDB(ref err) => Some(err),
            _ => None,
        }
    }
}

impl From<DBError> for Rejections {
    fn from(err: DBError) -> Rejections {
        Rejections::ErrorFromDB(err)
    }
}

impl From<PWHashError> for Rejections {
    fn from(err: PWHashError) -> Rejections {
        Rejections::ErrorFromPWHash(err)
    }
}

impl Rejections {
    fn db_api_err(api: &str, resp: impl std::fmt::Debug) -> Rejections {
        Rejections::ErrorDBAPI(api.into(), format!("{:?}", resp))
    }
}

#[derive(Clone)]
pub struct StatusReportWrapper {
    inner_report: Arc<Mutex<InnerStatusReport>>,
}

impl StatusReportWrapper {
    pub fn new() -> Self {
        Self {
            inner_report: Arc::new(Mutex::new(InnerStatusReport::default())),
        }
    }

    pub async fn mod_report(&self, mod_func: impl Fn(&mut InnerStatusReport) -> ()) -> () {
        mod_func(&mut *self.inner_report.lock().await)
    }

    pub async fn read_report(&self) -> InnerStatusReport {
        *self.inner_report.lock().await
    }
}

#[derive(Debug, Copy, Clone, Default)]
pub struct InnerStatusReport {
    // Counts of times the API was triggered
    authenticate: u32,
    authenticate_ro: u32,
    authenticate_fe: u32,
    authenticate_di: u32,
    get_status_report: u32,
    validate_account: u32,
    create_account: u32,
    refresh_session: u32,
    refresh_session_ro: u32,
    refresh_session_di: u32,
    validate_session: u32,
    validate_session_fe: u32,
    validate_session_ro: u32,
    validate_session_di: u32,
    logout_session: u32,
    logout_session_fe: u32,
    logout_session_ro: u32,
    logout_session_di: u32,
    get_channel_lists: u32,
    get_channel_list: u32,
    get_channel_xml_ro: u32,
    set_channel_list: u32,
    rename_channel: u32,
    create_channel_list: u32,
    delete_channel: u32,
    set_active_channel: u32,
    get_active_channel_name: u32,
    get_active_channel: u32,
    get_active_channel_fe: u32,
    get_active_channel_ro: u32,
    get_active_channel_di: u32,

    // Other status
    auth_success: u32,
    account_created: u32,
}

impl fmt::Display for InnerStatusReport {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, concat!(
                "API Handler Status Report:\n",
                "  Authentications: {}\n",
                "    Successful: {}\n",
                "    Frontend Auths: {}\n",
                "    Roku Auths: {}\n",
                "    Display Auths: {}\n",
                "  Account Creations: {}\n",
                "    Attempted: {}\n",
                "  Refresh Session: {}\n",
                "    Roku: {}\n",
                "    Display: {}\n",
                "  Validations:\n",
                "    Account: {}\n",
                "    Session: {}\n",
                "      Frontend: {}\n",
                "      Roku: {}\n",
                "      Display: {}\n",
                "  Logouts: {}\n",
                "    Frontend: {}\n",
                "    Roku: {}\n",
                "    Display: {}\n",
                "  Channel Stuff:\n",
                "    Create: {}\n",
                "    Rename: {}\n",
                "    Set Active: {}\n",
                "    Delete: {}\n",
                "    Change Content: {}\n",
                "    Get Content: {}\n",
                "    Get XML Content Roku: {}\n",
                "    Get Channel Lists: {}\n",
                "    Get Active Name: {}\n",
                "    Get Active: {}\n",
                "      Frontend: {}\n",
                "      Roku: {}\n",
                "      Display: {}\n",
                "  Status Reports: {}\n"
                ),
            self.authenticate, self.auth_success,
            self.authenticate_fe, self.authenticate_ro,
            self.authenticate_di,
            self.account_created, self.create_account,
            self.refresh_session, self.refresh_session_ro,
            self.refresh_session_di,
            self.validate_account, self.validate_session,
            self.validate_session_fe, self.validate_session_ro,
            self.validate_session_di, self.logout_session,
            self.logout_session_fe, self.logout_session_ro,
            self.logout_session_di,
            self.create_channel_list, self.rename_channel,
            self.set_active_channel, self.delete_channel,
            self.set_channel_list, self.get_channel_list,
            self.get_channel_xml_ro, self.get_channel_lists,
            self.get_active_channel_name,
            self.get_active_channel, self.get_active_channel_fe,
            self.get_active_channel_ro, self.get_active_channel_di,
            self.get_status_report
        )
    }
}

#[derive(Clone)]
pub struct APIParams {
    db: db::Db,
    email: email::Email,
    // API Status Report
    a_s_r: StatusReportWrapper,
}

impl APIParams {
    pub fn new(db: db::Db, email: email::Email) -> Self {
        // a_s_r = API Status Report
        let a_s_r = StatusReportWrapper::new();
        Self {
            db,
            email,
            a_s_r,
        }
    }
}

pub async fn authenticate(sess_type: SessType, params: APIParams,
        form_dat: models::AuthForm
    )
    -> Result<impl Reply, Rejection>
{
    trace!("Beginning authenticate {:?}", sess_type);
    params.a_s_r.mod_report(|report: &mut InnerStatusReport| {
        match sess_type {
            SessType::Frontend => { report.authenticate_fe += 1; },
            SessType::Roku => { report.authenticate_ro += 1; },
            SessType::Display => { report.authenticate_di += 1; },
        }
        report.authenticate += 1;
    }).await;

    let (pass_hash, hash_ver, valid_status) = 
        match params.db.please(Action::GetUserPassHash {
            user: form_dat.username.clone(),
        }).await {
            Ok(Response::UserPassHash(pass_hash, hash_ver, valid_status)) => 
                Ok((pass_hash, hash_ver, valid_status)),
            Ok(resp) => Err(Rejections::db_api_err("GetUserPassHash", resp)),
            Err(DBError::InvalidRowCount(0)) => Err(Rejections::InvalidUserNonValidated.into()),
            Err(err) => Err(Rejections::from(err)),
        }?;

    // TODO - am I returning something that indicates a user does/doesn't exist
    if !valid_status {
        return Err(Rejections::InvalidUserNonValidated.into());
    }

    // TODO - I was authenticating the password after making the session key
    // that didn't seem to make sense.  Now make sure this version makes sense...
    match password_hash_version::validate_pw_ver(&form_dat.username,
        &form_dat.password, &pass_hash, hash_ver)
    {
        Ok(true) => Ok(()),
        Ok(false) => Err(Rejections::InvalidPassword),
        Err(err) => Err(Rejections::from(err))
    }?;

    let sess_key = gen_large_rand_str();

    match params.db.please(Action::AddSessKey {
        user: db::NameOrID::Name(form_dat.username.clone()),
        sess_type: sess_type.clone(),
        sess_key: sess_key.clone(),
    }).await {
        Ok(Response::Empty) => Ok(()),
        Ok(resp) => Err(Rejections::db_api_err("AddSessKey", resp)),
        Err(err) => Err(Rejections::from(err)),
    }?;

    // Add the session key as content if this is a roku auth
    // TODO: make it so we don't have to do that anymore...
    let base_reply = match sess_type {
        SessType::Roku => warp::reply::html(sess_key.clone()),
        _ => warp::reply::html("".to_string()),
    };

    trace!("Session key: {}", sess_key.clone());

    // Keep a status report of this success
    params.a_s_r.mod_report(|report: &mut InnerStatusReport| {
        report.auth_success += 1;
    }).await;

    // Reply with success if we made it here
    Ok(warp::reply::with_header(
        base_reply,
        "Set-Cookie",
        format!("{}={}; Max-Age={}; SameSite=Lax",
            sess_type.get_session_cookie_name(), sess_key,
            sess_type.get_max_age().num_seconds())
    ))
}

pub async fn get_status_report(startup_time: DateTime<Utc>, params: APIParams)
    -> Result<impl Reply, Rejection>
{
    trace!("Starting get_status_report");
    params.a_s_r.mod_report(|report: &mut InnerStatusReport| {
        report.get_status_report += 1;
    }).await;

    let email_status_report = match params.email.get_status_report().await {
        Ok(report) => format!("{}", report),
        Err(err) => format!("Email report error: {}", err),
    };

    let db_status_report = match params.db.please(Action::GetStatusReport).await {
        Ok(Response::StatusReport(report)) => format!("{}", report),
        Ok(resp) => format!("Database responded wrong type: {:?}", resp),
        Err(err) => format!("Database report error: {}", err),
    };

    let api_status_report = params.a_s_r.read_report().await;

    Ok(warp::reply::html(format!(
            concat!(
                "Startup time: {}\n",
                "Version: {}\n",
                "{}\n",
                "{}\n",
                "{}",
            ),
            startup_time, helpers::VERSION,
            email_status_report, db_status_report, api_status_report
            )
        ))
}

pub async fn create_account(params: APIParams,
        form_dat: models::CreateAcctForm)
    -> Result<impl Reply, Rejection>
{
    trace!("Starting create_account");
    params.a_s_r.mod_report(|report: &mut InnerStatusReport| {
        report.create_account += 1;
    }).await;

    // Fail early if the username is invalid
    email::parse_addr(&form_dat.username)
        .map_err(|_| {Rejections::InvalidEmailAddr})?;

    // Fail early if the password isn't long enough
    if form_dat.password.len() < MIN_PASSWORD_LEN {
        return Err(Rejections::InvalidPassword.into());
    };

    // TODO: handle properly when the rand number is already in the DB
    let reg_key = gen_large_rand_str();
    debug!("Adding user with reg key ?val_code={}", reg_key);

    // Generate the password hash
    let pw_hash = match password_hash_version::hash_pw(
        &form_dat.username, &form_dat.password)
    {
        Ok(val) => Ok(val),
        Err(err) => Err(Rejections::from(err)),
    }?;

    let pw_hash_ver = password_hash_version::get_pw_ver();

    let user_id = match params.db.please(Action::AddUser {
        user: form_dat.username.clone(),
        pass_hash: pw_hash,
        pass_hash_ver: pw_hash_ver,
        reg_key: reg_key.clone(),
    }).await {
        Ok(Response::UserID(user_id)) => Ok(user_id),
        Ok(resp) => Err(Rejections::db_api_err("AddUser", resp)),
        Err(err) => Err(Rejections::from(err)),
    }?;

    params.email.please(email::Action::SendRegAcct(
        email::RegisterData {
            dest_addr: form_dat.username,
            reg_key: reg_key,
        }
    )).await;

    let first_chan_nm = "First Channel";

    let first_chan_success = match params.db.please(Action::CreateChannelList {
        user_id: user_id,
        list_name: first_chan_nm.into(),
    }).await {
        Ok(_) => true,
        Err(err) => {
            warn!("User created, error creating first channel list: {}", err);
            false
        } 
    };

    if first_chan_success {
        match params.db.please(Action::SetActiveChannel {
            user_id: user_id,
            list_name: first_chan_nm.into(),
        }).await {
            Ok(_) => {},
            Err(err) => {
                warn!("User created, error setting first channel active: {}", err)
            }
        }
    }

    params.a_s_r.mod_report(|report: &mut InnerStatusReport| {
        report.account_created += 1;
    }).await;

    Ok(StatusCode::OK)
}

pub async fn validate_account(params: APIParams, opts: models::ValidateAccountRequest)
    -> Result<impl Reply, Rejection>
{
    trace!("Starting validate_account");
    params.a_s_r.mod_report(|report: &mut InnerStatusReport| {
        report.validate_account += 1;
    }).await;

    match params.db.please(Action::ValidateAccount { val_code: opts.val_code }).await {
        Ok(Response::Bool(true)) => Ok(StatusCode::OK),
        Ok(resp) => Err(Rejections::db_api_err("ValidateAccount", resp).into()),
        Err(err) => Err(Rejections::from(err).into()),
    }
    
}

pub async fn validate_session(sess_type: SessType, params: APIParams,
        _sess_info: (String, i32))
    -> Result<impl Reply, Rejection>
{
    trace!("Beginning validate_session {:?}", sess_type);
    params.a_s_r.mod_report(|report: &mut InnerStatusReport| {
        match sess_type {
            SessType::Frontend => { report.validate_session_fe += 1; },
            SessType::Roku => { report.validate_session_ro += 1; },
            SessType::Display => { report.validate_session_di += 1; },
        }
        report.validate_session += 1;
    }).await;

    // If we can get to here, we're ok
    Ok(StatusCode::OK)
}

pub async fn retrieve_session_dat(session_id: String, params: APIParams, sess_type: SessType)
    -> Result<(String, i32), Rejection>
{
    trace!("Starting retrieve_session_dat");

    match params.db.please(Action::ValidateSessKey {
        sess_type: sess_type,
        sess_key: session_id.clone(),
    }).await {
        Ok(Response::ValidatedKey(true, user_id)) =>
            Ok((session_id, user_id)),
        Ok(Response::ValidatedKey(false, _)) =>
            Err(Rejections::InvalidSession.into()),
        Ok(resp) => Err(Rejections::db_api_err("ValidateSessKey", resp).into()),
        Err(err) => Err(Rejections::from(err).into()),
    }
}

pub async fn logout_session(sess_type: SessType, params: APIParams,
        sess_info: (String, i32))
    -> Result<impl Reply, Rejection>
{
    trace!("Beginning logout_session {:?}", sess_type);
    params.a_s_r.mod_report(|report: &mut InnerStatusReport| {
        match sess_type {
            SessType::Frontend => { report.logout_session_fe += 1; },
            SessType::Roku => { report.logout_session_ro += 1; },
            SessType::Display => { report.logout_session_di += 1; },
        }
        report.logout_session += 1;
    }).await;

    let (sess_key, _user_id) = sess_info;

    match params.db.please(Action::LogoutSessKey {
        sess_type: sess_type,
        sess_key: sess_key,
    }).await {
        Ok(Response::Empty) => Ok(StatusCode::OK),
        Ok(resp) => Err(Rejections::db_api_err("LogoutSessKey", resp).into()),
        Err(err) => Err(Rejections::from(err).into()),
    }
}

pub async fn get_channel_lists(params: APIParams, sess_info: (String, i32))
    -> Result<impl Reply, Rejection>
{
    trace!("Beginning get_channel_lists");
    params.a_s_r.mod_report(|report: &mut InnerStatusReport| {
        report.get_channel_lists += 1;
    }).await;

    let (_sess_key, user_id) = sess_info;

    match params.db.please(Action::GetChannelLists {
        user_id: user_id,
    }).await {
        Ok(Response::StringResp(val)) => Ok(warp::reply::html(val)),
        Ok(resp) => Err(Rejections::db_api_err("GetChannelLists", resp).into()),
        Err(err) => Err(Rejections::from(err).into()),
    }
}

pub async fn get_channel_list(sess_type: SessType, params: APIParams,
        sess_info: (String, i32), opts: models::GetChannelListQuery)
    -> Result<impl Reply, Rejection>
{
    trace!("Beginning get_channel_list {:?}", sess_type);
    params.a_s_r.mod_report(|report: &mut InnerStatusReport| {
        report.get_channel_list += 1;
    }).await;

    let (_sess_key, user_id) = sess_info;

    match params.db.please(Action::GetChannelList {
        user_id: user_id,
        list_name: opts.list_name,
    }).await {
        Ok(Response::StringResp(val)) => Ok(warp::reply::html(val)),
        Ok(resp) => Err(Rejections::db_api_err("GetChannelList", resp).into()),
        Err(err) => Err(Rejections::from(err).into()),
    }
}

pub async fn get_active_channel(sess_type: SessType, params: APIParams,
        sess_info: (String, i32))
    -> Result<impl Reply, Rejection>
{
    trace!("Beginning get_active_channel {:?}", sess_type);
    params.a_s_r.mod_report(|report: &mut InnerStatusReport| {
        match sess_type {
            SessType::Frontend => { report.get_active_channel_fe += 1; },
            SessType::Roku => { report.get_active_channel_ro += 1; },
            SessType::Display => { report.get_active_channel_di += 1; },
        }
        report.get_active_channel += 1;
    }).await;

    let (_sess_key, user_id) = sess_info;

    match params.db.please(Action::GetActiveChannel {
        user_id: user_id,
    }).await {
        Ok(Response::StringResp(val)) => Ok(warp::reply::html(val)),
        Ok(resp) => Err(Rejections::db_api_err("GetActiveChannel", resp).into()),
        Err(err) => Err(Rejections::from(err).into()),
    }
}

pub async fn get_channel_xml(sess_type: SessType, params: APIParams,
        sess_info: (String, i32))
    -> Result<impl Reply, Rejection>
{
    trace!("Beginning get_channel_xml {:?}", sess_type);
    params.a_s_r.mod_report(|report: &mut InnerStatusReport| {
        report.get_channel_xml_ro += 1;
    }).await;

    let (_sess_key, user_id) = sess_info;

    let channel_list = match params.db.please(Action::GetActiveChannel {
        user_id: user_id,
    }).await {
        Ok(Response::StringResp(val)) => Ok(val),
        Ok(resp) => Err(Rejections::db_api_err("GetActiveChannel", resp)),
        Err(err) => Err(Rejections::from(err)),
    }?;

    let json: serde_json::Value = match serde_json::from_str(&channel_list) {
        Ok(val) => Ok(val),
        Err(err) => Err(Rejections::ErrorInternal(format!("serde JSON Error: {}", err))),
    }?;

    let xml_str1 = helpers::build_xml(json.clone());
    Ok(warp::reply::html(xml_str1))
}

pub async fn get_active_channel_name(sess_type: SessType, params: APIParams,
        sess_info: (String, i32))
    -> Result<impl Reply, Rejection>
{
    trace!("Beginning get_active_channel_name {:?}", sess_type);
    params.a_s_r.mod_report(|report: &mut InnerStatusReport| {
        report.get_active_channel_name += 1;
    }).await;

    let (_sess_key, user_id) = sess_info;

    match params.db.please(Action::GetActiveChannelName {
        user_id: user_id,
    }).await {
        Ok(Response::StringResp(val)) => Ok(warp::reply::html(val)),
        Ok(resp) => Err(Rejections::db_api_err("GetActiveChannelName", resp).into()),
        Err(err) => Err(Rejections::from(err).into()),
    }
}

pub async fn refresh_session(sess_type: SessType, params: APIParams,
        sess_info: (String, i32)
    )
    -> Result<impl Reply, Rejection>
{
    trace!("Beginning refresh_session {:?}", sess_type);

    // Error out if they're trying to refresh a session that shouldn't be
    match sess_type {
        SessType::Roku => Ok(()),
        SessType::Display => Ok(()),
        _ => Err(Rejections::InvalidRefreshSessionType),
    }?;

    params.a_s_r.mod_report(|report: &mut InnerStatusReport| {
        match sess_type {
            SessType::Roku => { report.refresh_session_ro += 1; },
            SessType::Display => { report.refresh_session_di += 1; },
            _ => { panic!("Invalid session refresh attempt got too far: {:?}", sess_type) },
        }
        report.refresh_session += 1;
    }).await;

    let (old_sess_key, user_id) = sess_info;

    // Generate a new session key
    let new_sess_key = gen_large_rand_str();

    // Add the new to the DB, and fail if this action fails
    match params.db.please(Action::AddSessKey {
        user: db::NameOrID::ID(user_id),
        sess_type: sess_type.clone(),
        sess_key: new_sess_key.clone(),
    }).await {
        Ok(Response::Empty) => Ok(()),
        Ok(resp) => Err(Rejections::db_api_err("AddSessKey", resp)),
        Err(err) => Err(Rejections::from(err)),
    }?;

    // Remove the old session key, but don't fail if this fails...
    match params.db.please(Action::LogoutSessKey {
        sess_type: sess_type.clone(),
        sess_key: old_sess_key,
    }).await {
        Ok(Response::Empty) => (),
        Ok(resp) => error!("Invalid response from DB for LogoutSessKey: {:?}", resp),
        Err(err) => error!("Error from DB for LogoutSessKey: {:?}", err),
    };

    trace!("Refreshed session key: {}", new_sess_key.clone());

    Ok(warp::reply::with_header(
        warp::reply::html("".to_string()),
        "Set-Cookie",
        format!("{}={}; Max-Age={}; SameSite=Lax",
            sess_type.get_session_cookie_name(), new_sess_key,
            sess_type.get_max_age().num_seconds())
    ))
}

pub async fn set_channel_list(params: APIParams, sess_info: (String, i32),
    form_dat: models::SetChannelListForm)
    -> Result<impl Reply, Rejection>
{
    trace!("Beginning set_channel_list");
    params.a_s_r.mod_report(|report: &mut InnerStatusReport| {
        report.set_channel_list += 1;
    }).await;

    let (_sess_key, user_id) = sess_info;

    // TODO validate that input is json
    // TODO convert to XML now?

    match params.db.please(Action::SetChannelList {
        user_id: user_id,
        list_name: form_dat.listname,
        list_data: form_dat.listdata,
    }).await {
        Ok(Response::Empty) => Ok(StatusCode::OK),
        Ok(resp) => Err(Rejections::db_api_err("SetChannelList", resp).into()),
        Err(err) => Err(Rejections::from(err).into()),
    }
}

pub async fn rename_channel(params: APIParams, sess_info: (String, i32),
    form_dat: models::RenameChannelForm)
    -> Result<impl Reply, Rejection>
{
    trace!("Beginning rename_channel");
    params.a_s_r.mod_report(|report: &mut InnerStatusReport| {
        report.rename_channel += 1;
    }).await;

    let (_sess_key, user_id) = sess_info;

    match params.db.please(Action::RenameChannel {
        user_id: user_id,
        list_name: form_dat.listname,
        new_list_name: form_dat.newlistname,
    }).await {
        Ok(Response::Empty) => Ok(StatusCode::OK),
        Ok(resp) => Err(Rejections::db_api_err("RenameChannel", resp).into()),
        Err(err) => Err(Rejections::from(err).into()),
    }
}

pub async fn create_channel_list(params: APIParams, sess_info: (String, i32), 
    form_dat: models::CreateChannelListForm)
    -> Result<impl Reply, Rejection>
{
    trace!("Beginning create_channel_list");
    params.a_s_r.mod_report(|report: &mut InnerStatusReport| {
        report.create_channel_list += 1;
    }).await;

    let (_sess_key, user_id) = sess_info;

    match params.db.please(Action::CreateChannelList {
        user_id: user_id,
        list_name: form_dat.listname,
    }).await {
        Ok(Response::Empty) => Ok(StatusCode::OK),
        Ok(resp) => Err(Rejections::db_api_err("CreateChannelList", resp).into()),
        Err(err) => Err(Rejections::from(err).into()),
    }
}

pub async fn set_active_channel(params: APIParams, sess_info: (String, i32), 
    form_dat: models::SetActiveChannelForm)
    -> Result<impl Reply, Rejection>
{
    trace!("Beginning set_active_channel");
    params.a_s_r.mod_report(|report: &mut InnerStatusReport| {
        report.set_active_channel += 1;
    }).await;

    let (_sess_key, user_id) = sess_info;

    match params.db.please(Action::SetActiveChannel {
        user_id: user_id,
        list_name: form_dat.listname,
    }).await {
        Ok(Response::Empty) => Ok(StatusCode::OK),
        Ok(resp) => Err(Rejections::db_api_err("SetActiveChannel", resp).into()),
        Err(err) => Err(Rejections::from(err).into()),
    }
}

pub async fn delete_channel(params: APIParams, sess_info: (String, i32), 
    form_dat: models::DeleteChannelQuery)
    -> Result<impl Reply, Rejection>
{
    trace!("Beginning delete_channel");
    params.a_s_r.mod_report(|report: &mut InnerStatusReport| {
        report.delete_channel += 1;
    }).await;

    let (_sess_key, user_id) = sess_info;

    match params.db.please(Action::DeleteChannel {
        user_id: user_id,
        list_name: form_dat.listname,
    }).await {
        Ok(Response::Empty) => Ok(StatusCode::OK),
        Ok(resp) => Err(Rejections::db_api_err("DeleteChannel", resp).into()),
        Err(err) => Err(Rejections::from(err).into()),
    }
}

pub async fn validate_origin_or_referer(source: String, cors_origins: Vec<String>)
    -> Result<(), Rejection>
{
    trace!("Beginning validate_origin_or_referer");

    if cors_origins.iter().any(|origin| source.starts_with(origin)) {
        Ok(())
    } else {
        Err(reject::custom(Rejections::InvalidOriginOrReferer))
    }
}

pub async fn handle_rejection(err: Rejection)
    -> Result<impl Reply, Rejection>
{
    trace!("Beginning handle_rejection");

    let code;
    let message;

    // TODO add in more error handling?

    if err.is_not_found() {
        code = StatusCode::NOT_FOUND;
        message = "Not Found";
    } else {
        match err.find() {
            Some(Rejections::InvalidUserLookup) |
            Some(Rejections::InvalidPassword) |
            Some(Rejections::InvalidUserNonValidated) |
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
                info!("ErrorInternal: {}", content);
                code = StatusCode::INTERNAL_SERVER_ERROR;
                message = "Internal Server Error: INTERNAL";
            },
            Some(Rejections::ErrorFromDB(dberr))
            => {
                info!("ErrorFromDB: {}", dberr);
                code = StatusCode::INTERNAL_SERVER_ERROR;
                message = "Internal Server Error: DB";
            },
            other => {
                info!("Unhandled error on request: {:?}", other);
                code = StatusCode::INTERNAL_SERVER_ERROR;
                message = "Internal Server Error: OTHER";
            },
        }
    }

    Ok(warp::reply::with_status(message.to_string(), code))
}

fn gen_large_rand_str() -> String {
    // Generate a 64 character code in ascii hex
    let reg_key_p1 = rand::thread_rng().gen::<u128>();
    let reg_key_p2 = rand::thread_rng().gen::<u128>();
    format!("{:032X}{:032X}", reg_key_p1, reg_key_p2)
}

pub async fn orderly_shutdown(params: APIParams) -> () {
    info!("Beginning orderly shutdown");

    params.email.please(email::Action::Shutdown).await;
    match params.db.please(db::Action::Shutdown).await {
        Ok(Response::Shutdown) => (),
        Ok(resp) => error!("Wrong shutdown response from DB: {:?}", resp),
        Err(err) => error!("Error from DB: {:?}", err),
    };
}

#[cfg(test)]
mod tests{
    use super::*;

    /// Test out handle_rejection's properly handling DBError results
    /// Also validates ability to transform DBError type into Rejections,
    /// and Rejections into Rejection
    #[tokio::test]
    async fn try_handle_rejection_dberror() {
        let rejection: Rejection = Rejections::from(DBError::InvalidRowCount(5)).into();
        let result = handle_rejection(rejection).await.unwrap();
        let expected = Ok(
            warp::reply::with_status("Internal Server Error: DB",
                StatusCode::INTERNAL_SERVER_ERROR)
        );

        assert_eq!(
            format!("{:?}", expected.into_response()),
            format!("{:?}", result.into_response())
        );
    }

    #[test]
    fn print_error() {
        let reject1 = Rejections::from(DBError::InvalidRowCount(10));
        let reject2 = Rejections::db_api_err("ValidateAccount", Response::Bool(false));
        let reject3 = Rejections::from(DBError::from(diesel::result::Error::NotFound));

        println!("{}", reject1);
        println!("{}", reject2);
        println!("{}", reject3);
    }
}
