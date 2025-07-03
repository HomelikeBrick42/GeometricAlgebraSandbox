use crate::{
    Variable,
    multivector::Multivector,
    parsing::{AstExpression, AstExpressionKind, BinaryOperator, UnaryOperator},
};
use std::collections::BTreeMap;

pub fn evaluate_expression(
    expression: &AstExpression,
    variables: &BTreeMap<String, Variable>,
) -> Result<Multivector, String> {
    Ok(match expression.kind {
        AstExpressionKind::Name {
            name,
            ref name_token,
        } => match variables.get(name) {
            Some(variable) => variable.value,
            None => {
                return Err(format!(
                    "{}: Unknown variable '{name}'",
                    name_token.location
                ));
            }
        },
        AstExpressionKind::Number {
            number,
            number_token: _,
        } => Multivector {
            s: number,
            ..Multivector::ZERO
        },
        AstExpressionKind::Unary {
            ref operator,
            operator_token: _,
            ref operand,
        } => {
            let operand = evaluate_expression(operand, variables)?;
            match operator {
                UnaryOperator::Negate => -operand,
                UnaryOperator::Dual => operand.dual(),
                UnaryOperator::Reverse => operand.reverse(),
                UnaryOperator::Normalise => operand.normalised(),
                UnaryOperator::Magnitude => Multivector {
                    s: operand.magnitude(),
                    ..Multivector::ZERO
                },
                UnaryOperator::Sin => Multivector {
                    s: f32::sin(operand.s),
                    ..Multivector::ZERO
                },
                UnaryOperator::Cos => Multivector {
                    s: f32::cos(operand.s),
                    ..Multivector::ZERO
                },
                UnaryOperator::ASin => Multivector {
                    s: f32::asin(operand.s),
                    ..Multivector::ZERO
                },
                UnaryOperator::ACos => Multivector {
                    s: f32::acos(operand.s),
                    ..Multivector::ZERO
                },
            }
        }
        AstExpressionKind::Binary {
            ref left,
            ref operator,
            ref operator_token,
            ref right,
        } => {
            let left = evaluate_expression(left, variables)?;
            let right = evaluate_expression(right, variables)?;
            match operator {
                BinaryOperator::Add => left + right,
                BinaryOperator::Subtract => left - right,
                BinaryOperator::Multiply => left * right,
                BinaryOperator::Divide => {
                    return Err(format!("{}: Divide unimplemented", operator_token.location));
                }
                BinaryOperator::Wedge => left.wedge(right),
                BinaryOperator::Inner => left.inner(right),
                BinaryOperator::Regressive => left.regressive(right),
            }
        }
    })
}
