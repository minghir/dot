#[derive(Debug, Clone, PartialEq)]
pub enum BinaryOp {
    Add,
    Sub,
    Mul,
    Div,
    Rem,
    EqEq,
    BangEq,
    Lt,
    LtEq,
    Gt,
    GtEq,
    And,
    Or,
}

#[derive(Debug, Clone, PartialEq)]
pub enum PlaceStep {
    Index(Expr),
    Field(String),
}

#[derive(Debug, Clone, PartialEq)]
pub enum Expr {
    Int(i64),
    Float(f64),
    Color([u8; 4]),
    Str(String),
    Var(String),
    List(Vec<Expr>),
    Index {
        target: Box<Expr>,
        index: Box<Expr>,
    },
    UnaryMinus(Box<Expr>),
    Binary {
        op: BinaryOp,
        left: Box<Expr>,
        right: Box<Expr>,
    },
    Call {
        name: String,
        args: Vec<Expr>,
    },
    StructInit {
        name: String,
        fields: Vec<(String, Expr)>,
    },
    FieldAccess {
        target: Box<Expr>,
        field: String,
    },
}

#[derive(Debug, Clone, PartialEq)]
pub enum ForSource {
    Range {
        start: Expr,
        end: Expr,
        inclusive: bool,
    },
    Collection(Expr),
}

#[derive(Debug, Clone, PartialEq)]
pub enum Stmt {
    Let {
        name: String,
        value: Expr,
    },
    Assign {
        name: String,
        value: Expr,
    },
    FuncDecl {
        name: String,
        params: Vec<String>,
        body: Vec<Stmt>,
        line: usize,
    },
    StructDecl {
        name: String,
        fields: Vec<String>,
        line: usize,
    },
    Return {
        value: Option<Expr>,
        line: usize,
    },
    If {
        condition: Expr,
        then_branch: Vec<Stmt>,
        else_branch: Option<Vec<Stmt>>,
    },
    While {
        condition: Expr,
        body: Vec<Stmt>,
    },
    For {
        var: String,
        source: ForSource,
        body: Vec<Stmt>,
    },
    Break,
    Continue,
    PlaceAssign {
        target: String,
        path: Vec<PlaceStep>,
        value: Expr,
        line: usize,
    },
    Import {
        path: Expr,
        line: usize,
    },
    ExprStmt(Expr),
}

#[derive(Debug, Clone, PartialEq)]
pub struct Program {
    pub statements: Vec<Stmt>,
}