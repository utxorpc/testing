use std::fmt::Display;

use miette::Diagnostic;
use thiserror::Error;

#[derive(Debug, Clone, Error, Diagnostic)]
#[error("Field {field} mismatch, expected {expected}, obtained {obtained}")]
pub struct FieldError {
    pub field: String,
    pub expected: String,
    pub obtained: String,
}

#[derive(Debug, Clone, Error, Diagnostic)]
#[error("Compare error")]
pub struct CompareError {
    #[related(nested)]
    pub errors: Vec<FieldError>,
}

fn eval_eq<T: PartialEq + std::fmt::Debug>(
    key: &str,
    expected: &T,
    obtained: &T,
) -> Result<(), FieldError> {
    if expected == obtained {
        Ok(())
    } else {
        Err(FieldError {
            field: key.to_string(),
            expected: format!("{:?}", expected),
            obtained: format!("{:?}", obtained),
        })
    }
}

fn eval_rational(
    key: &str,
    expected: &utxorpc::spec::cardano::RationalNumber,
    obtained: &utxorpc::spec::cardano::RationalNumber,
) -> Result<(), FieldError> {
    let expected_val = (expected.numerator as i32) / (expected.denominator as i32);
    let obtained_val = (obtained.numerator as i32) / (obtained.denominator as i32);

    if expected_val == obtained_val {
        Ok(())
    } else {
        Err(FieldError {
            field: key.to_string(),
            expected: format!("{}/{}", expected.numerator, expected.denominator),
            obtained: format!("{}/{}", obtained.numerator, obtained.denominator),
        })
    }
}

macro_rules! report_eq {
    ($expected:expr, $obtained:expr, $errors:expr, $field:ident) => {
        if let Err(e) = eval_eq(&stringify!($field), &$expected.$field, &$obtained.$field) {
            $errors.push(e);
        }
    };
}

macro_rules! report_optional {
    ($expected:expr, $obtained:expr, $errors:expr, $field:ident, ($a:ident, $b:ident) => $sub_expr:expr) => {
        match (&$expected.$field, &$obtained.$field) {
            (Some(expected), Some(obtained)) => {
                let $a = expected;
                let $b = obtained;
                $sub_expr;
            }
            (None, None) => {}
            _ => {
                $errors.push(FieldError {
                    field: stringify!($field).to_string(),
                    expected: format!("{:?}", $expected.$field),
                    obtained: format!("{:?}", $obtained.$field),
                });
            }
        }
    };
}

macro_rules! report_rational {
    ($expected:expr, $obtained:expr, $errors:expr) => {
        if let Err(e) = eval_rational(&stringify!($expected), &$expected, &$obtained) {
            $errors.push(e);
        }
    };
    ($expected:expr, $obtained:expr, $errors:expr, $field:ident) => {
        if let Err(e) = eval_rational(&stringify!($field), &$expected.$field, &$obtained.$field) {
            $errors.push(e);
        }
    };
}

pub fn compare_params(
    e: &utxorpc::spec::cardano::PParams,
    o: &utxorpc::spec::cardano::PParams,
) -> Result<(), CompareError> {
    let mut errs = Vec::new();

    report_eq!(e, o, errs, coins_per_utxo_byte);
    report_eq!(e, o, errs, max_tx_size);
    report_eq!(e, o, errs, min_fee_coefficient);
    report_eq!(e, o, errs, min_fee_constant);
    report_eq!(e, o, errs, max_block_body_size);
    report_eq!(e, o, errs, max_block_header_size);
    report_eq!(e, o, errs, stake_key_deposit);
    report_eq!(e, o, errs, pool_deposit);
    report_eq!(e, o, errs, pool_retirement_epoch_bound);
    report_eq!(e, o, errs, desired_number_of_pools);
    report_optional!(e, o, errs, pool_influence, (a, b) => report_rational!(a, b, errs));
    report_optional!(e, o, errs, monetary_expansion, (a, b) => report_rational!(a, b, errs));
    report_optional!(e, o, errs, treasury_expansion, (a, b) => report_rational!(a, b, errs));
    report_eq!(e, o, errs, min_pool_cost);
    report_eq!(e, o, errs, protocol_version);
    report_eq!(e, o, errs, max_value_size);
    report_eq!(e, o, errs, collateral_percentage);
    report_eq!(e, o, errs, max_collateral_inputs);

    report_optional!(e, o, errs, cost_models, (a, b) => {
        report_eq!(a, b, errs, plutus_v1);
        report_eq!(a, b, errs, plutus_v2);
        report_eq!(a, b, errs, plutus_v3);
    });

    report_optional!(e, o, errs, prices, (a, b) => {
        report_optional!(a, b, errs, memory, (a, b) => report_rational!(a, b, errs));
        report_optional!(a, b, errs, steps, (a, b) => report_rational!(a, b, errs));
    });

    report_eq!(e, o, errs, max_execution_units_per_transaction);
    report_eq!(e, o, errs, max_execution_units_per_block);
    report_eq!(e, o, errs, min_fee_script_ref_cost_per_byte);
    report_eq!(e, o, errs, pool_voting_thresholds);
    report_eq!(e, o, errs, drep_voting_thresholds);
    report_eq!(e, o, errs, min_committee_size);
    report_eq!(e, o, errs, committee_term_limit);
    report_eq!(e, o, errs, governance_action_validity_period);
    report_eq!(e, o, errs, governance_action_deposit);
    report_eq!(e, o, errs, drep_deposit);
    report_eq!(e, o, errs, drep_inactivity_period);

    if !errs.is_empty() {
        return Err(CompareError { errors: errs });
    }

    Ok(())
}
