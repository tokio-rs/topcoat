use topcoat_core::context::Cx;

pub struct SessionToken {}

pub fn start_session(cx: &Cx) -> SessionToken {
    SessionToken {}
}

pub fn stop_session(cx: &Cx) {}

pub fn session(cx: &Cx) -> SessionToken {
    SessionToken {}
}
