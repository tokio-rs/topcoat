#![allow(dead_code)]

// This marker is the compiler-facing contract. A Topcoat macro can emit it
// around an ordinary Rust closure without lowering the expression to JS.
fn __topcoat_split_root<R>(_: &str, expression: impl FnOnce() -> R) -> R {
    expression()
}

mod models {
    pub struct Product {
        pub price: f64,
        pub quantity: u32,
        pub discount: f64,
    }

    impl Product {
        pub fn total(&self) -> f64 {
            discounted(self.price, self.discount) * self.quantity as f64
        }

        pub fn load_from_database() -> Self {
            let _server_only = std::net::TcpListener::bind("127.0.0.1:0");
            Self { price: 4.5, quantity: 2, discount: 0.1 }
        }

        pub fn unused_client_method(&self) -> f64 {
            self.price * 1000.0
        }
    }

    fn discounted(price: f64, discount: f64) -> f64 {
        price * (1.0 - discount)
    }
}

// A small stand-in for the signal API isolates extraction from transport.
// The driver does not recognize this type specially.
pub struct Signal<T> {
    pub value: T,
}

impl<T> Signal<T> {
    pub fn get(&self) -> &T {
        &self.value
    }
}

fn page() {
    use models::Product;
    let product_signal = Signal { value: Product::load_from_database() };
    __topcoat_split_root("page", || product_signal.get().price);
    __topcoat_split_root("page", || product_signal.get().total());
}

fn shard() {
    let product_signal = Signal { value: models::Product::load_from_database() };
    __topcoat_split_root("shard", || product_signal.get().quantity);
}

fn main() {
    page();
    shard();
}
