use miette::IntoDiagnostic as _;
use utxorpc::CardanoQueryClient;

mod compare;
mod dbsync;

pub use dbsync::*;

pub trait CardanoOracle {
    async fn params(&self, epoch: u64) -> miette::Result<utxorpc::spec::cardano::PParams>;
}

pub async fn test_params_match(
    oracle: impl CardanoOracle,
    subject: &mut CardanoQueryClient,
) -> miette::Result<()> {
    let epoch = 792; // TODO: make this a query to the subject

    let expected = oracle.params(epoch).await?;

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
    };

    compare::compare_params(&expected, &obtained)?;

    Ok(())
}
