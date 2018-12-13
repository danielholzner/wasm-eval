#[macro_use] extern crate lazy_static;
extern crate futures;
extern crate js_sys;
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

type Integer = i32;

#[wasm_bindgen]
extern {
    #[wasm_bindgen(js_namespace = console)]
    fn log(msg: &str);
}

macro_rules! log {
    ($($t:tt)*) => (log(&format!($($t)*)))
}

/// A future that becomes ready after a tick of the micro task queue.
pub struct NextTick {
    inner: JsFuture,
}

impl NextTick {
    /// Construct a new `NextTick` future.
    pub fn new() -> NextTick {
        // Create a resolved promise that will run its callbacks on the next
        // tick of the micro task queue.
        let promise = js_sys::Promise::resolve(&JsValue::NULL);
        // Convert the promise into a `JsFuture`.
        let inner = JsFuture::from(promise);
        NextTick { inner }
    }
}

impl Future for NextTick {
    type Item = ();
    type Error = ();

    fn poll(&mut self) -> Poll<(), ()> {
        // Polling a `NextTick` just forwards to polling if the inner promise is
        // ready.
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

/// Export a function to JavaScript that does some work in the next tick of the
/// micro task queue!
#[wasm_bindgen]
pub fn schedule_eval(expr: String, vals: Vec<Integer>) -> js_sys::Promise {
    let future = NextTick::new()
        // Do some work...
        .and_then(move |_| {
           Ok(eval(&insert_values(&expr, &vals).unwrap()).unwrap())
        })
        // And then convert the `Item` and `Error` into `JsValue`.
        .map(|result| {
            JsValue::from_serde(&result).unwrap()
        })
        .map_err(|error| {
            log!("{:?}", error);
            let js_error = js_sys::Error::new(&format!("uh oh! {:?}", error));
            JsValue::from(js_error)
        });

    // Convert the `Future<Item = JsValue, Error = JsValue>` into a JavaScript
    // `Promise`!
    future_to_promise(future)
}

/// Evals the given expression
/// If the expression can't be evaled Null is returned
#[wasm_bindgen]
pub fn eval_syn(expr: &str, vals: &[Integer]) -> JsValue {
    let mut res = JsValue::NULL;
    if let Ok(expr) = insert_values(expr, vals) {
        if let Ok(evaled) = eval(&expr) {
            if let Ok(final_res) = JsValue::from_serde(&evaled) {
                res = final_res;
            }
        }
    }
    res
}

fn insert_values(expr: &str, vals: &[Integer]) -> Result<String, Box<error::Error>> {
    lazy_static! {
        static ref RE: Regex = Regex::new(r"(\{})").unwrap();
    }
    log!("{:#?}", vals);
    let mut idx = 0;
    let result = RE.replace_all(expr, |caps: &Captures| {
        println!("{:?}", caps);
        let replacement = vals[idx].to_string();
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
        let nums = [1, 2, 3];
        let expr = String::from("{} + {} * {}");
        assert_eq!(insert_values(&expr, &nums).unwrap(), "1 + 2 * 3");
    }

    #[test]
    #[should_panic]
    fn should_panic_when_value_size_does_not_match() {
        let nums = [1, 2];
        let expr = String::from("{} + {} * {}");
        insert_values(&expr, &nums).unwrap();
    }

    #[test]
    fn should_eval_string() {
        let nums = [1, 2, 3];
        let expr = String::from("{} + {} * {}");
        assert_eq!(eval(&insert_values(&expr, &nums).unwrap()), Ok(to_value(7)));
    }
}
