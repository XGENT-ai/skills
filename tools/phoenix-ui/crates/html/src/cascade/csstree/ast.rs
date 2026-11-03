//! The css-tree AST subset the cascade needs (see `parser.rs`).

/// `Declaration.important`: `false`, `true` for `!important`, or the raw
/// ident for hacks like `!ie`.
#[derive(Debug, Clone, PartialEq)]
pub enum Important {
    No,
    Yes,
    Other(String),
}

impl Important {
    /// JS `!!child.important`.
    pub fn truthy(&self) -> bool {
        !matches!(self, Important::No)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum Node {
    StyleSheet {
        children: Vec<Node>,
    },
    Rule {
        prelude: Box<Node>,
        block: Box<Node>,
    },
    Atrule {
        name: String,
        prelude: Option<Box<Node>>,
        block: Option<Box<Node>>,
    },
    Block {
        children: Vec<Node>,
    },
    Declaration {
        important: Important,
        property: String,
        value: Box<Node>,
    },
    Raw {
        value: String,
    },
    Comment {
        value: String,
    },
    Cdo,
    Cdc,
    Value {
        children: Vec<Node>,
    },
    WhiteSpace {
        value: String,
    },
    Hash {
        value: String,
    },
    Operator {
        value: String,
    },
    Parentheses {
        children: Vec<Node>,
    },
    Brackets {
        children: Vec<Node>,
    },
    Str {
        value: String,
    },
    Dimension {
        value: String,
        unit: String,
    },
    Percentage {
        value: String,
    },
    Number {
        value: String,
    },
    Function {
        name: String,
        children: Vec<Node>,
    },
    Url {
        value: String,
    },
    Identifier {
        name: String,
    },
    UnicodeRange {
        value: String,
    },
    SelectorList {
        children: Vec<Node>,
    },
    Selector {
        children: Vec<Node>,
    },
    TypeSelector {
        name: String,
    },
    ClassSelector {
        name: String,
    },
    IdSelector {
        name: String,
    },
    AttributeSelector {
        name: Box<Node>,
        matcher: Option<String>,
        value: Option<Box<Node>>,
        flags: Option<String>,
    },
    PseudoClassSelector {
        name: String,
        children: Option<Vec<Node>>,
    },
    PseudoElementSelector {
        name: String,
        children: Option<Vec<Node>>,
    },
    Combinator {
        name: String,
    },
    NestingSelector,
    Nth {
        nth: Box<Node>,
        selector: Option<Box<Node>>,
    },
    AnPlusB {
        a: Option<String>,
        b: Option<String>,
    },
}

impl Node {
    pub fn is_raw(&self) -> bool {
        matches!(self, Node::Raw { .. })
    }
}
