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
use web_sys::console::{log_1, time_with_label, time_end_with_label};

type Integer = i32;

macro_rules! log {
    ( $( $t:tt )* ) => {
        log_1(&format!( $( $t )* ).into());
    }
}

pub struct Timer<'a> {
    name: &'a str,
}

impl<'a> Timer<'a> {
    pub fn new(name: &'a str) -> Timer<'a> {
        time_with_label(name);
        Timer { name }
    }
}

impl<'a> Drop for Timer<'a> {
    fn drop(&mut self) {
        time_end_with_label(self.name);
    }
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
pub fn schedule_eval(expr: String, vals: JsValue) -> js_sys::Promise {
    let future = NextTick::new()
        // Do some work...
        .and_then(move |_| {
           let elements: Vec<String> = vals.into_serde().unwrap();
           Ok(eval(&insert_values(&expr, &elements).unwrap()).unwrap())
        })
        // And then convert the `Item` and `Error` into `JsValue`.
        .map(|result| {
            JsValue::from_serde(&result).unwrap()
        })
        .map_err(|error| {
            let js_error = js_sys::Error::new(&format!("uh oh! {:?}", error));
            JsValue::from(js_error)
        });

    // Convert the `Future<Item = JsValue, Error = JsValue>` into a JavaScript
    // `Promise`!
    future_to_promise(future)
}

/// Evals the given expression
/// If the expression can't be evaled UNDEFINED is returned
#[wasm_bindgen]
pub fn eval_syn(expr: &str, vals: JsValue) -> JsValue {
    let _timer = Timer::new("eval_test");
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
    let result = RE.replace_all(expr, |caps: &Captures| {
        println!("{:?}", caps);
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
