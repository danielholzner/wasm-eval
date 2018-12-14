#[macro_use] extern crate lazy_static;
extern crate futures;
extern crate js_sys;
extern crate web_sys;
extern crate wasm_bindgen;
extern crate wasm_bindgen_futures;
extern crate regex;
extern crate eval;

use futures::{Async, Future, Poll};
use wasm_bindgen::prelude::*;
use wasm_bindgen_futures::{JsFuture, future_to_promise};
use regex::{Regex, Captures};
use std::error;
use eval::eval;


pub struct NextTick {
    inner: JsFuture,
}

impl NextTick {
    pub fn new() -> NextTick {
        let promise = js_sys::Promise::resolve(&JsValue::NULL);
        let inner = JsFuture::from(promise);
        NextTick { inner }
    }
}

impl Future for NextTick {
    type Item = ();
    type Error = ();

    fn poll(&mut self) -> Poll<(), ()> {
        match self.inner.poll() {
            Ok(Async::Ready(_)) => Ok(Async::Ready(())),
            Ok(Async::NotReady) => Ok(Async::NotReady),
            Err(_) => unreachable!(
                "We only create NextTick with a resolved inner promise, never \
                 a rejected one, so we can't get an error here"
            ),
        }
    }
}

/// Returns Javascript Promise
#[wasm_bindgen]
pub fn schedule_eval(expr: String, vals: JsValue) -> js_sys::Promise {
    let future = NextTick::new()
        .and_then(move |_| {
           let elements: Vec<String> = vals.into_serde().unwrap();
           Ok(eval(&insert_values(&expr, &elements).unwrap()).unwrap())
        })
        .map(|result| {
            JsValue::from_serde(&result).unwrap()
        })
        .map_err(|error| {
            let js_error = js_sys::Error::new(&format!("uh oh! {:?}", error));
            JsValue::from(js_error)
        });

    future_to_promise(future)
}

/// Evals the given expression
/// If the expression can't be evaled UNDEFINED is returned
#[wasm_bindgen]
pub fn eval_syn(expr: &str, vals: JsValue) -> JsValue {
    let mut res = JsValue::UNDEFINED;

    if let Ok(elements) = vals.into_serde::<Vec<String>>() {
        if let Ok(expr) = insert_values(expr, &elements) {
            if let Ok(evaled) = eval(&expr) {
                if let Ok(final_res) = JsValue::from_serde(&evaled) {
                    res = final_res;
                }
            }
        }
    }
    res
}

fn insert_values(expr: &str, vals: &[String]) -> Result<String, Box<error::Error>> {
    lazy_static! {
        static ref RE: Regex = Regex::new(r"(\{})").unwrap();
    }
    let mut idx = 0;
    let result = RE.replace_all(expr, |_caps: &Captures| {
        let replacement = &vals[idx];
        idx = idx + 1;
        replacement
    }).to_string();

    Ok(result)
}

#[cfg(test)]
mod tests {
    use regex::{ Regex, Captures };
    use eval::{eval, to_value};
    use super::*;

    #[test]
    fn should_relace_regex() {
        let re = Regex::new(r"([^,\s]+),\s+(\S+)").unwrap();
        let result = re.replace("Springsteen, Bruce", |caps: &Captures| {
            format!("{} {}", &caps[2], &caps[1])
        });
        assert_eq!(result, "Bruce Springsteen");
    }

    #[test]
    fn should_insert_numbers() {
        let nums = [String::from("1"),String::from("2"), String::from("3")];
        let expr = String::from("{} + {} * {}");
        assert_eq!(insert_values(&expr, &nums).unwrap(), "1 + 2 * 3");
    }

    #[test]
    #[should_panic]
    fn should_panic_when_value_size_does_not_match() {
        let nums = [String::from("1"), String::from("2")];
        let expr = String::from("{} + {} * {}");
        insert_values(&expr, &nums).unwrap();
    }

    #[test]
    fn should_eval_string() {
        let nums = [String::from("1"),String::from("2"), String::from("3")];
        let expr = String::from("{} + {} * {}");
        assert_eq!(eval(&insert_values(&expr, &nums).unwrap()), Ok(to_value(7)));
    }
}
