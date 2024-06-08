use {
    solana_client::rpc_client::RpcClient,
    stake::{get_stake_for_vote_account, parse_args},
    std::sync::Arc,
};
mod stake;
fn main() {
    let cli_args = parse_args();
    let rpc_url = "https://mainnet.helius-rpc.com/?api-key=3ccd3ceb-7ef3-42e9-a155-708552f77a35";
    let rpc_client = Arc::new(RpcClient::new(rpc_url));

    get_stake_for_vote_account(rpc_client, cli_args)
}
