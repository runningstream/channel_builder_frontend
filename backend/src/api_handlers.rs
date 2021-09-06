use crate::{db, email, helpers, models, password_hash_version};
use helpers::SessType;
use db::{Action, Response};
use rand::Rng;
use warp::http::StatusCode;
use warp::reject;


#[derive(Debug)]                                                                                                                                                                               
pub enum Rejections {                                                                                                                                                                              
    // User-caused problems                                                                                                                                                                    
    InvalidUserLookup, InvalidUserNonValidated,                                                                                                                                                
    InvalidPassword, InvalidSession, InvalidEmailAddr,                                                                                                                                         
    InvalidValidationCode, InvalidOriginOrReferer,                                                                                                                                             
                                                                                                                                                                                               
    // System Problems                                                                                                                                                                         
    ErrorInternal(String)                                                                                                                                                                      
}                                                                                                                                                                                              
                                                                                                                                                                                               
impl reject::Reject for Rejections {}


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
            Ok(resp) => {
                return Err(reject::custom(Rejections::ErrorInternal(
                    format!("GetUserPassHash response: {:?}", resp)
                )));
            },
            Err(_) => {
                return Err(reject::custom(Rejections::InvalidUserLookup));
            },
        };

    if !valid_status {
        return Err(reject::custom(Rejections::InvalidUserNonValidated));
    }

    let sess_key = gen_large_rand_str();

    match db.please(Action::AddSessKey {
        user: form_dat.username.clone(),
        sess_type: sess_type.clone(),
        sess_key: sess_key.clone(),
    }).await {
        Ok(_) => {},
        Err(err) => {
            return Err(reject::custom(
                Rejections::ErrorInternal(
                    format!("AddSessKey response: {:?}",err)
                )
            ))
        },
    };

    let max_age = sess_type.get_max_age();

    //println!("Authenticated {:?}: {:?} key {}", sess_type, form_dat, sess_key);

    // Add the session key as content if this is a roku auth
    // TODO: make it so we don't have to do that anymore...
    let base_reply = match sess_type {
        SessType::Roku => warp::reply::html(sess_key.clone()),
        _ => warp::reply::html("".to_string()),
    };

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
                    max_age)
            )),
        Ok(false) =>
            Err(reject::custom(Rejections::InvalidPassword)),
        Err(err) =>
            Err(reject::custom(Rejections::ErrorInternal(
                format!("Validating password: {:?}", err)
            ))),
    }
}

pub async fn create_account(db: db::Db, email_inst: email::Email,
        form_dat: models::CreateAcctForm)
    -> Result<impl warp::Reply, warp::Rejection>
{
    // Fail early if the username is invalid
    match email::parse_addr(&form_dat.username) {
        Ok(_) => {},
        Err(_) => {
            return Err(reject::custom(Rejections::InvalidEmailAddr));
        }
    };

    // TODO: handle properly when the rand number is already in the DB
    let reg_key = gen_large_rand_str();
    println!("Adding user with reg key ?val_code={}", reg_key);

    // Generate the password hash
    let pw_hash = match password_hash_version::hash_pw(
        &form_dat.username, &form_dat.password)
    {
        Ok(val) => val,
        Err(err) => {
            return Err(reject::custom(Rejections::ErrorInternal(
                format!("Error hashing password: {}", err)
            )));
        },
    };

    let pw_hash_ver = password_hash_version::get_pw_ver();

    let user_id = match db.please(Action::AddUser {
        user: form_dat.username.clone(),
        pass_hash: pw_hash,
        pass_hash_ver: pw_hash_ver,
        reg_key: reg_key.clone(),
    }).await {
        Ok(Response::UserID(user_id)) => user_id,
        Ok(resp) => {
            return Err(reject::custom(Rejections::ErrorInternal(
                format!("AddUser response: {:?}", resp)
            )));
        },
        Err(err) => {
            return Err(reject::custom(Rejections::ErrorInternal(
                format!("AddUser error: {:?}", err)
            )));
        },
    };

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
            println!("User created, error creating first channel list: {}", err);
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
                println!("User created, error setting first channel active: {}", err)
            }
        }
    }

    Ok(StatusCode::OK)
}

pub async fn validate_account(db: db::Db,
    opts: models::ValidateAccountRequest)
    -> Result<impl warp::Reply, warp::Rejection>
{
    match db.please(Action::ValidateAccount { val_code: opts.val_code }).await {
        Ok(Response::Bool(true)) => Ok(StatusCode::OK),
        Ok(resp) =>
            Err(reject::custom(Rejections::ErrorInternal(
                format!("ValidateAccount Response {:?}", resp)
            ))),
        Err(db::DBError::InvalidValidationCode) =>
            Err(reject::custom(Rejections::InvalidValidationCode)),
        Err(err) =>
            Err(reject::custom(Rejections::ErrorInternal(
                format!("ValidateAccount Error {:?}", err)
            ))),
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
        Err(err) => 
            Err(reject::custom(Rejections::ErrorInternal(
                format!("LogoutSessKey Error: {:?}", err)
            ))),
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
        Ok(resp) =>
            Err(reject::custom(Rejections::ErrorInternal(
                format!("GetChannelLists Response: {:?}", resp)
            ))),
        Err(err) => 
            Err(reject::custom(Rejections::ErrorInternal(
                format!("GetChannelLists Error: {:?}", err)
            ))),
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
        Ok(resp) =>
            Err(reject::custom(Rejections::ErrorInternal(
                format!("GetChannelList Response: {:?}", resp)
            ))),
        Err(err) =>
            Err(reject::custom(Rejections::ErrorInternal(
                format!("GetChannelList Error: {:?}", err)
            ))),
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
        Ok(resp) => {
            return Err(reject::custom(Rejections::ErrorInternal(
                format!("GetActiveChannel Response: {:?}", resp)
            )));
        },
        Err(err) => {
            return Err(reject::custom(Rejections::ErrorInternal(
                format!("GetActiveChannel Error: {:?}", err)
            )));
        },
    };

    let json: serde_json::Value = match serde_json::from_str(&channel_list) {
        Ok(val) => val,
        Err(err) => {
            return Err(reject::custom(Rejections::ErrorInternal(
                format!("serde JSON Error: {:?}", err)
            )));
        },
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
        Err(err) =>
            Err(reject::custom(Rejections::ErrorInternal(
                format!("SetChannelList Error: {:?}", err)
            ))),
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
        Err(err) =>
            Err(reject::custom(Rejections::ErrorInternal(
                format!("CreateChannelList Error: {:?}", err)
            ))),
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
        Err(err) =>
            Err(reject::custom(Rejections::ErrorInternal(
                format!("SetActiveChannel Error: {:?}", err)
            ))),
    }
}

fn gen_large_rand_str() -> String {
    // Generate a 64 character code in ascii hex
    let reg_key_p1 = rand::thread_rng().gen::<u128>();
    let reg_key_p2 = rand::thread_rng().gen::<u128>();
    format!("{:032X}{:032X}", reg_key_p1, reg_key_p2)
}
