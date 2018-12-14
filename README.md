## Description
This crate can evaluate expressions like `("{} + {} * {}", [1,2,3])`. It was developed to eval JavaScript-code of the client in a safe context.

The crate is based on the Rust [eval](https://docs.rs/eval)-crate and its supported 
operators are:

 `! != "" '' () [] . , > < >= <= == + - * / % && || n..m.`

 See for supported operators.
## Requirements
Your web-server must support the `application/wasm` Mime-Type.

Example for Jetty:
```java
//add new mime-type mapping
ServletContextHandler context = new ServletContextHandler(ServletContextHandler.SESSIONS);
MimeTypes mt = context.getMimeTypes();
mt.addMimeMapping("wasm", "application/wasm");
```

## Usage
Import the wasm-module into your app (e.g. import in ember via [ember-auto-import](https://github.com/ef4/ember-auto-import)):
```javascript
(async () => {
    // Importing wasm module
    const { eval_syn, schedule_eval } = await import('wasm-eval')
    const values = [1, 2, 3];
    const expr = "{} + {} * {}";

    //eval_syn returns the value directly
    console.log(eval_syn(expr, values)); // 1 + 2 * 3 = 7

    /**
     * returns a promise (non-blocking) 
     * Useful for long expressions
     */
    schedule_eval(expr, values).then(res => {
        if(res) {
            console.log(res); // 7
        } else { // null is returned on parse error
            console.log("Could not parse expression");
        }
    });
})();
```