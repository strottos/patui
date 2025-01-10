use thiserror::Error;

use super::{
    ast::{Expr, Lit, TermPart},
    PatuiExpr,
};

/// Error type for querying a `PatuiExpr`.
#[derive(Error, Debug)]
pub enum PatuiExprVisitorError {
    /// Visitor implementer defined error.
    #[error("Visitor error: {0}")]
    VisitorError(String),
}

pub(crate) trait Visitor {
    fn visit_expr(&mut self, _expr: &Expr) -> Result<(), PatuiExprVisitorError> {
        Ok(())
    }

    fn visit_term(&mut self, _term: &Vec<TermPart>) -> Result<(), PatuiExprVisitorError> {
        Ok(())
    }

    fn visit_lit(&mut self, _lit: &Lit) -> Result<(), PatuiExprVisitorError> {
        Ok(())
    }

    fn visit_ident(&mut self, _ident: &str) -> Result<(), PatuiExprVisitorError> {
        Ok(())
    }

    fn visit_call(
        &mut self,
        _function: &str,
        _args: &Vec<Expr>,
    ) -> Result<(), PatuiExprVisitorError> {
        Ok(())
    }
}

impl PatuiExpr {
    pub(crate) fn visit(&self, visitor: &mut dyn Visitor) -> Result<(), PatuiExprVisitorError> {
        self.expr().visit(visitor)
    }
}

impl Expr {
    pub(crate) fn visit(&self, visitor: &mut dyn Visitor) -> Result<(), PatuiExprVisitorError> {
        self.walk_expr(visitor)?;
        visitor.visit_expr(self)?;

        Ok(())
    }

    fn walk_expr(&self, visitor: &mut dyn Visitor) -> Result<(), PatuiExprVisitorError> {
        match self {
            Expr::Term(vec) => {
                for value in vec {
                    match value {
                        TermPart::Lit(lit) => visitor.visit_lit(lit)?,
                        TermPart::Ident(ident) => visitor.visit_ident(ident)?,
                        TermPart::Call(function, args) => {
                            visitor.visit_call(function, args)?;
                            args.iter()
                                .map(|expr| expr.visit(visitor))
                                .collect::<Result<Vec<_>, PatuiExprVisitorError>>()?;
                        }
                        TermPart::Index(expr) => {
                            expr.visit(visitor)?;
                        }
                    }
                }
                visitor.visit_term(vec)?;
            }
            Expr::If(expr, expr1, expr2) => {
                expr.visit(visitor)?;
                expr1.visit(visitor)?;
                expr2.visit(visitor)?;
            }
            Expr::UnOp(un_op, expr) => {
                expr.visit(visitor)?;
            }
            Expr::BinOp(bin_op, expr, expr1) => {
                expr.visit(visitor)?;
                expr1.visit(visitor)?;
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashSet;

    use crate::expr::ast::{BinOp, UnOp};

    use super::*;

    use assertor::*;
    use tracing_test::traced_test;

    #[traced_test]
    #[test]
    fn visitor_basic() {
        let expr = PatuiExpr::try_from("1".to_string()).unwrap();

        struct StepVisitor {
            visited: Option<Lit>,
        }

        impl Visitor for StepVisitor {
            fn visit_lit(&mut self, lit: &Lit) -> Result<(), PatuiExprVisitorError> {
                self.visited = Some(lit.clone());
                Ok(())
            }
        }

        let mut step_visitor = StepVisitor { visited: None };

        expr.visit(&mut step_visitor).unwrap();

        assert_that!(step_visitor.visited).is_some();
        assert_that!(step_visitor.visited.unwrap()).is_equal_to(&Lit::Integer(1));
    }

    #[traced_test]
    #[test]
    fn visitor_nested() {
        let expr = PatuiExpr::try_from(
            "(1 == (2 + 3)) && (true || (foo.bar[1] == (bar[baz()]))) || (\"123\" == 123) || ([1,2,3] == {\"a\": 1}) || !true",
        )
        .unwrap();

        struct StepVisitor {
            lits: Vec<Lit>,
            idents: Vec<String>,
            calls: Vec<(String, Vec<Expr>)>,
            terms: Vec<Vec<TermPart>>,
            exprs: Vec<Expr>,
        }

        impl Visitor for StepVisitor {
            fn visit_lit(&mut self, lit: &Lit) -> Result<(), PatuiExprVisitorError> {
                self.lits.push(lit.clone());
                Ok(())
            }

            fn visit_ident(&mut self, ident: &str) -> Result<(), PatuiExprVisitorError> {
                self.idents.push(ident.to_string());
                Ok(())
            }

            fn visit_call(
                &mut self,
                function: &str,
                args: &Vec<Expr>,
            ) -> Result<(), PatuiExprVisitorError> {
                self.calls.push((function.to_string(), args.clone()));
                Ok(())
            }

            fn visit_term(&mut self, term: &Vec<TermPart>) -> Result<(), PatuiExprVisitorError> {
                self.terms.push(term.clone());
                Ok(())
            }

            fn visit_expr(&mut self, expr: &Expr) -> Result<(), PatuiExprVisitorError> {
                self.exprs.push(expr.clone());
                Ok(())
            }
        }

        let mut step_visitor = StepVisitor {
            lits: Vec::new(),
            idents: Vec::new(),
            calls: Vec::new(),
            terms: Vec::new(),
            exprs: Vec::new(),
        };

        expr.visit(&mut step_visitor).unwrap();

        assert_eq!(
            step_visitor.lits,
            vec![
                Lit::Integer(1),
                Lit::Integer(2),
                Lit::Integer(3),
                Lit::Bool(true),
                Lit::Integer(1),
                Lit::String("123".to_string()),
                Lit::Integer(123),
                Lit::List(vec![
                    Expr::Term(vec![TermPart::Lit(Lit::Integer(1))]),
                    Expr::Term(vec![TermPart::Lit(Lit::Integer(2))]),
                    Expr::Term(vec![TermPart::Lit(Lit::Integer(3))]),
                ]),
                Lit::Map(vec![(
                    "a".to_string(),
                    Expr::Term(vec![TermPart::Lit(Lit::Integer(1))])
                )]),
                Lit::Bool(true),
            ],
        );
        assert_eq!(
            step_visitor.idents,
            vec!["foo".to_string(), "bar".to_string(), "bar".to_string(),]
        );
        assert_eq!(step_visitor.calls, vec![("baz".to_string(), vec![]),]);
        assert_eq!(
            step_visitor.terms,
            vec![
                vec![TermPart::Lit(Lit::Integer(1))],
                vec![TermPart::Lit(Lit::Integer(2))],
                vec![TermPart::Lit(Lit::Integer(3))],
                vec![TermPart::Lit(Lit::Bool(true))],
                vec![TermPart::Lit(Lit::Integer(1))],
                vec![
                    TermPart::Ident("foo".to_string()),
                    TermPart::Ident("bar".to_string()),
                    TermPart::Index(Box::new(Expr::Term(vec![TermPart::Lit(Lit::Integer(1))]))),
                ],
                vec![TermPart::Call("baz".to_string(), vec![])],
                vec![
                    TermPart::Ident("bar".to_string()),
                    TermPart::Index(Box::new(Expr::Term(vec![TermPart::Call(
                        "baz".to_string(),
                        vec![]
                    )])),),
                ],
                vec![TermPart::Lit(Lit::String("123".to_string()))],
                vec![TermPart::Lit(Lit::Integer(123))],
                vec![TermPart::Lit(Lit::List(vec![
                    Expr::Term(vec![TermPart::Lit(Lit::Integer(1))]),
                    Expr::Term(vec![TermPart::Lit(Lit::Integer(2))]),
                    Expr::Term(vec![TermPart::Lit(Lit::Integer(3))]),
                ]))],
                vec![TermPart::Lit(Lit::Map(vec![(
                    "a".to_string(),
                    Expr::Term(vec![TermPart::Lit(Lit::Integer(1))])
                )]))],
                vec![TermPart::Lit(Lit::Bool(true))],
            ]
        );
        // What kind of a mad person would I be to assert_eq! the entire thing? Nevertheless, here
        // we are. If they are ordered differently because of some changes in algorithms we don't
        // care, if something changes this should warn you and be wary in such cases unless you
        // are actively changing the AST structure (and you might want to just redo this if so).
        assert_eq!(
            step_visitor.exprs,
            vec![
                Expr::Term(vec![TermPart::Lit(Lit::Integer(1))]),
                Expr::Term(vec![TermPart::Lit(Lit::Integer(2))]),
                Expr::Term(vec![TermPart::Lit(Lit::Integer(3))]),
                Expr::BinOp(
                    BinOp::Add,
                    Box::new(Expr::Term(vec![TermPart::Lit(Lit::Integer(2))])),
                    Box::new(Expr::Term(vec![TermPart::Lit(Lit::Integer(3))])),
                ),
                Expr::BinOp(
                    BinOp::Equal,
                    Box::new(Expr::Term(vec![TermPart::Lit(Lit::Integer(1))])),
                    Box::new(Expr::BinOp(
                        BinOp::Add,
                        Box::new(Expr::Term(vec![TermPart::Lit(Lit::Integer(2))])),
                        Box::new(Expr::Term(vec![TermPart::Lit(Lit::Integer(3))])),
                    )),
                ),
                Expr::Term(vec![TermPart::Lit(Lit::Bool(true))]),
                Expr::Term(vec![TermPart::Lit(Lit::Integer(1))]),
                Expr::Term(vec![
                    TermPart::Ident("foo".to_string()),
                    TermPart::Ident("bar".to_string()),
                    TermPart::Index(Box::new(Expr::Term(vec![TermPart::Lit(Lit::Integer(1))])),),
                ]),
                Expr::Term(vec![TermPart::Call("baz".to_string(), vec![])]),
                Expr::Term(vec![
                    TermPart::Ident("bar".to_string()),
                    TermPart::Index(Box::new(Expr::Term(vec![TermPart::Call(
                        "baz".to_string(),
                        vec![]
                    )])),),
                ]),
                Expr::BinOp(
                    BinOp::Equal,
                    Box::new(Expr::Term(vec![
                        TermPart::Ident("foo".to_string()),
                        TermPart::Ident("bar".to_string()),
                        TermPart::Index(Box::new(Expr::Term(vec![TermPart::Lit(Lit::Integer(1))])),),
                    ]),),
                    Box::new(Expr::Term(vec![
                        TermPart::Ident("bar".to_string()),
                        TermPart::Index(Box::new(Expr::Term(vec![TermPart::Call(
                            "baz".to_string(),
                            vec![]
                        )])),),
                    ]),),
                ),
                Expr::BinOp(
                    BinOp::Or,
                    Box::new(Expr::Term(vec![TermPart::Lit(Lit::Bool(true))])),
                    Box::new(Expr::BinOp(
                        BinOp::Equal,
                        Box::new(Expr::Term(vec![
                            TermPart::Ident("foo".to_string()),
                            TermPart::Ident("bar".to_string()),
                            TermPart::Index(Box::new(Expr::Term(vec![TermPart::Lit(
                                Lit::Integer(1)
                            )])),),
                        ]),),
                        Box::new(Expr::Term(vec![
                            TermPart::Ident("bar".to_string()),
                            TermPart::Index(Box::new(Expr::Term(vec![TermPart::Call(
                                "baz".to_string(),
                                vec![]
                            )])),),
                        ]),),
                    )),
                ),
                Expr::Term(vec![TermPart::Lit(Lit::String("123".to_string()))]),
                Expr::Term(vec![TermPart::Lit(Lit::Integer(123))]),
                Expr::BinOp(
                    BinOp::Equal,
                    Box::new(Expr::Term(vec![TermPart::Lit(Lit::String(
                        "123".to_string()
                    ))])),
                    Box::new(Expr::Term(vec![TermPart::Lit(Lit::Integer(123))])),
                ),
                Expr::Term(vec![TermPart::Lit(Lit::List(vec![
                    Expr::Term(vec![TermPart::Lit(Lit::Integer(1))]),
                    Expr::Term(vec![TermPart::Lit(Lit::Integer(2))]),
                    Expr::Term(vec![TermPart::Lit(Lit::Integer(3))]),
                ]))]),
                Expr::Term(vec![TermPart::Lit(Lit::Map(vec![(
                    "a".to_string(),
                    Expr::Term(vec![TermPart::Lit(Lit::Integer(1))])
                )]))]),
                Expr::BinOp(
                    BinOp::Equal,
                    Box::new(Expr::Term(vec![TermPart::Lit(Lit::List(vec![
                        Expr::Term(vec![TermPart::Lit(Lit::Integer(1))]),
                        Expr::Term(vec![TermPart::Lit(Lit::Integer(2))]),
                        Expr::Term(vec![TermPart::Lit(Lit::Integer(3))]),
                    ]))])),
                    Box::new(Expr::Term(vec![TermPart::Lit(Lit::Map(vec![(
                        "a".to_string(),
                        Expr::Term(vec![TermPart::Lit(Lit::Integer(1))])
                    )]))])),
                ),
                Expr::Term(vec![TermPart::Lit(Lit::Bool(true))]),
                Expr::UnOp(
                    UnOp::Not,
                    Box::new(Expr::Term(vec![TermPart::Lit(Lit::Bool(true))]))
                ),
                Expr::BinOp(
                    BinOp::Or,
                    Box::new(Expr::BinOp(
                        BinOp::Equal,
                        Box::new(Expr::Term(vec![TermPart::Lit(Lit::List(vec![
                            Expr::Term(vec![TermPart::Lit(Lit::Integer(1))]),
                            Expr::Term(vec![TermPart::Lit(Lit::Integer(2))]),
                            Expr::Term(vec![TermPart::Lit(Lit::Integer(3))]),
                        ]))])),
                        Box::new(Expr::Term(vec![TermPart::Lit(Lit::Map(vec![(
                            "a".to_string(),
                            Expr::Term(vec![TermPart::Lit(Lit::Integer(1))])
                        )]))])),
                    )),
                    Box::new(Expr::UnOp(
                        UnOp::Not,
                        Box::new(Expr::Term(vec![TermPart::Lit(Lit::Bool(true))]))
                    )),
                ),
                Expr::BinOp(
                    BinOp::Or,
                    Box::new(Expr::BinOp(
                        BinOp::Equal,
                        Box::new(Expr::Term(vec![TermPart::Lit(Lit::String(
                            "123".to_string()
                        ))])),
                        Box::new(Expr::Term(vec![TermPart::Lit(Lit::Integer(123))])),
                    )),
                    Box::new(Expr::BinOp(
                        BinOp::Or,
                        Box::new(Expr::BinOp(
                            BinOp::Equal,
                            Box::new(Expr::Term(vec![TermPart::Lit(Lit::List(vec![
                                Expr::Term(vec![TermPart::Lit(Lit::Integer(1))]),
                                Expr::Term(vec![TermPart::Lit(Lit::Integer(2))]),
                                Expr::Term(vec![TermPart::Lit(Lit::Integer(3))]),
                            ]))])),
                            Box::new(Expr::Term(vec![TermPart::Lit(Lit::Map(vec![(
                                "a".to_string(),
                                Expr::Term(vec![TermPart::Lit(Lit::Integer(1))])
                            )]))])),
                        )),
                        Box::new(Expr::UnOp(
                            UnOp::Not,
                            Box::new(Expr::Term(vec![TermPart::Lit(Lit::Bool(true))]))
                        )),
                    )),
                ),
                Expr::BinOp(
                    BinOp::Or,
                    Box::new(Expr::BinOp(
                        BinOp::Or,
                        Box::new(Expr::Term(vec![TermPart::Lit(Lit::Bool(true))])),
                        Box::new(Expr::BinOp(
                            BinOp::Equal,
                            Box::new(Expr::Term(vec![
                                TermPart::Ident("foo".to_string()),
                                TermPart::Ident("bar".to_string()),
                                TermPart::Index(Box::new(Expr::Term(vec![TermPart::Lit(
                                    Lit::Integer(1)
                                )])),),
                            ]),),
                            Box::new(Expr::Term(vec![
                                TermPart::Ident("bar".to_string()),
                                TermPart::Index(Box::new(Expr::Term(vec![TermPart::Call(
                                    "baz".to_string(),
                                    vec![]
                                )])),),
                            ]),),
                        )),
                    )),
                    Box::new(Expr::BinOp(
                        BinOp::Or,
                        Box::new(Expr::BinOp(
                            BinOp::Equal,
                            Box::new(Expr::Term(vec![TermPart::Lit(Lit::String(
                                "123".to_string()
                            ))])),
                            Box::new(Expr::Term(vec![TermPart::Lit(Lit::Integer(123))])),
                        )),
                        Box::new(Expr::BinOp(
                            BinOp::Or,
                            Box::new(Expr::BinOp(
                                BinOp::Equal,
                                Box::new(Expr::Term(vec![TermPart::Lit(Lit::List(vec![
                                    Expr::Term(vec![TermPart::Lit(Lit::Integer(1))]),
                                    Expr::Term(vec![TermPart::Lit(Lit::Integer(2))]),
                                    Expr::Term(vec![TermPart::Lit(Lit::Integer(3))]),
                                ]))])),
                                Box::new(Expr::Term(vec![TermPart::Lit(Lit::Map(vec![(
                                    "a".to_string(),
                                    Expr::Term(vec![TermPart::Lit(Lit::Integer(1))])
                                )]))])),
                            )),
                            Box::new(Expr::UnOp(
                                UnOp::Not,
                                Box::new(Expr::Term(vec![TermPart::Lit(Lit::Bool(true))]))
                            )),
                        )),
                    )),
                ),
                Expr::BinOp(
                    BinOp::And,
                    Box::new(Expr::BinOp(
                        BinOp::Equal,
                        Box::new(Expr::Term(vec![TermPart::Lit(Lit::Integer(1))])),
                        Box::new(Expr::BinOp(
                            BinOp::Add,
                            Box::new(Expr::Term(vec![TermPart::Lit(Lit::Integer(2))])),
                            Box::new(Expr::Term(vec![TermPart::Lit(Lit::Integer(3))])),
                        )),
                    )),
                    Box::new(Expr::BinOp(
                        BinOp::Or,
                        Box::new(Expr::BinOp(
                            BinOp::Or,
                            Box::new(Expr::Term(vec![TermPart::Lit(Lit::Bool(true))])),
                            Box::new(Expr::BinOp(
                                BinOp::Equal,
                                Box::new(Expr::Term(vec![
                                    TermPart::Ident("foo".to_string()),
                                    TermPart::Ident("bar".to_string()),
                                    TermPart::Index(Box::new(Expr::Term(vec![TermPart::Lit(
                                        Lit::Integer(1)
                                    )])),),
                                ]),),
                                Box::new(Expr::Term(vec![
                                    TermPart::Ident("bar".to_string()),
                                    TermPart::Index(Box::new(Expr::Term(vec![TermPart::Call(
                                        "baz".to_string(),
                                        vec![]
                                    )])),),
                                ]),),
                            )),
                        )),
                        Box::new(Expr::BinOp(
                            BinOp::Or,
                            Box::new(Expr::BinOp(
                                BinOp::Equal,
                                Box::new(Expr::Term(vec![TermPart::Lit(Lit::String(
                                    "123".to_string()
                                ))])),
                                Box::new(Expr::Term(vec![TermPart::Lit(Lit::Integer(123))])),
                            )),
                            Box::new(Expr::BinOp(
                                BinOp::Or,
                                Box::new(Expr::BinOp(
                                    BinOp::Equal,
                                    Box::new(Expr::Term(vec![TermPart::Lit(Lit::List(vec![
                                        Expr::Term(vec![TermPart::Lit(Lit::Integer(1))]),
                                        Expr::Term(vec![TermPart::Lit(Lit::Integer(2))]),
                                        Expr::Term(vec![TermPart::Lit(Lit::Integer(3))]),
                                    ]))])),
                                    Box::new(Expr::Term(vec![TermPart::Lit(Lit::Map(vec![(
                                        "a".to_string(),
                                        Expr::Term(vec![TermPart::Lit(Lit::Integer(1))])
                                    )]))])),
                                )),
                                Box::new(Expr::UnOp(
                                    UnOp::Not,
                                    Box::new(Expr::Term(vec![TermPart::Lit(Lit::Bool(true))]))
                                )),
                            )),
                        )),
                    )),
                ),
            ]
        );
    }
}
