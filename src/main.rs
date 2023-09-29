use cfg_if::cfg_if;
// boilerplate to run in different modes
cfg_if! {
    if #[cfg(feature = "ssr")] {
    use leptos::*;
    use axum::{
        routing::{post, get},
        extract::{State, Path, RawQuery, FromRef},
        http::{Request, HeaderMap},
        response::{IntoResponse, Response},
        Router,
    };
    use axum::body::Body as AxumBody;
    use crate::todo::*;
    use leptos_todo_app_axum_seaorm::*;
    use crate::fallback::file_and_error_handler;
    use leptos_axum::{generate_route_list, LeptosRoutes, handle_server_fns_with_context};
    use migration::{Migrator, MigratorTrait};
    use sea_orm::{Database, DatabaseConnection};

    #[derive(FromRef, Debug, Clone)]
    pub struct AppState{
        pub leptos_options: LeptosOptions,
        pub db: DatabaseConnection,
    }

    async fn server_fn_handler(State(app_state): State<AppState>, path: Path<String>, headers: HeaderMap, raw_query: RawQuery,
    request: Request<AxumBody>) -> impl IntoResponse {
        handle_server_fns_with_context(path, headers, raw_query, move || {
            provide_context(app_state.db.clone());
        }, request).await
    }

    async fn leptos_routes_handler(State(app_state): State<AppState>, req: Request<AxumBody>) -> Response{
            let handler = leptos_axum::render_app_to_stream_with_context(app_state.leptos_options.clone(),
            move || {
                provide_context(app_state.db.clone());
            },
            || view! { <TodoApp/> }
        );
        handler(req).await.into_response()
    }

    #[tokio::main]
    async fn main() {
        simple_logger::init_with_level(log::Level::Error).expect("couldn't initialize logging");

        let db = Database::connect("sqlite:Todos.db?mode=rwc").await.expect("couldn't connect to DB");
        Migrator::up(&db, None).await.expect("couldn't run database migrations");

        // Setting this to None means we'll be using cargo-leptos and its env vars
        let conf = get_configuration(None).await.unwrap();
        let leptos_options = conf.leptos_options;
        let addr = leptos_options.site_addr;
        let routes = generate_route_list(|| view! { <TodoApp/> });

        let app_state = AppState{
            leptos_options,
            db: db.clone(),
        };

        // build our application with a route
        let app = Router::new()
        .route("/api/*fn_name", post(server_fn_handler))
        .leptos_routes_with_handler(routes, get(leptos_routes_handler) )
        .fallback(file_and_error_handler)
        .with_state(app_state);

        // run our app with hyper
        // `axum::Server` is a re-export of `hyper::Server`
        logging::log!("listening on http://{}", &addr);
        axum::Server::bind(&addr)
            .serve(app.into_make_service())
            .await
            .unwrap();
        }
    }

    // client-only stuff for Trunk
    else {
        pub fn main() {
            // This example cannot be built as a trunk standalone CSR-only app.
            // Only the server may directly connect to the database.
        }
    }
}
