#[macro_use] extern crate serde_derive;

use futures::prelude::*;
use actix::prelude::*;
use actix_web::{App, HttpRequest, HttpResponse, State, Json, FutureResponse, AsyncResponder};
use juniper::{RootNode, GraphQLType};
use juniper::http::GraphQLRequest;
use juniper::graphiql::graphiql_source;
use failure::Error;

struct GraphQLExecutor<Q, M, CTX> where
    Q: GraphQLType,
    M: GraphQLType
{
    schema: RootNode<'static, Q, M>,
    ctx: CTX,
}

impl<Q, M, CTX> Actor for GraphQLExecutor<Q, M, CTX> where
    Q: GraphQLType + 'static,
    M: GraphQLType + 'static,
    CTX: 'static
{
    type Context = SyncContext<Self>;
}

#[derive(Serialize, Deserialize)]
struct GraphQLData(GraphQLRequest);

impl Message for GraphQLData {
    type Result = Result<String, Error>;
}

impl<Q, M, CTX> Handler<GraphQLData> for GraphQLExecutor<Q, M, CTX> where
    Q: GraphQLType<Context = CTX> + 'static,
    M: GraphQLType<Context = CTX> + 'static,
    CTX: 'static
{
    type Result = Result<String, Error>;

    fn handle(&mut self, msg: GraphQLData, _: &mut Self::Context) -> Self::Result {
        let res = msg.0.execute(&self.schema, &self.ctx);
        serde_json::to_string(&res).map_err(|e| e.into())
    }
}

pub struct AppState<Q: GraphQLType + 'static, M: GraphQLType + 'static, CTX: 'static>(Addr<GraphQLExecutor<Q, M, CTX>>);

fn graphiql_handler<Q, M, C>(_req: &HttpRequest<AppState<Q, M, C>>) -> Result<HttpResponse, Error> where
    Q: GraphQLType + 'static,
    M: GraphQLType + 'static
{
    let html = graphiql_source("graphql");
    Ok(HttpResponse::Ok()
        .content_type("text/html; charset=utf-8")
        .body(html))
}

fn graphql_handler<Q, M, CTX>((st, data): (State<AppState<Q, M, CTX>>, Json<GraphQLData>)) -> FutureResponse<HttpResponse> where
    Q: GraphQLType<Context = CTX> + 'static,
    M: GraphQLType<Context = CTX> + 'static
{
    st.0.send(data.0)
        .from_err()
        .and_then(|res| match res {
            Ok(user) => Ok(HttpResponse::Ok()
                .content_type("application/json")
                .body(user)),
            Err(_) => Ok(HttpResponse::InternalServerError().into()),
        })
        .responder()
}

/// create an actix_web::App
/// # Example
/// ```
/// struct Query;
/// 
/// graphql_object!(Query: DBContext |&self| {
///     field version(&executor) -> FieldResult<String> {
///     Ok("1.0.0".to_string())
///    }
/// });
/// 
/// fn main() {
///     let sys = actix::System::new("");
///     actix_web::server::new(|| {
///         graphql_app(|| Query, || EmptyMutation::new(), || DBContext)
///     }).bind("127.0.0.1:5000").unwrap().start();
///     sys.run();
/// }
/// ```
pub fn graphql_app<Q, QueryFactoryT, M, MutationFactoryT, CTX, CF>(query_factory: QueryFactoryT, mutation_factory: MutationFactoryT, ctx_factory: CF) -> App<AppState<Q, M, CTX>> where
    Q: GraphQLType<TypeInfo = (), Context = CTX> + 'static,
    M: GraphQLType<TypeInfo = (), Context = CTX> + 'static,
    QueryFactoryT: Fn() -> Q + Send + Sync + 'static,
    MutationFactoryT: Fn() -> M + Send + Sync + 'static,
    CF: Fn() -> CTX + Send + Sync + 'static
{
    let addr = SyncArbiter::start(4, move || {
        GraphQLExecutor {
            schema: RootNode::new(query_factory(), mutation_factory()),
            ctx: ctx_factory(),
        }
    });
    App::with_state(AppState(addr))
        .resource("/graphql", |r| r.with(graphql_handler))
        .resource("/graphiql", |r| r.f(graphiql_handler))
}