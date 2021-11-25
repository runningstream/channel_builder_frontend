use std::{error, fmt};
use crate::{db, email, helpers, models, password_hash_version};
use helpers::SessType;
use db::{Action, Response, DBError};
use password_hash_version::PWHashError;
use rand::Rng;
use warp::http::StatusCode;
use warp::{reject, Reply, Rejection};


#[derive(Debug)]
pub enum Rejections {
    // User-caused problems
    InvalidUserLookup, InvalidUserNonValidated,
    InvalidPassword, InvalidSession, InvalidEmailAddr,
    InvalidValidationCode, InvalidOriginOrReferer,
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

pub async fn authenticate_ro(db: db::Db, form_dat: models::AuthForm)
    -> Result<impl Reply, Rejection>
{
    trace!("Starting authenticate_ro");
    authenticate_gen(SessType::Roku, db, form_dat).await
}

pub async fn authenticate_fe(db: db::Db, form_dat: models::AuthForm)
    -> Result<impl Reply, Rejection>
{
    trace!("Starting authenticate_fe");
    authenticate_gen(SessType::Frontend, db, form_dat).await
}

async fn authenticate_gen(sess_type: SessType, db: db::Db, form_dat: models::AuthForm)
    -> Result<impl Reply, Rejection>
{
    trace!("Starting authenticate_gen");
    let (pass_hash, hash_ver, valid_status) = 
        match db.please(Action::GetUserPassHash {
            user: form_dat.username.clone(),
        }).await {
            Ok(Response::UserPassHash(pass_hash, hash_ver, valid_status)) => 
                Ok((pass_hash, hash_ver, valid_status)),
            Ok(resp) => Err(Rejections::db_api_err("GetUserPassHash", resp)),
            Err(err) => Err(Rejections::from(err)),
        }?;


    if !valid_status {
        return Err(Rejections::InvalidUserNonValidated.into());
    }

    let sess_key = gen_large_rand_str();

    match db.please(Action::AddSessKey {
        user: form_dat.username.clone(),
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

    // TODO - why do I authenticate the password after making the session key?
    // that doesn't seem to make sense

    match password_hash_version::validate_pw_ver(&form_dat.username,
        &form_dat.password, &pass_hash, hash_ver)
    {
        Ok(true) =>
            Ok(warp::reply::with_header(
                base_reply,
                "Set-Cookie", 
                format!("{}={}; Max-Age={}; SameSite=Lax", 
                    helpers::SESSION_COOKIE_NAME, sess_key,
                    sess_type.get_max_age().num_seconds())
            )),
        Ok(false) => Err(Rejections::InvalidPassword.into()),
        Err(err) => Err(Rejections::from(err).into())
    }
}

pub async fn create_account(db: db::Db, email_inst: email::Email,
        form_dat: models::CreateAcctForm)
    -> Result<impl Reply, Rejection>
{
    // Fail early if the username is invalid
    email::parse_addr(&form_dat.username)
        .map_err(|_| {Rejections::InvalidEmailAddr})?;

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

    let user_id = match db.please(Action::AddUser {
        user: form_dat.username.clone(),
        pass_hash: pw_hash,
        pass_hash_ver: pw_hash_ver,
        reg_key: reg_key.clone(),
    }).await {
        Ok(Response::UserID(user_id)) => Ok(user_id),
        Ok(resp) => Err(Rejections::db_api_err("AddUser", resp)),
        Err(err) => Err(Rejections::from(err)),
    }?;

    email_inst.please(email::Action::SendRegAcct(
        email::RegisterData {
            dest_addr: form_dat.username,
            reg_key: reg_key,
        }
    )).await;

    let first_chan_nm = "First Channel";

    let first_chan_success = match db.please(Action::CreateChannelList {
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
        match db.please(Action::SetActiveChannel {
            user_id: user_id,
            list_name: first_chan_nm.into(),
        }).await {
            Ok(_) => {},
            Err(err) => {
                warn!("User created, error setting first channel active: {}", err)
            }
        }
    }

    Ok(StatusCode::OK)
}

pub async fn validate_account(db: db::Db,
    opts: models::ValidateAccountRequest)
    -> Result<impl Reply, Rejection>
{
    match db.please(Action::ValidateAccount { val_code: opts.val_code }).await {
        Ok(Response::Bool(true)) => Ok(StatusCode::OK),
        Ok(resp) => Err(Rejections::db_api_err("ValidateAccount", resp).into()),
        Err(err) => Err(Rejections::from(err).into()),
    }
    
}

pub async fn validate_session_fe(db: db::Db, sess_info: (String, i32))
    -> Result<impl Reply, Rejection>
{
    validate_session(SessType::Frontend, db, sess_info).await
}

pub async fn validate_session_ro(db: db::Db, sess_info: (String, i32))
    -> Result<impl Reply, Rejection>
{
    validate_session(SessType::Roku, db, sess_info).await
}

async fn validate_session(_sess_type: SessType, _db: db::Db,
    _sess_info: (String, i32))
    -> Result<impl Reply, Rejection>
{
    // If we can get to here, we're ok
    Ok(StatusCode::OK)
}

pub async fn retrieve_session_dat(session_id: String, db: db::Db, sess_type: SessType)
    -> Result<(String, i32), Rejection>
{
    trace!("Starting retrieve_session_dat");
    match db.please(Action::ValidateSessKey {
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

pub async fn logout_session_fe(db: db::Db, sess_info: (String, i32))
    -> Result<impl Reply, Rejection>
{
    let (sess_key, _user_id) = sess_info;

    match db.please(Action::LogoutSessKey {
        sess_type: SessType::Frontend,
        sess_key: sess_key,
    }).await {
        Ok(Response::Empty) => Ok(StatusCode::OK),
        Ok(resp) => Err(Rejections::db_api_err("LogoutSessKey", resp).into()),
        Err(err) => Err(Rejections::from(err).into()),
    }
}

pub async fn get_channel_lists(db: db::Db, sess_info: (String, i32))
    -> Result<impl Reply, Rejection>
{
    let (_sess_key, user_id) = sess_info;

    match db.please(Action::GetChannelLists {
        user_id: user_id,
    }).await {
        Ok(Response::StringResp(val)) => Ok(warp::reply::html(val)),
        Ok(resp) => Err(Rejections::db_api_err("GetChannelLists", resp).into()),
        Err(err) => Err(Rejections::from(err).into()),
    }
}

pub async fn get_channel_list(db: db::Db, sess_info: (String, i32), 
    opts: models::GetChannelListQuery)
    -> Result<impl Reply, Rejection>
{
    let (_sess_key, user_id) = sess_info;

    match db.please(Action::GetChannelList {
        user_id: user_id,
        list_name: opts.list_name,
    }).await {
        Ok(Response::StringResp(val)) => Ok(warp::reply::html(val)),
        Ok(resp) => Err(Rejections::db_api_err("GetChannelList", resp).into()),
        Err(err) => Err(Rejections::from(err).into()),
    }
}

pub async fn get_channel_xml_ro(db: db::Db, sess_info: (String, i32)) 
    -> Result<impl Reply, Rejection>
{
    let (_sess_key, user_id) = sess_info;

    let channel_list = match db.please(Action::GetActiveChannel {
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

pub async fn set_channel_list(db: db::Db, sess_info: (String, i32), 
    form_dat: models::SetChannelListForm)
    -> Result<impl Reply, Rejection>
{
    let (_sess_key, user_id) = sess_info;

    // TODO validate that input is json
    // TODO convert to XML now?

    match db.please(Action::SetChannelList {
        user_id: user_id,
        list_name: form_dat.listname,
        list_data: form_dat.listdata,
    }).await {
        Ok(Response::Empty) => Ok(StatusCode::OK),
        Ok(resp) => Err(Rejections::db_api_err("SetChannelList", resp).into()),
        Err(err) => Err(Rejections::from(err).into()),
    }
}

pub async fn create_channel_list(db: db::Db, sess_info: (String, i32), 
    form_dat: models::CreateChannelListForm)
    -> Result<impl Reply, Rejection>
{
    let (_sess_key, user_id) = sess_info;

    match db.please(Action::CreateChannelList {
        user_id: user_id,
        list_name: form_dat.listname,
    }).await {
        Ok(Response::Empty) => Ok(StatusCode::OK),
        Ok(resp) => Err(Rejections::db_api_err("CreateChannelList", resp).into()),
        Err(err) => Err(Rejections::from(err).into()),
    }
}

pub async fn set_active_channel(db: db::Db, sess_info: (String, i32), 
    form_dat: models::SetActiveChannelForm)
    -> Result<impl Reply, Rejection>
{
    let (_sess_key, user_id) = sess_info;

    match db.please(Action::SetActiveChannel {
        user_id: user_id,
        list_name: form_dat.listname,
    }).await {
        Ok(Response::Empty) => Ok(StatusCode::OK),
        Ok(resp) => Err(Rejections::db_api_err("SetActiveChannel", resp).into()),
        Err(err) => Err(Rejections::from(err).into()),
    }
}

pub async fn validate_origin_or_referer(source: String, cors_origin: String)
    -> Result<(), Rejection>
{
    if source.starts_with(&cors_origin) {
        Ok(())
    } else {
        Err(reject::custom(Rejections::InvalidOriginOrReferer))
    }
}

pub async fn handle_rejection(err: Rejection)
    -> Result<impl Reply, Rejection>
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

#[cfg(test)]
mod tests{
    use super::*;

    /// Test out handle_rejection's properly handling DBError results
    /// Also validates ability to transform DBError type into Rejections,
    /// and Rejections into Rejection
    #[tokio::test]
    async fn try_handle_rejection_dberror() {
        let rejection: Rejection = Rejections::from(DBError::InvalidUsername).into();
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
        let reject1 = Rejections::from(DBError::InvalidUsername);
        let reject2 = Rejections::db_api_err("ValidateAccount", Response::Bool(false));

        println!("{}", reject1);
        println!("{}", reject2);
    }
}
