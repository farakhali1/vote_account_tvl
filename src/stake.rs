use {
    log::{error, info},
    serde::{Deserialize, Serialize},
    solana_account_decoder::UiAccountEncoding,
    solana_client::{
        rpc_client::RpcClient,
        rpc_config::{RpcAccountInfoConfig, RpcProgramAccountsConfig},
        rpc_filter::{Memcmp, RpcFilterType},
    },
    solana_sdk::{
        account::Account,
        account_utils::StateMut,
        pubkey::Pubkey,
        stake::{self, state::StakeStateV2},
    },
    std::{collections::HashMap, env, fs::File, str::FromStr, sync::Arc},
};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidatorStakeForEpoch {
    pub total_stake: u128,
    pub epoch: u64,
}

pub const STAKE_PROGRAM_ID: &Pubkey = &stake::program::id();
pub const RPC_CALL_MAX_RETRIES: u64 = 5;

pub struct CLIARGS {
    pub validator_vote_account: Pubkey,
    pub epoch: Option<u64>,
}

pub fn parse_args() -> CLIARGS {
    let args: Vec<String> = env::args().collect();

    if args.len() < 2 || args.len() > 3 {
        eprintln!("Usage: {} <validator_vote_account> [epoch]", args[0]);
        std::process::exit(1);
    }

    let validator_vote_account = Pubkey::from_str(args[1].clone().as_str()).unwrap();

    let epoch = if args.len() == 3 {
        Some(args[2].parse().expect("Epoch must be a number"))
    } else {
        None
    };

    CLIARGS {
        validator_vote_account,
        epoch,
    }
}

pub fn get_stake_for_vote_account(rpc_client: Arc<RpcClient>, cli_args: CLIARGS) {
    let all_stake_accounts_info =
        get_vote_stake_accounts(rpc_client.clone(), &cli_args.validator_vote_account);
    if all_stake_accounts_info.is_some() {
        if cli_args.epoch.is_some() {
            let total_stake: u128 = process_stake_accounts_info_for_total_stake(
                cli_args.epoch.unwrap(),
                &all_stake_accounts_info.unwrap(),
            );
            println!("{:?}", total_stake);
        } else {
            let stake_account: Option<Pubkey> = process_stake_accounts_info_for_staker(
                rpc_client.get_epoch_info().unwrap().epoch,
                &all_stake_accounts_info.unwrap(),
            );
            println!("{:?}", stake_account.unwrap());
        }
    } else {
        error!("unable to get all stakers info");
    }
}

pub fn get_vote_stake_accounts(
    rpc_client: Arc<RpcClient>,
    vote_account_pubkey: &Pubkey,
) -> Option<Vec<(Pubkey, Account)>> {
    let program_accounts_config = RpcProgramAccountsConfig {
        account_config: RpcAccountInfoConfig {
            encoding: Some(UiAccountEncoding::Base64),
            ..RpcAccountInfoConfig::default()
        },
        filters: Some(vec![
            RpcFilterType::Memcmp(Memcmp::new_base58_encoded(0, &[2, 0, 0, 0])),
            RpcFilterType::Memcmp(Memcmp::new_base58_encoded(
                124,
                vote_account_pubkey.as_ref(),
            )),
        ]),
        ..RpcProgramAccountsConfig::default()
    };

    let mut retries = 0;
    loop {
        let result = rpc_client
            .get_program_accounts_with_config(STAKE_PROGRAM_ID, program_accounts_config.clone());

        if result.is_ok() {
            return Some(result.unwrap());
        } else {
            error!(
                "Error in get_program_accounts_with_config RPC call attempt {}",
                retries + 1
            );
            retries += 1;
            if retries >= RPC_CALL_MAX_RETRIES {
                return None;
            }
        }
    }
}

pub fn process_stake_accounts_info_for_total_stake(
    epoch: u64,
    all_stake_accounts_info: &[(Pubkey, Account)],
) -> u128 {
    let mut total_stake: u128 = 0;
    for (_stake_pubkey, stake_account) in all_stake_accounts_info {
        if let Ok(stake_state) = stake_account.state() {
            if let StakeStateV2::Stake(_, stake, _) = stake_state {
                if stake.delegation.activation_epoch < epoch
                    && stake.delegation.deactivation_epoch > epoch
                {
                    total_stake += stake.delegation.stake as u128;
                }
            }
        }
    }
    // save_stake_info_to_file(cli_args, total_stake);
    total_stake
}

pub fn process_stake_accounts_info_for_staker(
    epoch: u64,
    all_stake_accounts_info: &[(Pubkey, Account)],
) -> Option<Pubkey> {
    let mut epoch_to_pubkey_map: HashMap<u64, Pubkey> = HashMap::new();

    for (stake_pubkey, stake_account) in all_stake_accounts_info {
        if let Ok(stake_state) = stake_account.state() {
            if let StakeStateV2::Stake(_, stake, _) = stake_state {
                let activation_epoch = stake.delegation.activation_epoch;
                let deactivation_epoch = stake.delegation.deactivation_epoch;
                if deactivation_epoch > epoch && stake.delegation.stake > 1000000000 {
                    epoch_to_pubkey_map.insert(activation_epoch, *stake_pubkey);
                }
            }
        }
    }

    if let Some(&pubkey) = epoch_to_pubkey_map.get(&(epoch - 11)) {
        return Some(pubkey);
    }

    let mut epochs: Vec<(&u64, &Pubkey)> = epoch_to_pubkey_map.iter().collect();
    println!("{:?}", epoch_to_pubkey_map);
    epochs.sort_by_key(|&(epoch, _)| *epoch);
    println!("{:?}", epochs);
    epochs.first().map(|&(_, pubkey)| *pubkey)
}

pub fn save_stake_info_to_file(epoch: u64, total_stake: u128) {
    let file_name = format!("{}.json", epoch);
    let file = File::create(file_name).unwrap();
    let file_data = ValidatorStakeForEpoch {
        epoch: epoch,
        total_stake: total_stake,
    };
    serde_json::to_writer(file, &file_data).unwrap();
    info!("stake info updated to file successfully");
}
