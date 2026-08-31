use pin_project_lite::pin_project;

pin_project! {
    pub struct JoinView<U> {
        #[pin]
        units: U,
    }
}
