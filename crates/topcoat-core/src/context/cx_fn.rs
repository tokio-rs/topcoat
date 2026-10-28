//! Async functions of a [`Cx`] that can be stored and called later.

use std::{fmt, pin::Pin};

use crate::context::Cx;

/// The future returned by [`BoxCxFn::call`] and its numbered variants.
pub type BoxCxFuture<'cx, R> = Pin<Box<dyn Future<Output = R> + Send + 'cx>>;

/// Declares the traits and the boxed type for async functions that take a
/// [`Cx`] and the listed extra arguments.
macro_rules! cx_fn {
    (
        $future:ident,
        $(#[$bound_meta:meta])*
        $bound:ident,
        $(#[$boxed_meta:meta])*
        $boxed:ident,
        ($($arg:ident: $ty:ident),*)
    ) => {
        /// Names the future that a function returns for one lifetime of its
        /// [`Cx`], so [`
        #[doc = stringify!($bound)]
        /// `] can require it to be `Send`.
        #[doc(hidden)]
        pub trait $future<'cx, $($ty,)* R>:
            Fn(&'cx Cx $(, $ty)*) -> <Self as $future<'cx, $($ty,)* R>>::Fut
        {
            type Fut: Future<Output = R> + Send + 'cx;
        }

        impl<'cx, F, Fut, $($ty,)* R> $future<'cx, $($ty,)* R> for F
        where
            F: Fn(&'cx Cx $(, $ty)*) -> Fut,
            Fut: Future<Output = R> + Send + 'cx,
        {
            type Fut = Fut;
        }

        $(#[$bound_meta])*
        pub trait $bound<$($ty,)* R>:
            AsyncFn(&Cx $(, $ty)*) -> R
            + for<'cx> $future<'cx, $($ty,)* R>
            + Send
            + Sync
            + 'static
        {
        }

        impl<F, $($ty,)* R> $bound<$($ty,)* R> for F where
            F: AsyncFn(&Cx $(, $ty)*) -> R
                + for<'cx> $future<'cx, $($ty,)* R>
                + Send
                + Sync
                + 'static
        {
        }

        $(#[$boxed_meta])*
        pub struct $boxed<$($ty,)* R>(
            Box<dyn for<'cx> Fn(&'cx Cx $(, $ty)*) -> BoxCxFuture<'cx, R> + Send + Sync>,
        );

        impl<$($ty: 'static,)* R: 'static> $boxed<$($ty,)* R> {
            /// Stores an async closure or an `async fn`.
            pub fn new(f: impl $bound<$($ty,)* R>) -> Self {
                Self(Box::new(move |cx $(, $arg)*| Box::pin(f(cx $(, $arg)*))))
            }

            /// Stores a function that returns a boxed future.
            ///
            /// Use this for a function that captures state, which
            /// [`new`](Self::new) does not accept.
            pub fn boxed(
                f: impl for<'cx> Fn(&'cx Cx $(, $ty)*) -> BoxCxFuture<'cx, R>
                    + Send
                    + Sync
                    + 'static,
            ) -> Self {
                Self(Box::new(f))
            }

            /// Calls the function with `cx`, returning its future.
            pub fn call<'cx>(&self, cx: &'cx Cx $(, $arg: $ty)*) -> BoxCxFuture<'cx, R> {
                (self.0)(cx $(, $arg)*)
            }
        }

        impl<$($ty,)* R> fmt::Debug for $boxed<$($ty,)* R> {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.debug_struct(stringify!($boxed)).finish_non_exhaustive()
            }
        }
    };
}

cx_fn!(
    CxFnFuture,
    /// An async function of a [`Cx`] that returns `R`.
    ///
    /// Async closures and `async fn`s implement this trait when their future
    /// is `Send`. Accept one with `impl AsyncCxFn<R>` and store it as a
    /// [`BoxCxFn`]. `R` can be any type, such as a `Result` for a function
    /// that can fail.
    ///
    /// A plain closure that returns an `async` block does not implement this
    /// trait, and neither does an async closure that captures state. Use
    /// [`BoxCxFn::boxed`] for those.
    ///
    /// A future that holds a value that is not `Send` across an `.await` is
    /// rejected:
    ///
    /// ```compile_fail
    /// # use std::rc::Rc;
    /// # use topcoat_core::context::{BoxCxFn, Cx};
    /// async fn wait() {}
    ///
    /// let f: BoxCxFn<usize> = BoxCxFn::new(async |_cx: &Cx| {
    ///     let count = Rc::new(1);
    ///     wait().await;
    ///     *count
    /// });
    /// ```
    AsyncCxFn,
    /// A stored async function of a [`Cx`] that returns `R`.
    ///
    /// Create one from an async closure or an `async fn` with
    /// [`new`](Self::new), and run it with [`call`](Self::call). A type can
    /// hold one to compute a value for each request:
    ///
    /// ```
    /// use topcoat_core::{
    ///     context::{AsyncCxFn, BoxCxFn, Cx},
    ///     error::Result,
    /// };
    ///
    /// /// A page title that is computed for each request.
    /// pub struct Title(BoxCxFn<Result<String>>);
    ///
    /// impl Title {
    ///     pub fn dynamic(f: impl AsyncCxFn<Result<String>>) -> Self {
    ///         Self(BoxCxFn::new(f))
    ///     }
    ///
    ///     pub async fn get(&self, cx: &Cx) -> Result<String> {
    ///         self.0.call(cx).await
    ///     }
    /// }
    ///
    /// async fn post_title(cx: &Cx) -> Result<String> {
    ///     Ok(String::from("Hello"))
    /// }
    ///
    /// let inline = Title::dynamic(async |cx| Ok(String::from("Hello")));
    /// let named = Title::dynamic(post_title);
    /// ```
    ///
    /// Each call boxes the returned future. Creating one from an async
    /// closure or an `async fn` that captures nothing does not allocate.
    /// Calls are not memoized.
    BoxCxFn,
    ()
);

cx_fn!(
    CxFnFuture1,
    /// An async function of a [`Cx`] and one more argument that returns
    /// `R`.
    ///
    /// Works like [`AsyncCxFn`]. Store it as a [`BoxCxFn1`].
    AsyncCxFn1,
    /// A stored async function of a [`Cx`] and one more argument that
    /// returns `R`.
    ///
    /// Works like [`BoxCxFn`].
    BoxCxFn1,
    (a: A)
);

cx_fn!(
    CxFnFuture2,
    /// An async function of a [`Cx`] and two more arguments that returns
    /// `R`.
    ///
    /// Works like [`AsyncCxFn`]. Store it as a [`BoxCxFn2`].
    AsyncCxFn2,
    /// A stored async function of a [`Cx`] and two more arguments that
    /// returns `R`.
    ///
    /// Works like [`BoxCxFn`].
    BoxCxFn2,
    (a: A, b: B)
);

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        context::context,
        error::{Error, Result},
    };

    struct Name(&'static str);

    fn named(name: &'static str) -> Cx {
        Cx::new().with(Name(name))
    }

    async fn greet(cx: &Cx) -> Result<String> {
        Ok(format!("hello {}", context::<Name>(cx).0))
    }

    #[tokio::test]
    async fn calls_an_async_closure_with_the_given_context() {
        let f = BoxCxFn::new(async |cx| context::<Name>(cx).0.len());
        assert_eq!(f.call(&named("ada")).await, 3);
        assert_eq!(f.call(&named("grace")).await, 5);
    }

    #[tokio::test]
    async fn calls_an_async_fn() {
        let f = BoxCxFn::new(greet);
        assert_eq!(f.call(&named("ada")).await.unwrap(), "hello ada");
    }

    #[tokio::test]
    async fn propagates_errors_from_a_closure_using_the_question_mark() {
        let f: BoxCxFn<Result<String>> = BoxCxFn::new(async |cx| {
            let greeting = greet(cx).await?;
            Err(Error::msg(greeting))
        });
        assert!(f.call(&named("ada")).await.is_err());
    }

    #[tokio::test]
    async fn passes_extra_arguments() {
        let one = BoxCxFn1::new(async |cx, times: usize| context::<Name>(cx).0.repeat(times));
        assert_eq!(one.call(&named("ab"), 2).await, "abab");

        let two = BoxCxFn2::new(async |cx, prefix: &'static str, suffix: char| {
            format!("{prefix}{}{suffix}", context::<Name>(cx).0)
        });
        assert_eq!(two.call(&named("ada"), "<", '>').await, "<ada>");
    }

    #[tokio::test]
    async fn boxed_accepts_a_function_that_captures_state() {
        let suffix = String::from("!");
        let f = BoxCxFn::boxed(move |cx| {
            let suffix = suffix.clone();
            Box::pin(async move { format!("{}{suffix}", context::<Name>(cx).0) })
        });
        assert_eq!(f.call(&named("ada")).await, "ada!");
    }

    #[test]
    fn stored_functions_are_send_and_sync() {
        fn assert_send_sync<T: Send + Sync>() {}
        assert_send_sync::<BoxCxFn<Result<String>>>();
        assert_send_sync::<BoxCxFn2<u64, String, Option<String>>>();
    }
}
