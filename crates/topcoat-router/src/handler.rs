use crate::Params;

pub trait Handler: std::fmt::Debug {
    async fn handle(&self, params: &Params);
}

type HandlerFn = fn(&Params)
