use syn::spanned::Spanned;

use super::{common, path::qpath};
use crate::pretty::{BreakMode, Delim, PrettyPrint, Printer, TextMode};

impl PrettyPrint for syn::Expr {
    fn pretty_print(&self, printer: &mut Printer<'_>) {
        match self {
            Self::Array(expr) => expr.pretty_print(printer),
            Self::Assign(expr) => expr.pretty_print(printer),
            Self::Async(expr) => expr.pretty_print(printer),
            Self::Await(_) | Self::Field(_) | Self::MethodCall(_) | Self::Try(_) => {
                chain(printer, self);
            }
            Self::Binary(expr) => expr.pretty_print(printer),
            Self::Block(expr) => expr.pretty_print(printer),
            Self::Break(expr) => expr.pretty_print(printer),
            Self::Call(expr) => expr.pretty_print(printer),
            Self::Cast(expr) => expr.pretty_print(printer),
            Self::Closure(expr) => expr.pretty_print(printer),
            Self::Const(expr) => expr.pretty_print(printer),
            Self::Continue(expr) => expr.pretty_print(printer),
            Self::ForLoop(expr) => expr.pretty_print(printer),
            Self::Group(expr) => expr.expr.pretty_print(printer),
            Self::If(expr) => expr.pretty_print(printer),
            Self::Index(expr) => expr.pretty_print(printer),
            Self::Infer(expr) => expr.underscore_token.pretty_print(printer),
            Self::Let(expr) => expr.pretty_print(printer),
            Self::Lit(expr) => expr.pretty_print(printer),
            Self::Loop(expr) => expr.pretty_print(printer),
            Self::Macro(expr) => expr.mac.pretty_print(printer),
            Self::Match(expr) => expr.pretty_print(printer),
            Self::Paren(expr) => expr.pretty_print(printer),
            Self::Path(expr) => expr.pretty_print(printer),
            Self::Range(expr) => expr.pretty_print(printer),
            Self::Reference(expr) => expr.pretty_print(printer),
            Self::Repeat(expr) => expr.pretty_print(printer),
            Self::Return(expr) => expr.pretty_print(printer),
            Self::Struct(expr) => expr.pretty_print(printer),
            Self::TryBlock(expr) => expr.pretty_print(printer),
            Self::Tuple(expr) => expr.pretty_print(printer),
            Self::Unary(expr) => expr.pretty_print(printer),
            Self::Unsafe(expr) => expr.pretty_print(printer),
            Self::Verbatim(tokens) => {
                common::verbatim_span(printer, tokens.span(), || tokens.to_string());
            }
            Self::While(expr) => expr.pretty_print(printer),
            Self::Yield(expr) => expr.pretty_print(printer),
            _ => common::verbatim(printer, self),
        }
    }
}

/// Whether an expression starts and ends with its own braced block, so it
/// attaches to what precedes it with a plain space and, as a match arm body,
/// needs no trailing comma.
pub(super) fn is_block_like(expr: &syn::Expr) -> bool {
    matches!(
        expr,
        syn::Expr::Async(_)
            | syn::Expr::Block(_)
            | syn::Expr::Const(_)
            | syn::Expr::ForLoop(_)
            | syn::Expr::If(_)
            | syn::Expr::Loop(_)
            | syn::Expr::Match(_)
            | syn::Expr::TryBlock(_)
            | syn::Expr::Unsafe(_)
            | syn::Expr::While(_)
    )
}

/// One link of a `.`-chain: a method call, a field access, an `.await`, or a
/// trailing `?`.
enum Link<'a> {
    Method(&'a syn::ExprMethodCall),
    Field(&'a syn::ExprField),
    Await(&'a syn::ExprAwait),
    Try(&'a syn::ExprTry),
}

/// Prints a `.`-chain. A chain with at least two method calls breaks before
/// every `.` when it exceeds the margin; anything shorter stays attached to its
/// receiver.
fn chain(printer: &mut Printer<'_>, expr: &syn::Expr) {
    let mut links = Vec::new();
    let mut base = expr;
    loop {
        match base {
            syn::Expr::MethodCall(inner) => {
                links.push(Link::Method(inner));
                base = &inner.receiver;
            }
            syn::Expr::Field(inner) => {
                links.push(Link::Field(inner));
                base = &inner.base;
            }
            syn::Expr::Await(inner) => {
                links.push(Link::Await(inner));
                base = &inner.base;
            }
            syn::Expr::Try(inner) => {
                links.push(Link::Try(inner));
                base = &inner.expr;
            }
            _ => break,
        }
    }
    links.reverse();

    let method_calls = links
        .iter()
        .filter(|link| matches!(link, Link::Method(_)))
        .count();
    let breakable = method_calls >= 2;

    base.pretty_print(printer);
    if breakable {
        printer.scan_begin(BreakMode::Consistent);
        printer.scan_indent(1);
    }
    for link in &links {
        match link {
            Link::Method(inner) => {
                if breakable {
                    printer.scan_break();
                }
                inner.dot_token.pretty_print(printer);
                inner.method.pretty_print(printer);
                inner.turbofish.pretty_print(printer);
                inner
                    .paren_token
                    .pretty_print(printer, Some(BreakMode::Consistent), |printer| {
                        inner.args.pretty_print(printer);
                    });
            }
            Link::Field(inner) => {
                if breakable {
                    printer.scan_break();
                }
                inner.dot_token.pretty_print(printer);
                inner.member.pretty_print(printer);
            }
            Link::Await(inner) => {
                if breakable {
                    printer.scan_break();
                }
                inner.dot_token.pretty_print(printer);
                inner.await_token.pretty_print(printer);
            }
            Link::Try(inner) => {
                inner.question_token.pretty_print(printer);
            }
        }
    }
    if breakable {
        printer.scan_indent(-1);
        printer.scan_end();
    }
}

/// The binding strength of a binary operator, used to lay out a chain of
/// equal-strength operators as one flat sequence of break points.
fn precedence(op: &syn::BinOp) -> u8 {
    match op {
        syn::BinOp::Mul(_) | syn::BinOp::Div(_) | syn::BinOp::Rem(_) => 11,
        syn::BinOp::Add(_) | syn::BinOp::Sub(_) => 10,
        syn::BinOp::Shl(_) | syn::BinOp::Shr(_) => 9,
        syn::BinOp::BitAnd(_) => 8,
        syn::BinOp::BitXor(_) => 7,
        syn::BinOp::BitOr(_) => 6,
        syn::BinOp::Eq(_)
        | syn::BinOp::Lt(_)
        | syn::BinOp::Le(_)
        | syn::BinOp::Ne(_)
        | syn::BinOp::Ge(_)
        | syn::BinOp::Gt(_) => 5,
        syn::BinOp::And(_) => 4,
        syn::BinOp::Or(_) => 3,
        _ => 0,
    }
}

/// Prints the left operand of a binary chain, folding operands of the same
/// binding strength into the enclosing group so `a + b + c` breaks as one flat
/// list rather than a staircase.
fn binary_operand(printer: &mut Printer<'_>, expr: &syn::Expr, strength: u8) {
    if let syn::Expr::Binary(inner) = expr
        && precedence(&inner.op) == strength
        && strength > 0
    {
        binary_operand(printer, &inner.left, strength);
        binary_operator_and_right(printer, inner);
    } else {
        expr.pretty_print(printer);
    }
}

/// Prints a break point, the operator, and the right operand of a binary
/// expression: ` + rhs` on one line, `\n    + rhs` when broken.
fn binary_operator_and_right(printer: &mut Printer<'_>, expr: &syn::ExprBinary) {
    printer.scan_same_line_trivia();
    printer.scan_break();
    " ".pretty_print(printer);
    expr.op.pretty_print(printer);
    " ".pretty_print(printer);
    printer.scan_trivia(true, true);
    expr.right.pretty_print(printer);
}

impl PrettyPrint for syn::ExprBinary {
    fn pretty_print(&self, printer: &mut Printer<'_>) {
        self.attrs.pretty_print(printer);
        printer.scan_begin(BreakMode::Consistent);
        printer.scan_indent(1);
        binary_operand(printer, &self.left, precedence(&self.op));
        binary_operator_and_right(printer, self);
        printer.scan_indent(-1);
        printer.scan_end();
    }
}

impl PrettyPrint for syn::ExprUnary {
    fn pretty_print(&self, printer: &mut Printer<'_>) {
        self.attrs.pretty_print(printer);
        self.op.pretty_print(printer);
        self.expr.pretty_print(printer);
    }
}

impl PrettyPrint for syn::ExprReference {
    fn pretty_print(&self, printer: &mut Printer<'_>) {
        self.attrs.pretty_print(printer);
        self.and_token.pretty_print(printer);
        if let Some(mutability) = &self.mutability {
            mutability.pretty_print(printer);
            " ".pretty_print(printer);
        }
        self.expr.pretty_print(printer);
    }
}

impl PrettyPrint for syn::ExprCast {
    fn pretty_print(&self, printer: &mut Printer<'_>) {
        self.attrs.pretty_print(printer);
        self.expr.pretty_print(printer);
        " ".pretty_print(printer);
        self.as_token.pretty_print(printer);
        " ".pretty_print(printer);
        self.ty.pretty_print(printer);
    }
}

impl PrettyPrint for syn::ExprAssign {
    fn pretty_print(&self, printer: &mut Printer<'_>) {
        self.attrs.pretty_print(printer);
        self.left.pretty_print(printer);
        " ".pretty_print(printer);
        self.eq_token.pretty_print(printer);
        " ".pretty_print(printer);
        self.right.pretty_print(printer);
    }
}

impl PrettyPrint for syn::ExprLet {
    fn pretty_print(&self, printer: &mut Printer<'_>) {
        self.attrs.pretty_print(printer);
        self.let_token.pretty_print(printer);
        " ".pretty_print(printer);
        self.pat.pretty_print(printer);
        " ".pretty_print(printer);
        self.eq_token.pretty_print(printer);
        " ".pretty_print(printer);
        self.expr.pretty_print(printer);
    }
}

impl PrettyPrint for syn::ExprLit {
    fn pretty_print(&self, printer: &mut Printer<'_>) {
        self.attrs.pretty_print(printer);
        self.lit.pretty_print(printer);
    }
}

impl PrettyPrint for syn::ExprPath {
    fn pretty_print(&self, printer: &mut Printer<'_>) {
        self.attrs.pretty_print(printer);
        qpath(printer, self.qself.as_ref(), &self.path);
    }
}

impl PrettyPrint for syn::ExprCall {
    fn pretty_print(&self, printer: &mut Printer<'_>) {
        self.attrs.pretty_print(printer);
        self.func.pretty_print(printer);
        self.paren_token
            .pretty_print(printer, Some(BreakMode::Consistent), |printer| {
                self.args.pretty_print(printer);
            });
    }
}

impl PrettyPrint for syn::ExprIndex {
    fn pretty_print(&self, printer: &mut Printer<'_>) {
        self.attrs.pretty_print(printer);
        self.expr.pretty_print(printer);
        self.bracket_token
            .pretty_print(printer, Some(BreakMode::Inconsistent), |printer| {
                self.index.pretty_print(printer);
            });
    }
}

impl PrettyPrint for syn::ExprArray {
    fn pretty_print(&self, printer: &mut Printer<'_>) {
        self.attrs.pretty_print(printer);
        self.bracket_token
            .pretty_print(printer, Some(BreakMode::Consistent), |printer| {
                self.elems.pretty_print(printer);
            });
    }
}

impl PrettyPrint for syn::ExprRepeat {
    fn pretty_print(&self, printer: &mut Printer<'_>) {
        self.attrs.pretty_print(printer);
        self.bracket_token
            .pretty_print(printer, Some(BreakMode::Inconsistent), |printer| {
                self.expr.pretty_print(printer);
                self.semi_token.pretty_print(printer);
                " ".pretty_print(printer);
                self.len.pretty_print(printer);
            });
    }
}

impl PrettyPrint for syn::ExprTuple {
    fn pretty_print(&self, printer: &mut Printer<'_>) {
        self.attrs.pretty_print(printer);
        self.paren_token
            .pretty_print(printer, Some(BreakMode::Consistent), |printer| {
                if self.elems.len() == 1 {
                    self.elems[0].pretty_print(printer);
                    printer.scan_text(",".into(), TextMode::Always);
                    printer.advance_cursor(",");
                } else {
                    self.elems.pretty_print(printer);
                }
            });
    }
}

impl PrettyPrint for syn::ExprParen {
    fn pretty_print(&self, printer: &mut Printer<'_>) {
        self.attrs.pretty_print(printer);
        self.paren_token
            .pretty_print(printer, Some(BreakMode::Inconsistent), |printer| {
                self.expr.pretty_print(printer);
            });
    }
}

impl PrettyPrint for syn::ExprRange {
    fn pretty_print(&self, printer: &mut Printer<'_>) {
        self.attrs.pretty_print(printer);
        self.start.pretty_print(printer);
        self.limits.pretty_print(printer);
        self.end.pretty_print(printer);
    }
}

impl PrettyPrint for syn::RangeLimits {
    fn pretty_print(&self, printer: &mut Printer<'_>) {
        match self {
            Self::HalfOpen(dots) => dots.pretty_print(printer),
            Self::Closed(dots) => dots.pretty_print(printer),
        }
    }
}

/// The `..rest` tail of a struct literal, printed after the fields without a
/// trailing comma.
struct StructRest<'a>(&'a syn::ExprStruct);

impl PrettyPrint for StructRest<'_> {
    fn pretty_print(&self, printer: &mut Printer<'_>) {
        self.0.dot2_token.pretty_print(printer);
        self.0.rest.pretty_print(printer);
    }
}

impl PrettyPrint for syn::ExprStruct {
    fn pretty_print(&self, printer: &mut Printer<'_>) {
        self.attrs.pretty_print(printer);
        qpath(printer, self.qself.as_ref(), &self.path);
        " ".pretty_print(printer);

        let rest = (self.dot2_token.is_some() || self.rest.is_some()).then_some(StructRest(self));
        if self.fields.is_empty()
            && rest.is_none()
            && !printer.has_comment_before(self.brace_token.span.close().start())
        {
            common::empty_braces(printer, &self.brace_token);
            return;
        }
        self.brace_token
            .pretty_print(printer, Some(BreakMode::Consistent), |printer| {
                common::comma_separated(printer, &self.fields, rest.as_ref().map(|rest| rest as _));
            });
    }
}

impl PrettyPrint for syn::FieldValue {
    fn pretty_print(&self, printer: &mut Printer<'_>) {
        self.attrs.pretty_print(printer);
        self.member.pretty_print(printer);
        if let Some(colon_token) = &self.colon_token {
            colon_token.pretty_print(printer);
            " ".pretty_print(printer);
            self.expr.pretty_print(printer);
        }
    }
}

impl PrettyPrint for syn::ExprBlock {
    fn pretty_print(&self, printer: &mut Printer<'_>) {
        self.attrs.pretty_print(printer);
        self.label.pretty_print(printer);
        self.block.pretty_print(printer);
    }
}

impl PrettyPrint for syn::Label {
    fn pretty_print(&self, printer: &mut Printer<'_>) {
        self.name.pretty_print(printer);
        self.colon_token.pretty_print(printer);
        " ".pretty_print(printer);
    }
}

/// The single expression of a block that holds exactly one expression
/// statement without a trailing semicolon and nothing else.
fn single_expr(block: &syn::Block) -> Option<&syn::Expr> {
    if let [syn::Stmt::Expr(expr, None)] = block.stmts.as_slice() {
        Some(expr)
    } else {
        None
    }
}

/// Prints `{ expr }` whose break points join the caller's group, so the block
/// stays on one line when the group fits and breaks open with it otherwise.
fn inline_block(printer: &mut Printer<'_>, block: &syn::Block) {
    block.brace_token.pretty_print(printer, None, |printer| {
        block.stmts.pretty_print(printer);
    });
}

/// Prints a `unsafe`/`async`/`const`/`try` block body. A body of exactly one
/// expression without comments stays on one line when it fits.
fn keyword_block(printer: &mut Printer<'_>, block: &syn::Block) {
    if single_expr(block).is_some()
        && !printer.has_comment_before(block.brace_token.span.close().start())
    {
        printer.scan_begin(BreakMode::Consistent);
        inline_block(printer, block);
        printer.scan_end();
    } else {
        block.pretty_print(printer);
    }
}

/// Whether every branch of an `if`/`else` chain is a single expression, so the
/// whole chain can print on one line when it fits. An `if` without an `else`
/// always breaks.
fn is_inline_if_chain(expr_if: &syn::ExprIf) -> bool {
    if !expr_if.attrs.is_empty() || single_expr(&expr_if.then_branch).is_none() {
        return false;
    }
    match &expr_if.else_branch {
        None => false,
        Some((_, else_branch)) => match &**else_branch {
            syn::Expr::Block(block) => {
                block.attrs.is_empty()
                    && block.label.is_none()
                    && single_expr(&block.block).is_some()
            }
            syn::Expr::If(nested) => is_inline_if_chain(nested),
            _ => false,
        },
    }
}

/// Prints an inlineable `if`/`else` chain inside the caller's group, with
/// every block flattened to `{ expr }`.
fn print_inline_if(printer: &mut Printer<'_>, expr_if: &syn::ExprIf) {
    expr_if.if_token.pretty_print(printer);
    " ".pretty_print(printer);
    expr_if.cond.pretty_print(printer);
    " ".pretty_print(printer);
    inline_block(printer, &expr_if.then_branch);
    if let Some((else_token, else_branch)) = &expr_if.else_branch {
        " ".pretty_print(printer);
        else_token.pretty_print(printer);
        " ".pretty_print(printer);
        match &**else_branch {
            syn::Expr::If(nested) => print_inline_if(printer, nested),
            syn::Expr::Block(block) => inline_block(printer, &block.block),
            _ => else_branch.pretty_print(printer),
        }
    }
}

impl PrettyPrint for syn::ExprIf {
    fn pretty_print(&self, printer: &mut Printer<'_>) {
        self.attrs.pretty_print(printer);
        if is_inline_if_chain(self) && !printer.has_comment_before(self.span().end()) {
            printer.scan_begin(BreakMode::Consistent);
            print_inline_if(printer, self);
            printer.scan_end();
            return;
        }
        self.if_token.pretty_print(printer);
        " ".pretty_print(printer);
        self.cond.pretty_print(printer);
        " ".pretty_print(printer);
        self.then_branch.pretty_print(printer);
        if let Some((else_token, else_branch)) = &self.else_branch {
            " ".pretty_print(printer);
            else_token.pretty_print(printer);
            " ".pretty_print(printer);
            else_branch.pretty_print(printer);
        }
    }
}

impl PrettyPrint for syn::ExprMatch {
    fn pretty_print(&self, printer: &mut Printer<'_>) {
        self.attrs.pretty_print(printer);
        self.match_token.pretty_print(printer);
        " ".pretty_print(printer);
        self.expr.pretty_print(printer);
        " ".pretty_print(printer);
        common::statement_braces(printer, &self.brace_token, &self.arms);
    }
}

impl PrettyPrint for syn::Arm {
    fn pretty_print(&self, printer: &mut Printer<'_>) {
        self.attrs.pretty_print(printer);
        self.pat.pretty_print(printer);
        if let Some((if_token, guard)) = &self.guard {
            " ".pretty_print(printer);
            if_token.pretty_print(printer);
            " ".pretty_print(printer);
            guard.pretty_print(printer);
        }
        " ".pretty_print(printer);
        self.fat_arrow_token.pretty_print(printer);
        " ".pretty_print(printer);
        self.body.pretty_print(printer);
        // A block-like body is self-delimiting; everything else ends with a
        // comma, whether or not the source had one.
        if !is_block_like(&self.body) {
            common::comma(printer, self.comma.as_ref());
        }
    }
}

impl PrettyPrint for syn::ExprForLoop {
    fn pretty_print(&self, printer: &mut Printer<'_>) {
        self.attrs.pretty_print(printer);
        self.label.pretty_print(printer);
        self.for_token.pretty_print(printer);
        " ".pretty_print(printer);
        self.pat.pretty_print(printer);
        " ".pretty_print(printer);
        self.in_token.pretty_print(printer);
        " ".pretty_print(printer);
        self.expr.pretty_print(printer);
        " ".pretty_print(printer);
        self.body.pretty_print(printer);
    }
}

impl PrettyPrint for syn::ExprWhile {
    fn pretty_print(&self, printer: &mut Printer<'_>) {
        self.attrs.pretty_print(printer);
        self.label.pretty_print(printer);
        self.while_token.pretty_print(printer);
        " ".pretty_print(printer);
        self.cond.pretty_print(printer);
        " ".pretty_print(printer);
        self.body.pretty_print(printer);
    }
}

impl PrettyPrint for syn::ExprLoop {
    fn pretty_print(&self, printer: &mut Printer<'_>) {
        self.attrs.pretty_print(printer);
        self.label.pretty_print(printer);
        self.loop_token.pretty_print(printer);
        " ".pretty_print(printer);
        self.body.pretty_print(printer);
    }
}

impl PrettyPrint for syn::ExprClosure {
    fn pretty_print(&self, printer: &mut Printer<'_>) {
        self.attrs.pretty_print(printer);
        self.lifetimes.pretty_print(printer);
        if let Some(constness) = &self.constness {
            constness.pretty_print(printer);
            " ".pretty_print(printer);
        }
        if let Some(movability) = &self.movability {
            movability.pretty_print(printer);
            " ".pretty_print(printer);
        }
        if let Some(asyncness) = &self.asyncness {
            asyncness.pretty_print(printer);
            " ".pretty_print(printer);
        }
        if let Some(capture) = &self.capture {
            capture.pretty_print(printer);
            " ".pretty_print(printer);
        }
        self.or1_token.pretty_print(printer);
        for pair in self.inputs.pairs() {
            pair.value().pretty_print(printer);
            if let Some(punct) = pair.punct() {
                punct.pretty_print(printer);
                " ".pretty_print(printer);
            }
        }
        self.or2_token.pretty_print(printer);
        self.output.pretty_print(printer);

        if let syn::Expr::Block(block) = &*self.body
            && block.attrs.is_empty()
            && block.label.is_none()
        {
            " ".pretty_print(printer);
            keyword_block(printer, &block.block);
        } else if is_block_like(&self.body) || matches!(self.output, syn::ReturnType::Type(..)) {
            " ".pretty_print(printer);
            self.body.pretty_print(printer);
        } else {
            printer.scan_begin(BreakMode::Inconsistent);
            printer.scan_indent(1);
            printer.scan_break();
            " ".pretty_print(printer);
            self.body.pretty_print(printer);
            printer.scan_indent(-1);
            printer.scan_end();
        }
    }
}

impl PrettyPrint for syn::ExprAsync {
    fn pretty_print(&self, printer: &mut Printer<'_>) {
        self.attrs.pretty_print(printer);
        self.async_token.pretty_print(printer);
        " ".pretty_print(printer);
        if let Some(capture) = &self.capture {
            capture.pretty_print(printer);
            " ".pretty_print(printer);
        }
        keyword_block(printer, &self.block);
    }
}

impl PrettyPrint for syn::ExprUnsafe {
    fn pretty_print(&self, printer: &mut Printer<'_>) {
        self.attrs.pretty_print(printer);
        self.unsafe_token.pretty_print(printer);
        " ".pretty_print(printer);
        keyword_block(printer, &self.block);
    }
}

impl PrettyPrint for syn::ExprConst {
    fn pretty_print(&self, printer: &mut Printer<'_>) {
        self.attrs.pretty_print(printer);
        self.const_token.pretty_print(printer);
        " ".pretty_print(printer);
        keyword_block(printer, &self.block);
    }
}

impl PrettyPrint for syn::ExprTryBlock {
    fn pretty_print(&self, printer: &mut Printer<'_>) {
        self.attrs.pretty_print(printer);
        self.try_token.pretty_print(printer);
        " ".pretty_print(printer);
        keyword_block(printer, &self.block);
    }
}

impl PrettyPrint for syn::ExprReturn {
    fn pretty_print(&self, printer: &mut Printer<'_>) {
        self.attrs.pretty_print(printer);
        self.return_token.pretty_print(printer);
        if let Some(expr) = &self.expr {
            " ".pretty_print(printer);
            expr.pretty_print(printer);
        }
    }
}

impl PrettyPrint for syn::ExprBreak {
    fn pretty_print(&self, printer: &mut Printer<'_>) {
        self.attrs.pretty_print(printer);
        self.break_token.pretty_print(printer);
        if let Some(label) = &self.label {
            " ".pretty_print(printer);
            label.pretty_print(printer);
        }
        if let Some(expr) = &self.expr {
            " ".pretty_print(printer);
            expr.pretty_print(printer);
        }
    }
}

impl PrettyPrint for syn::ExprContinue {
    fn pretty_print(&self, printer: &mut Printer<'_>) {
        self.attrs.pretty_print(printer);
        self.continue_token.pretty_print(printer);
        if let Some(label) = &self.label {
            " ".pretty_print(printer);
            label.pretty_print(printer);
        }
    }
}

impl PrettyPrint for syn::ExprYield {
    fn pretty_print(&self, printer: &mut Printer<'_>) {
        self.attrs.pretty_print(printer);
        self.yield_token.pretty_print(printer);
        if let Some(expr) = &self.expr {
            " ".pretty_print(printer);
            expr.pretty_print(printer);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::super::common::tests::format;

    fn expr(source: &str) -> String {
        format::<syn::Expr>(source)
    }

    #[test]
    fn literals_and_paths() {
        assert_eq!(expr("42"), "42");
        assert_eq!(expr("\"hello\""), "\"hello\"");
        assert_eq!(expr("some::path::Item"), "some::path::Item");
    }

    #[test]
    fn call_fits_on_one_line() {
        assert_eq!(expr("foo(a, b)"), "foo(a, b)");
        assert_eq!(expr("foo()"), "foo()");
    }

    #[test]
    fn call_normalizes_spacing() {
        assert_eq!(expr("foo ( a , b )"), "foo(a, b)");
    }

    #[test]
    fn long_call_breaks_with_trailing_comma() {
        assert_eq!(
            expr(
                "some_function(first_extremely_long_argument_expression, second_extremely_long_argument_expression)"
            ),
            "some_function(\n    first_extremely_long_argument_expression,\n    second_extremely_long_argument_expression,\n)",
        );
    }

    #[test]
    fn nested_call_breaks_outside_in() {
        assert_eq!(
            expr(
                "outer_function(inner_function(first_long_argument_value, second_long_argument_value), third)"
            ),
            "outer_function(\n    inner_function(first_long_argument_value, second_long_argument_value),\n    third,\n)",
        );
    }

    #[test]
    fn binary_fits_on_one_line() {
        assert_eq!(expr("a + b * c"), "a + b * c");
        assert_eq!(expr("a && b || c"), "a && b || c");
    }

    #[test]
    fn long_binary_breaks_before_operators() {
        assert_eq!(
            expr(
                "first_long_operand_name + second_long_operand_name + third_long_operand_name + fourth_operand"
            ),
            "first_long_operand_name\n    + second_long_operand_name\n    + third_long_operand_name\n    + fourth_operand",
        );
    }

    #[test]
    fn comparison_and_assignment_ops() {
        assert_eq!(expr("a <= b"), "a <= b");
        assert_eq!(expr("count += 1"), "count += 1");
        assert_eq!(expr("total = total + step"), "total = total + step");
    }

    #[test]
    fn unary_and_reference() {
        assert_eq!(expr("!done"), "!done");
        assert_eq!(expr("-offset"), "-offset");
        assert_eq!(expr("*pointer"), "*pointer");
        assert_eq!(expr("&value"), "&value");
        assert_eq!(expr("&mut value"), "&mut value");
    }

    #[test]
    fn cast() {
        assert_eq!(expr("value as u64"), "value as u64");
    }

    #[test]
    fn index_try_await() {
        assert_eq!(expr("items[0]"), "items[0]");
        assert_eq!(expr("fallible()?"), "fallible()?");
        assert_eq!(expr("future.await"), "future.await");
        assert_eq!(
            expr("client.get(url)?.json().await?"),
            "client.get(url)?.json().await?"
        );
    }

    #[test]
    fn field_access() {
        assert_eq!(expr("point.x"), "point.x");
        assert_eq!(expr("tuple.0"), "tuple.0");
        assert_eq!(expr("outer.inner.value"), "outer.inner.value");
    }

    #[test]
    fn short_method_chain_stays_inline() {
        assert_eq!(expr("name.to_string()"), "name.to_string()");
        assert_eq!(expr("items.iter().count()"), "items.iter().count()");
    }

    #[test]
    fn long_method_chain_breaks_before_dots() {
        assert_eq!(
            expr(
                "collection.iter().filter(|item| item.is_active()).map(|item| item.name.clone()).collect::<Vec<_>>()"
            ),
            "collection\n    .iter()\n    .filter(|item| item.is_active())\n    .map(|item| item.name.clone())\n    .collect::<Vec<_>>()",
        );
    }

    #[test]
    fn single_method_call_breaks_arguments_not_dot() {
        assert_eq!(
            expr(
                "receiver.method_name(first_extremely_long_argument_value, second_extremely_long_argument_value)"
            ),
            "receiver.method_name(\n    first_extremely_long_argument_value,\n    second_extremely_long_argument_value,\n)",
        );
    }

    #[test]
    fn arrays_and_tuples() {
        assert_eq!(expr("[]"), "[]");
        assert_eq!(expr("[1, 2, 3]"), "[1, 2, 3]");
        assert_eq!(expr("[0u8; 32]"), "[0u8; 32]");
        assert_eq!(expr("()"), "()");
        assert_eq!(expr("(a, b)"), "(a, b)");
        assert_eq!(expr("(a,)"), "(a,)");
    }

    #[test]
    fn long_array_breaks() {
        assert_eq!(
            expr(
                "[first_extremely_long_element_name, second_extremely_long_element_name, third_lengthy_element]"
            ),
            "[\n    first_extremely_long_element_name,\n    second_extremely_long_element_name,\n    third_lengthy_element,\n]",
        );
    }

    #[test]
    fn ranges() {
        assert_eq!(expr("0..10"), "0..10");
        assert_eq!(expr("start..=end"), "start..=end");
        assert_eq!(expr(".."), "..");
        assert_eq!(expr("1.."), "1..");
    }

    #[test]
    fn struct_literal() {
        assert_eq!(expr("Point { x: 1, y: 2 }"), "Point { x: 1, y: 2 }");
        assert_eq!(expr("Point { x, y }"), "Point { x, y }");
        assert_eq!(expr("Empty {}"), "Empty {}");
    }

    #[test]
    fn struct_literal_with_rest_has_no_trailing_comma() {
        assert_eq!(
            expr("Config { verbose: true, ..Default::default() }"),
            "Config { verbose: true, ..Default::default() }",
        );
        assert_eq!(
            expr(
                "Configuration { first_long_field_name: first_value, second_long_field_name: second_value, ..defaults() }"
            ),
            "Configuration {\n    first_long_field_name: first_value,\n    second_long_field_name: second_value,\n    ..defaults()\n}",
        );
    }

    #[test]
    fn short_if_else_stays_inline() {
        assert_eq!(
            expr("if enabled { on() } else { off() }"),
            "if enabled { on() } else { off() }",
        );
    }

    #[test]
    fn if_without_else_breaks() {
        assert_eq!(expr("if enabled { on() }"), "if enabled {\n    on()\n}");
    }

    #[test]
    fn if_with_statement_branch_breaks() {
        assert_eq!(
            expr("if enabled { on(); } else { off() }"),
            "if enabled {\n    on();\n} else {\n    off()\n}",
        );
    }

    #[test]
    fn long_if_else_breaks_every_branch() {
        assert_eq!(
            expr(
                "if enabled { first_long_branch_expression_value() } else { second_long_branch_expression_value() }"
            ),
            "if enabled {\n    first_long_branch_expression_value()\n} else {\n    second_long_branch_expression_value()\n}",
        );
    }

    #[test]
    fn short_else_if_chain_stays_inline() {
        assert_eq!(
            expr("if a { 1 } else if b { 2 } else { 3 }"),
            "if a { 1 } else if b { 2 } else { 3 }",
        );
    }

    #[test]
    fn if_let() {
        assert_eq!(
            expr("if let Some(value) = optional { use_it(value) }"),
            "if let Some(value) = optional {\n    use_it(value)\n}",
        );
    }

    #[test]
    fn match_arms_one_per_line() {
        assert_eq!(
            expr("match status { Status::Active => 1, Status::Idle => 2, _ => 0 }"),
            "match status {\n    Status::Active => 1,\n    Status::Idle => 2,\n    _ => 0,\n}",
        );
    }

    #[test]
    fn match_with_guard_and_block_arm() {
        assert_eq!(
            expr("match value { n if n > 0 => { positive(n) } _ => negative() }"),
            "match value {\n    n if n > 0 => {\n        positive(n)\n    }\n    _ => negative(),\n}",
        );
    }

    #[test]
    fn empty_match() {
        assert_eq!(expr("match never {}"), "match never {}");
    }

    #[test]
    fn loops() {
        assert_eq!(expr("loop { tick() }"), "loop {\n    tick()\n}");
        assert_eq!(
            expr("while running { step() }"),
            "while running {\n    step()\n}",
        );
        assert_eq!(
            expr("for item in items { handle(item) }"),
            "for item in items {\n    handle(item)\n}",
        );
        assert_eq!(
            expr("'outer: loop { break 'outer }"),
            "'outer: loop {\n    break 'outer\n}",
        );
    }

    #[test]
    fn closures() {
        assert_eq!(expr("|| ready()"), "|| ready()");
        assert_eq!(expr("|x| x + 1"), "|x| x + 1");
        assert_eq!(expr("move |x, y| x * y"), "move |x, y| x * y");
        assert_eq!(expr("|x: u32| -> u32 { x }"), "|x: u32| -> u32 { x }");
    }

    #[test]
    fn closure_with_block_body_breaks_block() {
        assert_eq!(
            expr("|event| { handle(event); log(event) }"),
            "|event| {\n    handle(event);\n    log(event)\n}",
        );
    }

    #[test]
    fn control_flow_keywords() {
        assert_eq!(expr("return"), "return");
        assert_eq!(expr("return value"), "return value");
        assert_eq!(expr("break"), "break");
        assert_eq!(expr("continue"), "continue");
    }

    #[test]
    fn short_async_and_unsafe_blocks_stay_inline() {
        assert_eq!(
            expr("async move { fetch().await }"),
            "async move { fetch().await }"
        );
        assert_eq!(expr("unsafe { poke() }"), "unsafe { poke() }");
    }

    #[test]
    fn multi_statement_unsafe_block_breaks() {
        assert_eq!(
            expr("unsafe { init(); poke() }"),
            "unsafe {\n    init();\n    poke()\n}",
        );
    }

    #[test]
    fn parenthesized() {
        assert_eq!(expr("(a + b) * c"), "(a + b) * c");
    }

    #[test]
    fn comment_between_arguments() {
        assert_eq!(expr("foo(/* label */ value)"), "foo(/* label */ value)");
        assert_eq!(expr("foo(a, /* b */ c)"), "foo(a, /* b */ c)");
    }

    #[test]
    fn comment_in_binary_chain() {
        assert_eq!(expr("a + /* two */ b"), "a + /* two */ b");
    }

    #[test]
    fn qualified_path_call() {
        assert_eq!(
            expr("<Config as Default>::default()"),
            "<Config as Default>::default()",
        );
    }
}
