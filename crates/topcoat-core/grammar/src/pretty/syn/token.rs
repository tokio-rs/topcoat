use quote::ToTokens;
use syn::spanned::Spanned;

use crate::pretty::{PrettyPrint, Printer, TextMode};

impl PrettyPrint for syn::Ident {
    fn pretty_print(&self, printer: &mut Printer<'_>) {
        printer.move_cursor(self.span().start());
        printer.scan_text(self.to_string().into(), TextMode::Always);
        printer.move_cursor(self.span().end());
    }
}

impl PrettyPrint for syn::Lifetime {
    fn pretty_print(&self, printer: &mut Printer<'_>) {
        printer.move_cursor(self.span().start());
        printer.scan_text(self.to_string().into(), TextMode::Always);
        printer.move_cursor(self.span().end());
    }
}

impl PrettyPrint for syn::Lit {
    fn pretty_print(&self, printer: &mut Printer<'_>) {
        printer.move_cursor(self.span().start());
        printer.scan_text(self.to_token_stream().to_string().into(), TextMode::Always);
        printer.move_cursor(self.span().end());
    }
}

impl PrettyPrint for syn::Index {
    fn pretty_print(&self, printer: &mut Printer<'_>) {
        printer.move_cursor(self.span.start());
        printer.scan_text(self.index.to_string().into(), TextMode::Always);
        printer.move_cursor(self.span.end());
    }
}

impl PrettyPrint for syn::Member {
    fn pretty_print(&self, printer: &mut Printer<'_>) {
        match self {
            Self::Named(ident) => ident.pretty_print(printer),
            Self::Unnamed(index) => index.pretty_print(printer),
        }
    }
}

impl PrettyPrint for syn::BinOp {
    fn pretty_print(&self, printer: &mut Printer<'_>) {
        let text = match self {
            Self::Add(_) => "+",
            Self::Sub(_) => "-",
            Self::Mul(_) => "*",
            Self::Div(_) => "/",
            Self::Rem(_) => "%",
            Self::And(_) => "&&",
            Self::Or(_) => "||",
            Self::BitXor(_) => "^",
            Self::BitAnd(_) => "&",
            Self::BitOr(_) => "|",
            Self::Shl(_) => "<<",
            Self::Shr(_) => ">>",
            Self::Eq(_) => "==",
            Self::Lt(_) => "<",
            Self::Le(_) => "<=",
            Self::Ne(_) => "!=",
            Self::Ge(_) => ">=",
            Self::Gt(_) => ">",
            Self::AddAssign(_) => "+=",
            Self::SubAssign(_) => "-=",
            Self::MulAssign(_) => "*=",
            Self::DivAssign(_) => "/=",
            Self::RemAssign(_) => "%=",
            Self::BitXorAssign(_) => "^=",
            Self::BitAndAssign(_) => "&=",
            Self::BitOrAssign(_) => "|=",
            Self::ShlAssign(_) => "<<=",
            Self::ShrAssign(_) => ">>=",
            _ => {
                super::common::verbatim(printer, self);
                return;
            }
        };
        printer.move_cursor(self.span().start());
        printer.scan_text(text.into(), TextMode::Always);
        printer.move_cursor(self.span().end());
    }
}

impl PrettyPrint for syn::UnOp {
    fn pretty_print(&self, printer: &mut Printer<'_>) {
        let text = match self {
            Self::Deref(_) => "*",
            Self::Not(_) => "!",
            Self::Neg(_) => "-",
            _ => {
                super::common::verbatim(printer, self);
                return;
            }
        };
        printer.move_cursor(self.span().start());
        printer.scan_text(text.into(), TextMode::Always);
        printer.move_cursor(self.span().end());
    }
}

macro_rules! impl_token {
    ($token:tt) => {
        impl PrettyPrint for syn::Token![$token] {
            fn pretty_print(&self, printer: &mut Printer<'_>) {
                printer.move_cursor(self.span().start());
                printer.scan_text(stringify!($token).into(), TextMode::Always);
                printer.move_cursor(self.span().end());
            }
        }
    };
}

impl_token!(#);
impl_token!(!);
impl_token!(=);
impl_token!(.);
impl_token!(,);
impl_token!(:);
impl_token!(;);
impl_token!(*);
impl_token!(/);
impl_token!(%);
impl_token!(+);
impl_token!(-);
impl_token!(>);
impl_token!(<);
impl_token!($);
impl_token!(=>);
impl_token!(<=);
impl_token!(@);
impl_token!(&);
impl_token!(|);
impl_token!(?);
impl_token!(_);
impl_token!(::);
impl_token!(->);
impl_token!(..);
impl_token!(..=);
impl_token!(...);

impl_token!(as);
impl_token!(async);
impl_token!(auto);
impl_token!(await);
impl_token!(break);
impl_token!(const);
impl_token!(continue);
impl_token!(crate);
impl_token!(default);
impl_token!(dyn);
impl_token!(else);
impl_token!(enum);
impl_token!(extern);
impl_token!(fn);
impl_token!(for);
impl_token!(if);
impl_token!(impl);
impl_token!(in);
impl_token!(let);
impl_token!(loop);
impl_token!(match);
impl_token!(mod);
impl_token!(move);
impl_token!(mut);
impl_token!(pub);
impl_token!(ref);
impl_token!(return);
impl_token!(self);
impl_token!(Self);
impl_token!(static);
impl_token!(struct);
impl_token!(super);
impl_token!(trait);
impl_token!(try);
impl_token!(type);
impl_token!(union);
impl_token!(unsafe);
impl_token!(use);
impl_token!(where);
impl_token!(while);
impl_token!(yield);

macro_rules! impl_has_token {
    ($($for:tt)*) => {
        impl PrettyPrint for $($for)* {
            fn pretty_print(&self, printer: &mut Printer<'_>) {
                self.token().pretty_print(printer);
            }
        }
    };
}

impl_has_token!(syn::LitBool);
impl_has_token!(syn::LitInt);
impl_has_token!(syn::LitFloat);
impl_has_token!(syn::LitStr);
