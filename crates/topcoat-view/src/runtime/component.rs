use topcoat_core::error::Result;

pub trait Component {
    fn render(self) -> impl Future<Output = Result> + Send;
}
