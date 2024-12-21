//! Visitor

use super::ast::*;

use eyre::Result;

pub trait Visitor {
    fn visit_expr(&mut self, _expr: &Expr) -> Result<()> {
        Ok(())
    }

    fn visit_term(&mut self, _term: &Term) -> Result<()> {
        Ok(())
    }

    fn visit_lit(&mut self, _lit: &Lit) -> Result<()> {
        Ok(())
    }
}

impl PatuiExpr {
    pub(crate) fn visit(&self, visitor: &mut dyn Visitor) -> Result<()> {
        self.expr.visit(visitor)
    }
}

impl Expr {
    pub(crate) fn visit(&self, visitor: &mut dyn Visitor) -> Result<()> {
        self.walk_expr(visitor)?;
        visitor.visit_expr(&self)?;

        Ok(())
    }

    fn walk_expr(&self, visitor: &mut dyn Visitor) -> Result<()> {
        match &self.kind {
            ExprKind::Lit(lit) => visitor.visit_lit(lit)?,
            ExprKind::Term(term) => visitor.visit_term(term)?,
            ExprKind::If(p, p1, p2) => {
                p.visit(visitor)?;
                p1.visit(visitor)?;
                p2.visit(visitor)?;
            }
            ExprKind::UnOp(_, p) => {
                p.visit(visitor)?;
            }
            ExprKind::BinOp(_, p, p1) => {
                p.visit(visitor)?;
                p1.visit(visitor)?;
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use assertor::*;
    use tracing_test::traced_test;

    #[traced_test]
    #[test]
    fn visitor_basic() {
        let expr = PatuiExpr {
            raw: "1".to_string(),
            expr: Expr {
                kind: ExprKind::Lit(Lit {
                    kind: LitKind::Integer("1".to_string()),
                }),
            },
        };

        struct StepVisitor {
            visited: Option<Lit>,
        }

        impl Visitor for StepVisitor {
            fn visit_lit(&mut self, lit: &Lit) -> Result<()> {
                self.visited = Some(lit.clone());
                Ok(())
            }
        }

        let mut step_visitor = StepVisitor { visited: None };

        expr.visit(&mut step_visitor).unwrap();

        assert_that!(step_visitor.visited).is_some();
        assert_that!(step_visitor.visited.unwrap()).is_equal_to(Lit {
            kind: LitKind::Integer("1".to_string()),
        });
    }

    #[traced_test]
    #[test]
    fn visitor_nested() {
        let expr = PatuiExpr::try_from(
            "(1 == (2 + 3)) && (true || (foo.bar[1] == (bar[baz()]))) || (\"123\" == 123) || ([1,2,3] == {\"a\": 1}) || !true",
        )
        .unwrap();

        struct StepVisitor {
            lit_visits: i32,
            term_visits: i32,
            expr_visits: i32,
        }

        impl Visitor for StepVisitor {
            fn visit_lit(&mut self, _lit: &Lit) -> Result<()> {
                self.lit_visits += 1;
                Ok(())
            }

            fn visit_term(&mut self, _term: &Term) -> Result<()> {
                self.term_visits += 1;
                Ok(())
            }

            fn visit_expr(&mut self, _expr: &Expr) -> Result<()> {
                self.expr_visits += 1;
                Ok(())
            }
        }

        let mut step_visitor = StepVisitor {
            lit_visits: 0,
            term_visits: 0,
            expr_visits: 0,
        };

        expr.visit(&mut step_visitor).unwrap();

        assert_eq!(step_visitor.lit_visits, 9);
        assert_eq!(step_visitor.term_visits, 2);
        assert_eq!(step_visitor.expr_visits, 22);
    }
}
