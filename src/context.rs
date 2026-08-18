#[derive(Clone)]
pub struct AppContext<C>
where
    C: Clone + Send + Sync + 'static,
{
    context: C,
}

impl<C> AppContext<C>
where
    C: Clone + Send + Sync + 'static,
{
    pub fn new(context: C) -> Self {
        Self { context }
    }

    pub fn ctx(&self) -> C {
        self.context.clone()
    }
}
