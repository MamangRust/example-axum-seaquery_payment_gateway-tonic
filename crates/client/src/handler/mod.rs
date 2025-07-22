mod auth;
mod saldo;
mod topup;
mod transfer;
mod user;
mod withdraw;

use crate::state::AppState;
use anyhow::Result;
use std::sync::Arc;
use tokio::net::TcpListener;
use utoipa::{Modify, OpenApi, openapi::security::SecurityScheme};
use utoipa_axum::router::OpenApiRouter;
use utoipa_swagger_ui::SwaggerUi;

pub use self::auth::auth_routes;
pub use self::saldo::saldos_routes;
pub use self::topup::topup_routes;
pub use self::transfer::transfers_routes;
pub use self::user::users_routes;
pub use self::withdraw::withdraw_routes;

#[derive(OpenApi)]
#[openapi(
    paths(
        auth::login_user_handler,
        auth::get_me_handler,
        auth::register_user_handler,
        saldo::get_saldos,
        saldo::get_saldo,
        saldo::get_saldo_users,
        saldo::get_saldo_user,
        saldo::create_saldo,
        saldo::update_saldo,
        saldo::delete_saldo,
        topup::get_topups,
        topup::get_topup,
        topup::get_topup_users,
        topup::get_topup_user,
        topup::create_topup,
        topup::update_topup,
        topup::delete_topup,
        transfer::get_transfers,
        transfer::get_transfer,
        transfer::get_transfer_users,
        transfer::get_transfer_user,
        transfer::create_transfer,
        transfer::update_transfer,
        transfer::delete_transfer,
        user::get_users,
        user::get_user,
        user::create_user,
        user::update_user,
        user::delete_user,
        withdraw::get_withdraws,
        withdraw::get_withdraw,
        withdraw::get_withdraw_users,
        withdraw::get_withdraw_user,
        withdraw::create_withdraw,
        withdraw::update_withdraw,
        withdraw::delete_withdraw
    ),
    modifiers(&SecurityAddon),
    tags(
        (name = "Auth", description = "Authentication endpoints"),
        (name = "User", description = "User management endpoints"),
        (name = "Saldo", description = "Balance management endpoints"),
        (name = "Topup", description = "Top up endpoints"),
        (name = "Transfer", description = "Transfer endpoints"),
        (name = "Withdraw", description = "Withdrawal endpoints")
    )
)]
struct ApiDoc;

struct SecurityAddon;

impl Modify for SecurityAddon {
    fn modify(&self, openapi: &mut utoipa::openapi::OpenApi) {
        let components = openapi.components.as_mut().unwrap();

        components.add_security_scheme(
            "bearer_auth",
            SecurityScheme::Http(utoipa::openapi::security::Http::new(
                utoipa::openapi::security::HttpAuthScheme::Bearer,
            )),
        );
    }
}

pub struct AppRouter;

impl AppRouter {
    pub async fn serve(port: u16, app_state: AppState) -> Result<()> {
        let shared_state = Arc::new(app_state);

        let (router, api) = OpenApiRouter::with_openapi(ApiDoc::openapi())
            .merge(auth_routes(shared_state.clone()))
            .merge(users_routes(shared_state.clone()))
            .merge(saldos_routes(shared_state.clone()))
            .merge(topup_routes(shared_state.clone()))
            .merge(transfers_routes(shared_state.clone()))
            .merge(withdraw_routes(shared_state.clone()))
            .split_for_parts();

        let app =
            router.merge(SwaggerUi::new("/swagger-ui").url("/api-docs/openapi.json", api.clone()));

        let addr = format!("0.0.0.0:{port}");
        let listener = TcpListener::bind(&addr).await?;

        println!("Server running on http://{}", listener.local_addr()?);
        println!("API Documentation available at:");
        println!("- Swagger UI: http://localhost:{port}/swagger-ui");

        axum::serve(listener, app).await?;
        Ok(())
    }
}
