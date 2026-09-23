use super::{
    Attribute, AttributeNode, AttributeNodes, AttributeSpread, BindAttribute, EventHandler,
};
use crate::template::{
    TemplateBlock, TemplateBreak, TemplateContinue, TemplateElse, TemplateForLoop, TemplateIf,
    TemplateLocal, TemplateMatch,
};

/// A read-only walk over an attribute list's syntax tree, in the style of
/// `syn::visit`.
///
/// Each method visits one kind of node. The default implementations call the
/// free function of the same name, which visits the node's children. Override
/// a method to act on that kind of node, and call the free function from it to
/// keep walking into the children.
pub trait Visit<'ast> {
    /// Visits any node, dispatching on its kind.
    fn visit_node(&mut self, node: &'ast AttributeNode) {
        visit_node(self, node);
    }

    /// Visits a `name=value` attribute.
    fn visit_attribute(&mut self, node: &'ast Attribute) {
        visit_attribute(self, node);
    }

    /// Visits an inserted collection.
    fn visit_spread(&mut self, node: &'ast AttributeSpread) {
        visit_spread(self, node);
    }

    /// Visits a bind attribute.
    fn visit_bind_attribute(&mut self, node: &'ast BindAttribute) {
        visit_bind_attribute(self, node);
    }

    /// Visits an event handler.
    fn visit_event_handler(&mut self, node: &'ast EventHandler) {
        visit_event_handler(self, node);
    }

    /// Visits an `if`.
    fn visit_if(&mut self, node: &'ast TemplateIf<AttributeNodes>) {
        visit_if(self, node);
    }

    /// Visits the `else` branch of an `if`.
    fn visit_else(&mut self, node: &'ast TemplateElse<AttributeNodes>) {
        visit_else(self, node);
    }

    /// Visits a `let` binding.
    fn visit_local(&mut self, node: &'ast TemplateLocal) {
        visit_local(self, node);
    }

    /// Visits a `for` loop.
    fn visit_for_loop(&mut self, node: &'ast TemplateForLoop<AttributeNodes>) {
        visit_for_loop(self, node);
    }

    /// Visits a `continue;` statement.
    fn visit_continue(&mut self, node: &'ast TemplateContinue) {
        visit_continue(self, node);
    }

    /// Visits a `break;` statement.
    fn visit_break(&mut self, node: &'ast TemplateBreak) {
        visit_break(self, node);
    }

    /// Visits a `match`.
    fn visit_match(&mut self, node: &'ast TemplateMatch<AttributeNode>) {
        visit_match(self, node);
    }

    /// Visits a block.
    fn visit_block(&mut self, node: &'ast TemplateBlock<AttributeNodes>) {
        visit_block(self, node);
    }
}

/// Visits `node` by calling the [`Visit`] method for its kind.
pub fn visit_node<'ast>(visit: &mut (impl Visit<'ast> + ?Sized), node: &'ast AttributeNode) {
    match node {
        AttributeNode::Attribute(inner) => visit.visit_attribute(inner),
        AttributeNode::Spread(inner) => visit.visit_spread(inner),
        AttributeNode::BindAttribute(inner) => visit.visit_bind_attribute(inner),
        AttributeNode::EventHandler(inner) => visit.visit_event_handler(inner),
        AttributeNode::If(inner) => visit.visit_if(inner),
        AttributeNode::Local(inner) => visit.visit_local(inner),
        AttributeNode::ForLoop(inner) => visit.visit_for_loop(inner),
        AttributeNode::Continue(inner) => visit.visit_continue(inner),
        AttributeNode::Break(inner) => visit.visit_break(inner),
        AttributeNode::Match(inner) => visit.visit_match(inner),
        AttributeNode::Block(inner) => visit.visit_block(inner),
    }
}

/// Does nothing, since an attribute has no child nodes.
pub fn visit_attribute<'ast>(_visit: &mut (impl Visit<'ast> + ?Sized), _node: &'ast Attribute) {}

/// Does nothing, since an inserted collection has no child nodes.
pub fn visit_spread<'ast>(_visit: &mut (impl Visit<'ast> + ?Sized), _node: &'ast AttributeSpread) {}

/// Does nothing, since a bind attribute has no child nodes.
pub fn visit_bind_attribute<'ast>(
    _visit: &mut (impl Visit<'ast> + ?Sized),
    _node: &'ast BindAttribute,
) {
}

/// Does nothing, since an event handler has no child nodes.
pub fn visit_event_handler<'ast>(
    _visit: &mut (impl Visit<'ast> + ?Sized),
    _node: &'ast EventHandler,
) {
}

/// Visits the nodes of the `then` branch, followed by the `else` branch if
/// there is one.
pub fn visit_if<'ast>(
    visit: &mut (impl Visit<'ast> + ?Sized),
    node: &'ast TemplateIf<AttributeNodes>,
) {
    for node in &node.then_branch.children {
        visit.visit_node(node);
    }
    if let Some(else_branch) = &node.else_branch {
        visit.visit_else(else_branch);
    }
}

/// Visits the `else if` or `else` branch of an `if`.
pub fn visit_else<'ast>(
    visit: &mut (impl Visit<'ast> + ?Sized),
    node: &'ast TemplateElse<AttributeNodes>,
) {
    match node {
        TemplateElse::ElseIf { template_if, .. } => visit.visit_if(template_if),
        TemplateElse::Else { then_branch, .. } => visit_block(visit, then_branch),
    }
}

/// Does nothing, since a `let` binding has no child nodes.
pub fn visit_local<'ast>(_visit: &mut (impl Visit<'ast> + ?Sized), _node: &'ast TemplateLocal) {}

/// Visits the body of a `for` loop.
pub fn visit_for_loop<'ast>(
    visit: &mut (impl Visit<'ast> + ?Sized),
    node: &'ast TemplateForLoop<AttributeNodes>,
) {
    visit_block(visit, &node.body);
}

/// Does nothing, since a `continue;` statement has no child nodes.
pub fn visit_continue<'ast>(
    _visit: &mut (impl Visit<'ast> + ?Sized),
    _node: &'ast TemplateContinue,
) {
}

/// Does nothing, since a `break;` statement has no child nodes.
pub fn visit_break<'ast>(_visit: &mut (impl Visit<'ast> + ?Sized), _node: &'ast TemplateBreak) {}

/// Visits the body of each `match` arm.
pub fn visit_match<'ast>(
    visit: &mut (impl Visit<'ast> + ?Sized),
    node: &'ast TemplateMatch<AttributeNode>,
) {
    for arm in &node.arms {
        visit.visit_node(&arm.body);
    }
}

/// Visits each node in a block.
pub fn visit_block<'ast>(
    visit: &mut (impl Visit<'ast> + ?Sized),
    node: &'ast TemplateBlock<AttributeNodes>,
) {
    for node in &node.children {
        visit.visit_node(node);
    }
}
