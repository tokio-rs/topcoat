#[macro_export]
macro_rules! stylesheet {
    () => {
        ::topcoat::asset::asset!(concat!(env!("OUT_DIR"), "/tailwind.css"))
    };
}
