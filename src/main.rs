use clap::Parser;
use futures::FutureExt;
use miette::IntoDiagnostic;
use std::panic::AssertUnwindSafe;
use std::sync::Mutex;
use std::{collections::HashMap, sync::Arc};
use utxorpc::CardanoQueryClient;

mod oracle;
/// Custom parser for key=value pairs
fn parse_key_val(s: &str) -> Result<(String, String), String> {
    let pos = s
        .find('=')
        .ok_or_else(|| format!("invalid KEY=value: no `=` found in `{s}`"))?;
    Ok((s[..pos].to_string(), s[pos + 1..].to_string()))
}

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
struct Cli {
    /// The UTxO RPC endpoint to test
    #[arg(short, long, default_value = "http://localhost:50051")]
    subject: String,

    /// Metadata in key=value format to pass to the UTxO RPC client. Can be specified multiple times
    #[arg(short, long, value_parser = parse_key_val)]
    metadata: Vec<(String, String)>,
}

#[tokio::main]
async fn main() -> miette::Result<()> {
    let cli = Cli::parse();

    // Convert Vec of tuples to HashMap if needed
    let metadata: HashMap<String, String> = cli.metadata.into_iter().collect();

    let mut client = utxorpc::ClientBuilder::new()
        .uri(&cli.subject)
        .into_diagnostic()?;

    for (key, value) in metadata {
        client = client.metadata(key, value).into_diagnostic()?;
    }

    let mut client = client.build::<CardanoQueryClient>().await;

    let oracle =
        oracle::cardano::DBSyncOracle::new("postgresql://dbsync1uyuucx0e6ax6zl0m7mx:OLr9qvod0XI@dbsync-v3.demeter.run:5432/dbsync-preview").await?;

    oracle::cardano::test_params_match(oracle, &mut client).await?;

    Ok(())
}
