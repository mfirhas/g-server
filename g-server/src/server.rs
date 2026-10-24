#[derive(Clone, Debug)]
pub struct Server<C> {
    pub name: &'static str,
    pub ip_address: &'static str,
    pub port: u16,
    pub router: crate::axum::Router<()>,
    pub app_context: C,
}
