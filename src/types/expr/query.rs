use super::ast::*;
use super::visitor::Visitor;

use eyre::Result;

pub(crate) fn get_all_terms(expr: &PatuiExpr) -> Result<Vec<Expr>> {
    struct FullIdentsVisitor {
        idents: Vec<Expr>,
    }

    let mut visitor = FullIdentsVisitor { idents: Vec::new() };

    impl Visitor for FullIdentsVisitor {
        fn visit_expr(&mut self, expr: &Expr) -> Result<()> {
            match expr.kind() {
                ExprKind::Term(_) => {
                    self.idents.push(expr.clone());
                }
                _ => {}
            }

            Ok(())
        }
    }

    expr.visit(&mut visitor)?;

    Ok(visitor.idents)
}

#[cfg(test)]
mod tests {
    use super::*;

    use assertor::*;
    use tracing_test::traced_test;

    #[traced_test]
    #[test]
    fn basic_ident() {
        let expr = "foo".try_into().unwrap();
        let idents = get_all_terms(&expr).unwrap();

        assert_that!(idents).has_length(1);
        assert_that!(idents[0]).is_equal_to(Expr {
            kind: ExprKind::Term(Term {
                values: vec![TermParts::Ident("foo".to_string())],
            }),
        });
    }

    #[traced_test]
    #[test]
    fn complex_ident() {
        let expr = "foo.bar[1].baz".try_into().unwrap();
        let idents = get_all_terms(&expr).unwrap();

        assert_that!(idents).has_length(1);
        assert_that!(idents[0]).is_equal_to(Expr {
            kind: ExprKind::Term(Term {
                values: vec![
                    TermParts::Ident("foo".to_string()),
                    TermParts::Ident("bar".to_string()),
                    TermParts::Index(1),
                    TermParts::Ident("baz".to_string()),
                ],
            }),
        });
    }

    #[traced_test]
    #[test]
    fn nested_idents() {
        let expr = "foo.bar[1].baz == 123 && foo[0] == baz().foo[0]"
            .try_into()
            .unwrap();
        let idents = get_all_terms(&expr).unwrap();

        assert_that!(idents).has_length(3);
        assert_that!(idents[0]).is_equal_to(Expr {
            kind: ExprKind::Term(Term {
                values: vec![
                    TermParts::Ident("foo".to_string()),
                    TermParts::Ident("bar".to_string()),
                    TermParts::Index(1),
                    TermParts::Ident("baz".to_string()),
                ],
            }),
        });

        assert_that!(idents[1]).is_equal_to(Expr {
            kind: ExprKind::Term(Term {
                values: vec![TermParts::Ident("foo".to_string()), TermParts::Index(0)],
            }),
        });

        assert_that!(idents[2]).is_equal_to(Expr {
            kind: ExprKind::Term(Term {
                values: vec![
                    TermParts::Ident("baz".to_string()),
                    TermParts::Call(vec![]),
                    TermParts::Ident("foo".to_string()),
                    TermParts::Index(0),
                ],
            }),
        });
    }
}
