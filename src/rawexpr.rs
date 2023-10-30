use std::fmt::{Debug, Formatter, Error};

/// An un-interpreted expression as read in the GPR file.
pub enum RawExpr {
    Empty,
    StaticString(String), //  doesn't include surrounding quotes
    Identifier(String),   //  Could be "prj.pkg'attribute"
    Ampersand((Box<RawExpr>, Box<RawExpr>)),
    List(Vec<Box<RawExpr>>),
}

impl RawExpr {

    /// Combine two expressions with an "&"
    pub fn ampersand(self, right: Self) -> Self {
        match self {
            RawExpr::Empty => right,
            _ => RawExpr::Ampersand((Box::new(self), Box::new(right))),
        }
    }

    /// Append an element to a list
    pub fn append(&mut self, right: Self){
        match self {
            RawExpr::List(list) => list.push(Box::new(right)),
            _ => panic!("Can only append to a list expression"),
        }
    }
}

impl Debug for RawExpr {

    fn fmt(&self, f: &mut Formatter<'_>,) -> Result<(), Error> {
        match self {
            RawExpr::Empty           => write!(f, "<empty>"),
            RawExpr::StaticString(s) => write!(f, "'{}'", s),
            RawExpr::Identifier(s)   => write!(f, "{}", s),
            RawExpr::Ampersand((left, right)) =>
                write!(f, "{:?} & {:?}", left, right),
            RawExpr::List(v) =>
                write!(
                    f,
                    "({})",
                    v.iter()
                      .map(|e| format!("{:?}", e))
                      .collect::<Vec<String>>()
                      .join(", ")
                ),
        }
    }
}
