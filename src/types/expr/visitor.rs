//! Visitor

use super::ast::*;

use eyre::Result;

pub trait Visitor {
    fn visit_expr(&mut self, _expr: &PatuiExpr) -> Result<()> {
        Ok(())
    }

    fn visit_ident(&mut self, _ident: &Ident) -> Result<()> {
        Ok(())
    }

    fn visit_lit(&mut self, _lit: &Lit) -> Result<()> {
        Ok(())
    }
}

impl PatuiExpr {
    pub(crate) fn visit(&self, visitor: &mut dyn Visitor) -> Result<()> {
        self.walk_expr(visitor)?;
        visitor.visit_expr(self)?;

        Ok(())
    }

    fn walk_expr(&self, visitor: &mut dyn Visitor) -> Result<()> {
        match &self.kind {
            ExprKind::Lit(lit) => visitor.visit_lit(lit)?,
            ExprKind::Ident(ident) => visitor.visit_ident(ident)?,
            ExprKind::Field(p, ident) => {
                p.visit(visitor)?;
                visitor.visit_ident(ident)?;
            }
            ExprKind::Call(p, vec) => {
                p.visit(visitor)?;
                for expr in vec {
                    expr.visit(visitor)?;
                }
            }
            ExprKind::Index(p, p1) => {
                p.visit(visitor)?;
                p1.visit(visitor)?;
            }
            ExprKind::If(p, p1, p2) => {
                p.visit(visitor)?;
                p1.visit(visitor)?;
                p2.visit(visitor)?;
            }
            ExprKind::List(vec) => {
                for expr in vec {
                    expr.visit(visitor)?;
                }
            }
            ExprKind::Map(vec) => {
                for elems in vec {
                    let (k, v) = &**elems;
                    k.visit(visitor)?;
                    v.visit(visitor)?;
                }
            }
            ExprKind::Set(vec) => {
                for expr in vec {
                    expr.visit(visitor)?;
                }
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
            kind: ExprKind::Lit(Lit {
                kind: LitKind::Integer("1".to_string()),
            }),
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
            ident_visits: i32,
            expr_visits: i32,
        }

        impl Visitor for StepVisitor {
            fn visit_lit(&mut self, _lit: &Lit) -> Result<()> {
                self.lit_visits += 1;
                Ok(())
            }

            fn visit_ident(&mut self, _ident: &Ident) -> Result<()> {
                self.ident_visits += 1;
                Ok(())
            }

            fn visit_expr(&mut self, _expr: &PatuiExpr) -> Result<()> {
                self.expr_visits += 1;
                Ok(())
            }
        }

        let mut step_visitor = StepVisitor {
            lit_visits: 0,
            ident_visits: 0,
            expr_visits: 0,
        };

        expr.visit(&mut step_visitor).unwrap();

        assert_eq!(step_visitor.lit_visits, 13);
        assert_eq!(step_visitor.ident_visits, 4);
        assert_eq!(step_visitor.expr_visits, 33);
    }
}
