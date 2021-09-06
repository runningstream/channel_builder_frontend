/// Helper capabilities and constants that do not belong in any specific module
/// and are useful across modules.

use std::result::Result;
use std::fmt::Debug;
use std::time::Duration;
use std::thread::sleep;
use serde_json::Value;
use thiserror::Error;


/// The name of the session cookie in the frontend
pub static SESSION_COOKIE_NAME: &str = "session";

/// An enum that differentiates frontend session types
#[derive(Debug, Clone)]
pub enum SessType { Frontend, Roku }

impl SessType {
    pub fn get_max_age(&self) -> u32 {
        match *self {
            SessType::Frontend => 60 * 24 * 5, // 5 days
            SessType::Roku => 60 * 24 * 365,   // 365 days
        }
    }
}


#[derive(Error, Debug, PartialEq)]
pub enum RetryErr {
    #[error("retries were exhausted")]
    RetriesExhausted,
}

/// Recursively retry a function call count times, sleeping between each.
pub fn retry_on_err
    <FUNC: Fn() -> Result<RETTYPE, ERRTYPE>, 
        RETTYPE, ERRTYPE: Debug>
    ( count: u32, sleep_len: Duration, func: FUNC)
    -> Result<RETTYPE, RetryErr>
{
    if count <= 0 {
        println!("Retries exhausted");
        return Err(RetryErr::RetriesExhausted);
    }

    func().or_else( |err| {
        println!("Error with {} retries remaining: {:?}", count - 1, err);
        sleep(sleep_len);
        retry_on_err(count - 1, sleep_len, func)
    })
}


/// Build an XML string from serde JSON
pub fn build_xml(json: Value) -> String {
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn retry_fail() {
        fn always_fail() -> Result<(), ()> { Err(()) }

        let result = retry_on_err(1, Duration::new(0,100), always_fail);
        let expected = Err(RetryErr::RetriesExhausted);
        assert_eq!(expected, result);
    }

    #[test]
    fn retry_noretries() {
        fn succeed() -> Result<u32, ()> { Ok(4) }

        let result = retry_on_err(0, Duration::new(0,100), succeed);
        let expected = Err(RetryErr::RetriesExhausted);
        assert_eq!(expected, result);
    }

    #[test]
    fn retry_succeed() {
        fn succeed() -> Result<u32, ()> { Ok(4) }

        let result = retry_on_err(1, Duration::new(0,100), succeed);
        let expected = Ok(4);
        assert_eq!(expected, result);
    }
}
