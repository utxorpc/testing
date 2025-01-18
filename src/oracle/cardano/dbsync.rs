use miette::IntoDiagnostic as _;

pub struct DBSyncOracle {
    pool: sqlx::PgPool,
}

impl DBSyncOracle {
    pub async fn new(database_url: &str) -> miette::Result<Self> {
        let pool = sqlx::PgPool::connect(database_url)
            .await
            .into_diagnostic()?;

        Ok(Self { pool })
    }
}

impl super::CardanoOracle for DBSyncOracle {
    async fn params(&self, epoch: u64) -> miette::Result<utxorpc::spec::cardano::PParams> {
        // Query the latest epoch parameters from db-sync
        let params = sqlx::query!(
            r#"
            SELECT
                min_fee_a,
                min_fee_b,
                max_block_size,
                max_tx_size,
                max_bh_size,
                key_deposit,
                pool_deposit,
                max_epoch,
                optimal_pool_count,
                influence,
                monetary_expand_rate,
                treasury_growth_rate,
                decentralisation,
                protocol_major,
                protocol_minor,
                min_utxo_value,
                min_pool_cost,
                nonce,
                price_mem,
                price_step,
                max_tx_ex_mem,
                max_tx_ex_steps,
                max_block_ex_mem,
                max_block_ex_steps,
                max_val_size,
                collateral_percent,
                max_collateral_inputs,
                block_id,
                extra_entropy,
                coins_per_utxo_size,
                pvt_motion_no_confidence,
                pvt_committee_normal,
                pvt_committee_no_confidence,
                pvt_hard_fork_initiation,
                dvt_motion_no_confidence,
                dvt_committee_normal,
                dvt_committee_no_confidence,
                dvt_update_to_constitution,
                dvt_hard_fork_initiation,
                dvt_p_p_network_group,
                dvt_p_p_economic_group,
                dvt_p_p_technical_group,
                dvt_p_p_gov_group,
                dvt_treasury_withdrawal,
                committee_min_size,
                committee_max_term_length,
                gov_action_lifetime,
                gov_action_deposit,
                drep_deposit,
                drep_activity,
                pvtpp_security_group,
                min_fee_ref_script_cost_per_byte,
                costs
            FROM public.epoch_param
            JOIN public.cost_model ON epoch_param.cost_model_id = cost_model.id
            WHERE epoch_no = $1;
            "#,
            epoch as i64
        )
        .fetch_one(&self.pool)
        .await
        .into_diagnostic()?;

        // Parse the costs JSON into cost models

        let cost_models = utxorpc::spec::cardano::CostModels {
            plutus_v1: params
                .costs
                .get("PlutusV1")
                .and_then(|v| v.as_array())
                .map(|arr| utxorpc::spec::cardano::CostModel {
                    values: arr
                        .iter()
                        .map(|v| v.as_i64().unwrap_or_default() as i64)
                        .collect(),
                }),
            plutus_v2: params
                .costs
                .get("PlutusV2")
                .and_then(|v| v.as_array())
                .map(|arr| utxorpc::spec::cardano::CostModel {
                    values: arr
                        .iter()
                        .map(|v| v.as_i64().unwrap_or_default() as i64)
                        .collect(),
                }),
            plutus_v3: params
                .costs
                .get("PlutusV3")
                .and_then(|v| v.as_array())
                .map(|arr| utxorpc::spec::cardano::CostModel {
                    values: arr
                        .iter()
                        .map(|v| v.as_i64().unwrap_or_default() as i64)
                        .collect(),
                }),
        };

        // Prices are derived from the params query
        let prices = utxorpc::spec::cardano::ExPrices {
            memory: Some(utxorpc::spec::cardano::RationalNumber {
                numerator: params.price_mem.unwrap_or_default() as i32,
                denominator: 1,
            }),
            steps: Some(utxorpc::spec::cardano::RationalNumber {
                numerator: params.price_step.unwrap_or_default() as i32,
                denominator: 1,
            }),
        };

        Ok(utxorpc::spec::cardano::PParams {
            coins_per_utxo_byte: params
                .coins_per_utxo_size
                .unwrap_or_default()
                .to_string()
                .parse::<u64>()
                .unwrap_or_default(),
            max_tx_size: params.max_tx_size as u64,
            min_fee_coefficient: params.min_fee_a as u64,
            min_fee_constant: params.min_fee_b as u64,
            max_block_body_size: params.max_block_size as u64,
            max_block_header_size: params.max_bh_size as u64,
            stake_key_deposit: params
                .key_deposit
                .to_string()
                .parse::<u64>()
                .unwrap_or_default(),
            pool_deposit: params
                .pool_deposit
                .to_string()
                .parse::<u64>()
                .unwrap_or_default(),
            pool_retirement_epoch_bound: params.max_epoch as u64,
            desired_number_of_pools: params.optimal_pool_count as u64,
            pool_influence: Some(utxorpc::spec::cardano::RationalNumber {
                numerator: (params.influence * 1_000_000.0) as i32,
                denominator: 1_000_000,
            }),
            monetary_expansion: Some(utxorpc::spec::cardano::RationalNumber {
                numerator: (params.monetary_expand_rate * 1_000_000.0) as i32,
                denominator: 1_000_000,
            }),
            treasury_expansion: Some(utxorpc::spec::cardano::RationalNumber {
                numerator: (params.treasury_growth_rate * 1_000_000.0) as i32,
                denominator: 1_000_000,
            }),
            min_pool_cost: params
                .min_pool_cost
                .to_string()
                .parse::<u64>()
                .unwrap_or_default(),
            protocol_version: Some(utxorpc::spec::cardano::ProtocolVersion {
                major: params.protocol_major as u32,
                minor: params.protocol_minor as u32,
            }),
            max_value_size: params
                .max_val_size
                .unwrap_or_default()
                .to_string()
                .parse::<u64>()
                .unwrap_or_default(),
            collateral_percentage: params.collateral_percent.unwrap_or_default() as u64,
            max_collateral_inputs: params.max_collateral_inputs.unwrap_or_default() as u64,
            cost_models: Some(cost_models),
            prices: Some(prices),
            max_execution_units_per_transaction: Some(utxorpc::spec::cardano::ExUnits {
                memory: params
                    .max_tx_ex_mem
                    .unwrap_or_default()
                    .to_string()
                    .parse::<u64>()
                    .unwrap_or_default(),
                steps: params
                    .max_tx_ex_steps
                    .unwrap_or_default()
                    .to_string()
                    .parse::<u64>()
                    .unwrap_or_default(),
            }),
            max_execution_units_per_block: Some(utxorpc::spec::cardano::ExUnits {
                memory: params
                    .max_block_ex_mem
                    .unwrap_or_default()
                    .to_string()
                    .parse::<u64>()
                    .unwrap_or_default(),
                steps: params
                    .max_block_ex_steps
                    .unwrap_or_default()
                    .to_string()
                    .parse::<u64>()
                    .unwrap_or_default(),
            }),
            min_fee_script_ref_cost_per_byte: params.min_fee_ref_script_cost_per_byte.map(|v| {
                utxorpc::spec::cardano::RationalNumber {
                    numerator: (v * 1_000_000.0) as i32,
                    denominator: 1_000_000,
                }
            }),
            pool_voting_thresholds: Some(utxorpc::spec::cardano::VotingThresholds {
                thresholds: vec![
                    utxorpc::spec::cardano::RationalNumber {
                        numerator: params.pvt_motion_no_confidence.unwrap_or_default() as i32,
                        denominator: 1,
                    },
                    utxorpc::spec::cardano::RationalNumber {
                        numerator: params.pvt_committee_normal.unwrap_or_default() as i32,
                        denominator: 1,
                    },
                    utxorpc::spec::cardano::RationalNumber {
                        numerator: params.pvt_committee_no_confidence.unwrap_or_default() as i32,
                        denominator: 1,
                    },
                    utxorpc::spec::cardano::RationalNumber {
                        numerator: params.pvt_hard_fork_initiation.unwrap_or_default() as i32,
                        denominator: 1,
                    },
                    utxorpc::spec::cardano::RationalNumber {
                        numerator: params.pvtpp_security_group.unwrap_or_default() as i32,
                        denominator: 1,
                    },
                ],
            }),
            drep_voting_thresholds: Some(utxorpc::spec::cardano::VotingThresholds {
                thresholds: vec![
                    utxorpc::spec::cardano::RationalNumber {
                        numerator: params.dvt_motion_no_confidence.unwrap_or_default() as i32,
                        denominator: 1,
                    },
                    utxorpc::spec::cardano::RationalNumber {
                        numerator: params.dvt_committee_normal.unwrap_or_default() as i32,
                        denominator: 1,
                    },
                    utxorpc::spec::cardano::RationalNumber {
                        numerator: params.dvt_committee_no_confidence.unwrap_or_default() as i32,
                        denominator: 1,
                    },
                    utxorpc::spec::cardano::RationalNumber {
                        numerator: params.dvt_hard_fork_initiation.unwrap_or_default() as i32,
                        denominator: 1,
                    },
                    utxorpc::spec::cardano::RationalNumber {
                        numerator: params.dvt_p_p_gov_group.unwrap_or_default() as i32,
                        denominator: 1,
                    },
                ],
            }),
            min_committee_size: params
                .committee_min_size
                .unwrap_or_default()
                .to_string()
                .parse::<u32>()
                .unwrap_or_default(),
            committee_term_limit: params
                .committee_max_term_length
                .unwrap_or_default()
                .to_string()
                .parse::<u64>()
                .unwrap_or_default(),
            governance_action_validity_period: params
                .gov_action_lifetime
                .unwrap_or_default()
                .to_string()
                .parse::<u64>()
                .unwrap_or_default(),
            governance_action_deposit: params
                .gov_action_deposit
                .unwrap_or_default()
                .to_string()
                .parse::<u64>()
                .unwrap_or_default(),
            drep_deposit: params
                .drep_deposit
                .unwrap_or_default()
                .to_string()
                .parse::<u64>()
                .unwrap_or_default(),
            drep_inactivity_period: params
                .drep_activity
                .unwrap_or_default()
                .to_string()
                .parse::<u64>()
                .unwrap_or_default(),
        })
    }
}
