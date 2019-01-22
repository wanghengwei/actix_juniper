# actix_juniper
create juniper app for actix_web

# Usage
```rust
#[macro_use] extern crate juniper;

use juniper::{FieldResult, EmptyMutation};
use actix_juniper::graphql_app;

struct DBContext;

impl juniper::Context for DBContext {}

struct Query;

graphql_object!(Query: DBContext |&self| {
    field version(&executor) -> FieldResult<String> {
        Ok("1.0.0".to_string())
    }
});

fn main() {
    std::env::set_var("RUST_LOG", "info");
    env_logger::init();
    let sys = actix::System::new("");

    actix_web::server::new(|| {
        graphql_app(|| Query, || EmptyMutation::new(), || DBContext)
    }).bind("127.0.0.1:5000").unwrap().start();

    sys.run();
}
```
