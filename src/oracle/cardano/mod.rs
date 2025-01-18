use std::sync::Arc;
use std::sync::Mutex;

use miette::IntoDiagnostic as _;
use utxorpc::CardanoQueryClient;

mod dbsync;

pub use dbsync::*;

pub trait CardanoOracle {
    async fn params(&self, epoch: u64) -> miette::Result<utxorpc::spec::cardano::PParams>;
}

pub async fn test_params_match(
    oracle: impl CardanoOracle,
    subject: &mut CardanoQueryClient,
) -> miette::Result<()> {
    let epoch = 816; // TODO: make this a query to the subject

    let expected = oracle.params(epoch).await?;

    let expected = serde_json::to_value(expected).into_diagnostic()?;

    let obtained = subject
        .read_params(utxorpc::spec::query::ReadParamsRequest { field_mask: None })
        .await
        .into_diagnostic()?;

    let obtained = obtained
        .into_inner()
        .values
        .ok_or_else(|| miette::miette!("No params found"))?
        .params
        .ok_or_else(|| miette::miette!("No params found"))?;

    let obtained = match obtained {
        utxorpc::spec::query::any_chain_params::Params::Cardano(obtained) => obtained,
        _ => panic!("Expected Cardano params"),
    };

    let obtained = serde_json::to_value(obtained).unwrap();

    assert_json_diff::assert_json_matches_no_panic(
        &obtained,
        &expected,
        assert_json_diff::Config::new(assert_json_diff::CompareMode::Strict),
    )
    .map_err(|e| miette::miette!("Params do not match: {}", e))?;

    Ok(())
}
