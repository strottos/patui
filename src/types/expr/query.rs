use super::ast::*;
use super::visitor::Visitor;

use eyre::Result;

pub(crate) fn get_all_idents(expr: &PatuiExpr) -> Result<Vec<PatuiExpr>> {
    struct FullIdentsVisitor {
        idents: Vec<PatuiExpr>,
    }

    let mut visitor = FullIdentsVisitor { idents: Vec::new() };

    impl Visitor for FullIdentsVisitor {
        fn visit_expr(&mut self, expr: &PatuiExpr) -> Result<()> {
            match expr.kind() {
                ExprKind::Ident(_)
                | ExprKind::Field(_, _)
                | ExprKind::Index(_, _)
                | ExprKind::Call(_, _) => {
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
        let idents = get_all_idents(&expr).unwrap();

        assert_that!(idents).has_length(1);
        assert_that!(idents[0]).is_equal_to(PatuiExpr {
            raw: "foo".to_string(),
            kind: ExprKind::Ident(Ident {
                value: "foo".to_string(),
            }),
        });
    }

    #[traced_test]
    #[test]
    fn complex_ident() {
        let expr = "foo.bar[1].baz".try_into().unwrap();
        let idents = get_all_idents(&expr).unwrap();

        assert_that!(idents).has_length(4);
        assert_that!(idents[0]).is_equal_to(PatuiExpr {
            raw: "foo".to_string(),
            kind: ExprKind::Ident(Ident {
                value: "foo".to_string(),
            }),
        });
        assert_that!(idents[1]).is_equal_to(PatuiExpr {
            raw: "foo.bar".to_string(),
            kind: ExprKind::Field(
                P {
                    ptr: Box::new(PatuiExpr {
                        raw: "foo".to_string(),
                        kind: ExprKind::Ident(Ident {
                            value: "foo".to_string(),
                        }),
                    }),
                },
                Ident {
                    value: "bar".to_string(),
                },
            ),
        });
        assert_that!(idents[2]).is_equal_to(PatuiExpr {
            raw: "foo.bar[1]".to_string(),
            kind: ExprKind::Index(
                P {
                    ptr: Box::new(PatuiExpr {
                        raw: "foo.bar".to_string(),
                        kind: ExprKind::Field(
                            P {
                                ptr: Box::new(PatuiExpr {
                                    raw: "foo".to_string(),
                                    kind: ExprKind::Ident(Ident {
                                        value: "foo".to_string(),
                                    }),
                                }),
                            },
                            Ident {
                                value: "bar".to_string(),
                            },
                        ),
                    }),
                },
                P {
                    ptr: Box::new(PatuiExpr {
                        raw: "1".to_string(),
                        kind: ExprKind::Lit(Lit {
                            kind: LitKind::Integer("1".to_string()),
                        }),
                    }),
                },
            ),
        });
        assert_that!(idents[3]).is_equal_to(PatuiExpr {
            raw: "foo.bar[1].baz".to_string(),
            kind: ExprKind::Field(
                P {
                    ptr: Box::new(PatuiExpr {
                        raw: "foo.bar[1]".to_string(),
                        kind: ExprKind::Index(
                            P {
                                ptr: Box::new(PatuiExpr {
                                    raw: "foo.bar".to_string(),
                                    kind: ExprKind::Field(
                                        P {
                                            ptr: Box::new(PatuiExpr {
                                                raw: "foo".to_string(),
                                                kind: ExprKind::Ident(Ident {
                                                    value: "foo".to_string(),
                                                }),
                                            }),
                                        },
                                        Ident {
                                            value: "bar".to_string(),
                                        },
                                    ),
                                }),
                            },
                            P {
                                ptr: Box::new(PatuiExpr {
                                    raw: "1".to_string(),
                                    kind: ExprKind::Lit(Lit {
                                        kind: LitKind::Integer("1".to_string()),
                                    }),
                                }),
                            },
                        ),
                    }),
                },
                Ident {
                    value: "baz".to_string(),
                },
            ),
        });
    }

    #[traced_test]
    #[test]
    fn nested_idents() {
        let expr = "foo.bar[1].baz == 123 && foo[0] == baz().foo[0]"
            .try_into()
            .unwrap();
        let idents = get_all_idents(&expr).unwrap();

        assert_that!(idents).has_length(10);
        assert_that!(idents[3]).is_equal_to(PatuiExpr {
            raw: "foo.bar[1].baz".to_string(),
            kind: ExprKind::Field(
                P {
                    ptr: Box::new(PatuiExpr {
                        raw: "foo.bar[1]".to_string(),
                        kind: ExprKind::Index(
                            P {
                                ptr: Box::new(PatuiExpr {
                                    raw: "foo.bar".to_string(),
                                    kind: ExprKind::Field(
                                        P {
                                            ptr: Box::new(PatuiExpr {
                                                raw: "foo".to_string(),
                                                kind: ExprKind::Ident(Ident {
                                                    value: "foo".to_string(),
                                                }),
                                            }),
                                        },
                                        Ident {
                                            value: "bar".to_string(),
                                        },
                                    ),
                                }),
                            },
                            P {
                                ptr: Box::new(PatuiExpr {
                                    raw: "1".to_string(),
                                    kind: ExprKind::Lit(Lit {
                                        kind: LitKind::Integer("1".to_string()),
                                    }),
                                }),
                            },
                        ),
                    }),
                },
                Ident {
                    value: "baz".to_string(),
                },
            ),
        });

        assert_that!(idents[5]).is_equal_to(PatuiExpr {
            raw: "foo[0]".to_string(),
            kind: ExprKind::Index(
                P {
                    ptr: Box::new(PatuiExpr {
                        raw: "foo".to_string(),
                        kind: ExprKind::Ident(Ident {
                            value: "foo".to_string(),
                        }),
                    }),
                },
                P {
                    ptr: Box::new(PatuiExpr {
                        raw: "0".to_string(),
                        kind: ExprKind::Lit(Lit {
                            kind: LitKind::Integer("0".to_string()),
                        }),
                    }),
                },
            ),
        });

        assert_that!(idents[9]).is_equal_to(PatuiExpr {
            raw: "baz().foo[0]".to_string(),
            kind: ExprKind::Index(
                P {
                    ptr: Box::new(PatuiExpr {
                        raw: "baz().foo".to_string(),
                        kind: ExprKind::Field(
                            P {
                                ptr: Box::new(PatuiExpr {
                                    raw: "baz()".to_string(),
                                    kind: ExprKind::Call(
                                        P {
                                            ptr: Box::new(PatuiExpr {
                                                raw: "baz".to_string(),
                                                kind: ExprKind::Ident(Ident {
                                                    value: "baz".to_string(),
                                                }),
                                            }),
                                        },
                                        Vec::new(),
                                    ),
                                }),
                            },
                            Ident {
                                value: "foo".to_string(),
                            },
                        ),
                    }),
                },
                P {
                    ptr: Box::new(PatuiExpr {
                        raw: "0".to_string(),
                        kind: ExprKind::Lit(Lit {
                            kind: LitKind::Integer("0".to_string()),
                        }),
                    }),
                },
            ),
        });
    }
}
