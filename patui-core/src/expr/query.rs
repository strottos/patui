use thiserror::Error;

use crate::expr::visitor::PatuiExprVisitorError;

use super::{ast::TermPart, visitor::Visitor, PatuiExpr};

/// Error type for querying a `PatuiExpr`.
#[derive(Error, Debug)]
pub enum PatuiExprQueryError {
    /// Error while visiting the `PatuiExpr`.
    #[error("Error while visiting the `PatuiExpr`: {0}")]
    Visitor(#[from] PatuiExprVisitorError),
}

/// Given a `PatuiExpr`, get a list of every terms in the expression. Used to be able to figure
/// things out about an expression like what results does it depend on.
pub fn get_terms(expr: &PatuiExpr) -> Result<Vec<Vec<TermPart>>, PatuiExprQueryError> {
    struct TermsVisitor {
        terms: Vec<Vec<TermPart>>,
    }

    let mut visitor = TermsVisitor { terms: Vec::new() };

    impl Visitor for TermsVisitor {
        fn visit_term(&mut self, term: &Vec<TermPart>) -> Result<(), PatuiExprVisitorError> {
            self.terms.push(term.clone());

            Ok(())
        }
    }

    expr.visit(&mut visitor)?;

    Ok(visitor.terms)
}

#[cfg(test)]
mod tests {
    use crate::expr::ast::{Expr, Lit, TermPart};

    use super::*;

    use assertor::*;
    use tracing_test::traced_test;

    #[traced_test]
    #[test]
    fn basic_ident() {
        let expr = "foo".try_into().unwrap();
        let terms = get_terms(&expr).unwrap();

        assert_that!(terms).has_length(1);
        assert_that!(terms[0]).is_equal_to(vec![TermPart::Ident("foo".to_string())]);
    }

    #[traced_test]
    #[test]
    fn complex_ident() {
        let expr = "foo.bar[1].baz".try_into().unwrap();
        let terms = get_terms(&expr).unwrap();

        assert_that!(terms).has_length(2);
        assert_that!(terms[0]).is_equal_to(vec![TermPart::Lit(Lit::Integer(1))]);
        assert_that!(terms[1]).is_equal_to(vec![
            TermPart::Ident("foo".to_string()),
            TermPart::Ident("bar".to_string()),
            TermPart::Index(Box::new(Expr::Term(vec![TermPart::Lit(Lit::Integer(1))]))),
            TermPart::Ident("baz".to_string()),
        ]);
    }

    #[traced_test]
    #[test]
    fn nested_idents() {
        let expr = "foo.bar[1].baz == 123 && foo[0] == baz(\"foo\").foo[0]"
            .try_into()
            .unwrap();
        let terms = get_terms(&expr).unwrap();

        assert_that!(terms).has_length(8);
        assert_that!(terms[0]).is_equal_to(vec![TermPart::Lit(Lit::Integer(1))]);
        assert_that!(terms[1]).is_equal_to(vec![
            TermPart::Ident("foo".to_string()),
            TermPart::Ident("bar".to_string()),
            TermPart::Index(Box::new(Expr::Term(vec![TermPart::Lit(Lit::Integer(1))]))),
            TermPart::Ident("baz".to_string()),
        ]);
        assert_that!(terms[2]).is_equal_to(vec![TermPart::Lit(Lit::Integer(123))]);
        assert_that!(terms[3]).is_equal_to(vec![TermPart::Lit(Lit::Integer(0))]);
        assert_that!(terms[4]).is_equal_to(vec![
            TermPart::Ident("foo".to_string()),
            TermPart::Index(Box::new(Expr::Term(vec![TermPart::Lit(Lit::Integer(0))]))),
        ]);
        assert_that!(terms[5]).is_equal_to(vec![TermPart::Lit(Lit::String("foo".to_string()))]);
        assert_that!(terms[6]).is_equal_to(vec![TermPart::Lit(Lit::Integer(0))]);
        assert_that!(terms[7]).is_equal_to(vec![
            TermPart::Call(
                "baz".to_string(),
                vec![Expr::Term(vec![TermPart::Lit(Lit::String(
                    "foo".to_string(),
                ))])],
            ),
            TermPart::Ident("foo".to_string()),
            TermPart::Index(Box::new(Expr::Term(vec![TermPart::Lit(Lit::Integer(0))]))),
        ]);
    }
}
