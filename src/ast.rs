//! CoffeeQL Abstract Syntax Tree

use coffeeql_lexer::token::{
    CollectionKind, Duration, Distance,
    SortDir, DataType, Constraint,
};

// ── Root  

#[derive(Debug, Clone)]
pub enum Statement {
    Query(QueryNode),
    Shot(ShotNode),
    Grind(GrindNode),
    Menu(MenuNode),
}

// ── Query 

#[derive(Debug, Clone)]
pub struct QueryNode {
    pub collection: String,
    pub kind:       CollectionKind,
    pub chain:      Vec<ChainOp>,
}

// ── Chain Operations 

#[derive(Debug, Clone)]
pub enum ChainOp {
    Where(WhereNode),
    Give(GiveNode),
    Sort(SortNode),
    Cup(CupNode),
    Blend(BlendNode),
    Mix(MixNode),
    Pour(PourNode),
    Refill(RefillNode),
    Spill,
}

// ── Where 

#[derive(Debug, Clone)]
pub struct WhereNode {
    pub condition: Expression,
}

// ── Give 

#[derive(Debug, Clone)]
pub struct GiveNode {
    pub fields: Vec<FieldExpr>,
}

#[derive(Debug, Clone)]
pub enum FieldExpr {
    /// name, email
    Simple(String),
    /// specs.color
    Nested(Vec<String>),
    /// users[].name
    CrossStructured(String, String),
    /// orders{}.total
    CrossUnstructured(String, String),
    /// COUNT() as total
    Aggregate { func: AggFunc, field: Option<String>, alias: String },
    /// *
    Wildcard,
}

#[derive(Debug, Clone)]
pub enum AggFunc { Count, Sum, Avg, Max, Min }

// ── Sort

#[derive(Debug, Clone)]
pub struct SortNode {
    pub field:     String,
    pub direction: SortDir,
}

// ── Cup 

#[derive(Debug, Clone)]
pub struct CupNode {
    pub limit: u64,
}

// ── Blend 

#[derive(Debug, Clone)]
pub struct BlendNode {
    pub field: String,
}

// ── Mix 
#[derive(Debug, Clone)]
pub struct MixNode {
    pub collection:  String,
    pub kind:        CollectionKind,
    pub left_field:  String,
    pub right_field: String,
}

// ── Pour / Refill 

#[derive(Debug, Clone)]
pub struct PourNode {
    pub data: ObjectExpr,
}

#[derive(Debug, Clone)]
pub struct RefillNode {
    pub data: ObjectExpr,
}

#[derive(Debug, Clone)]
pub struct ObjectExpr {
    pub fields: Vec<(String, Expression)>,
}

// ── Shot

#[derive(Debug, Clone)]
pub struct ShotNode {
    pub queries: Vec<QueryNode>,
}

// ── Grind 

#[derive(Debug, Clone)]
pub struct GrindNode {
    pub collection: String,
    pub kind:       CollectionKind,
    pub schema:     Option<Vec<SchemaField>>,
    pub flex:       bool,
}

#[derive(Debug, Clone)]
pub struct SchemaField {
    pub name:        String,
    pub data_type:   DataType,
    pub constraints: Vec<Constraint>,
}

// ── Menu 

#[derive(Debug, Clone)]
pub struct MenuNode {
    pub collection: Option<(String, CollectionKind)>,
}

// ── Expression 

#[derive(Debug, Clone)]
pub enum Expression {
    // Literals
    Int(i64),
    Float(f64),
    Text(String),
    Bool(bool),
    Null,

    // Fields
    Field(String),
    NestedField(Vec<String>),

    // Binary ops
    Binary {
        left:  Box<Expression>,
        op:    BinaryOp,
        right: Box<Expression>,
    },

    // Logical
    And(Vec<Expression>),
    Or(Vec<Expression>),
    Not(Box<Expression>),

    // Special — geospatial
    Near {
        field:    String,
        lat:      f64,
        lon:      f64,
        distance: Distance,
    },

    // Special — AI similarity
    Like {
        field:     String,
        query:     String,
        threshold: f64,
    },

    // Special — array contains
    Has {
        field: String,
        value: Box<Expression>,
    },

    // Special — time window
    Last {
        field:    String,
        duration: Duration,
    },

    // Field exists check
    Exists { field: String },

    // Function calls
    FnCall { name: String, args: Vec<Expression> },

    // Wildcard
    Wildcard,
}

#[derive(Debug, Clone)]
pub enum BinaryOp {
    Eq, NotEq, Gt, Lt, Gte, Lte,
}
