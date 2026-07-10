use crate::ast::template::{
    TemplateBlock, TemplateBreak, TemplateContinue, TemplateElse, TemplateForLoop, TemplateIf,
    TemplateLet, TemplateMatch,
};

use super::{
    Attribute, AttributeNode, AttributeNodes, AttributeSpread, BindAttribute, EventHandler,
};

pub trait Visit<'ast> {
    fn visit_node(&mut self, node: &'ast AttributeNode) {
        visit_node(self, node);
    }

    fn visit_attribute(&mut self, node: &'ast Attribute) {
        visit_attribute(self, node);
    }

    fn visit_spread(&mut self, node: &'ast AttributeSpread) {
        visit_spread(self, node);
    }

    fn visit_bind_attribute(&mut self, node: &'ast BindAttribute) {
        visit_bind_attribute(self, node);
    }

    fn visit_event_handler(&mut self, node: &'ast EventHandler) {
        visit_event_handler(self, node);
    }

    fn visit_if(&mut self, node: &'ast TemplateIf<AttributeNodes>) {
        visit_if(self, node);
    }

    fn visit_else(&mut self, node: &'ast TemplateElse<AttributeNodes>) {
        visit_else(self, node);
    }

    fn visit_let(&mut self, node: &'ast TemplateLet) {
        visit_let(self, node);
    }

    fn visit_for_loop(&mut self, node: &'ast TemplateForLoop<AttributeNodes>) {
        visit_for_loop(self, node);
    }

    fn visit_continue(&mut self, node: &'ast TemplateContinue) {
        visit_continue(self, node);
    }

    fn visit_break(&mut self, node: &'ast TemplateBreak) {
        visit_break(self, node);
    }

    fn visit_match(&mut self, node: &'ast TemplateMatch<AttributeNode>) {
        visit_match(self, node);
    }

    fn visit_block(&mut self, node: &'ast TemplateBlock<AttributeNodes>) {
        visit_block(self, node);
    }
}

pub fn visit_node<'ast>(visit: &mut (impl Visit<'ast> + ?Sized), node: &'ast AttributeNode) {
    match node {
        AttributeNode::Attribute(inner) => visit.visit_attribute(inner),
        AttributeNode::Spread(inner) => visit.visit_spread(inner),
        AttributeNode::BindAttribute(inner) => visit.visit_bind_attribute(inner),
        AttributeNode::EventHandler(inner) => visit.visit_event_handler(inner),
        AttributeNode::If(inner) => visit.visit_if(inner),
        AttributeNode::Let(inner) => visit.visit_let(inner),
        AttributeNode::ForLoop(inner) => visit.visit_for_loop(inner),
        AttributeNode::Continue(inner) => visit.visit_continue(inner),
        AttributeNode::Break(inner) => visit.visit_break(inner),
        AttributeNode::Match(inner) => visit.visit_match(inner),
        AttributeNode::Block(inner) => visit.visit_block(inner),
    }
}

pub fn visit_attribute<'ast>(_visit: &mut (impl Visit<'ast> + ?Sized), _node: &'ast Attribute) {}

pub fn visit_spread<'ast>(_visit: &mut (impl Visit<'ast> + ?Sized), _node: &'ast AttributeSpread) {}

pub fn visit_bind_attribute<'ast>(
    _visit: &mut (impl Visit<'ast> + ?Sized),
    _node: &'ast BindAttribute,
) {
}

pub fn visit_event_handler<'ast>(
    _visit: &mut (impl Visit<'ast> + ?Sized),
    _node: &'ast EventHandler,
) {
}

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

pub fn visit_else<'ast>(
    visit: &mut (impl Visit<'ast> + ?Sized),
    node: &'ast TemplateElse<AttributeNodes>,
) {
    match node {
        TemplateElse::ElseIf { template_if, .. } => visit.visit_if(template_if),
        TemplateElse::Else { then_branch, .. } => visit_block(visit, then_branch),
    }
}

pub fn visit_let<'ast>(_visit: &mut (impl Visit<'ast> + ?Sized), _node: &'ast TemplateLet) {}

pub fn visit_for_loop<'ast>(
    visit: &mut (impl Visit<'ast> + ?Sized),
    node: &'ast TemplateForLoop<AttributeNodes>,
) {
    visit_block(visit, &node.body);
}

pub fn visit_continue<'ast>(
    _visit: &mut (impl Visit<'ast> + ?Sized),
    _node: &'ast TemplateContinue,
) {
}

pub fn visit_break<'ast>(_visit: &mut (impl Visit<'ast> + ?Sized), _node: &'ast TemplateBreak) {}

pub fn visit_match<'ast>(
    visit: &mut (impl Visit<'ast> + ?Sized),
    node: &'ast TemplateMatch<AttributeNode>,
) {
    for arm in &node.arms {
        visit.visit_node(&arm.body);
    }
}

pub fn visit_block<'ast>(
    visit: &mut (impl Visit<'ast> + ?Sized),
    node: &'ast TemplateBlock<AttributeNodes>,
) {
    for node in &node.children {
        visit.visit_node(node);
    }
}
