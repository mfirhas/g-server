#[derive(Clone, Copy, Debug)]
pub struct Server {
    pub name: &'static str,
    pub ip_address: &'static str,
    pub port: u16,
}
