use crate::api::common::{object_to_str, string_to_error_resp_json};
use crate::api::v2::transactions::{self, AvailSigner, Submit};
use crate::api::v2::types::Error;
use crate::data::{
	get_confidence_achieved_message_from_db, get_data_verified_message_from_db,
	get_header_verified_message_from_db,
};

use crate::light_client_commons::init_db;
use crate::network::rpc;
// use crate::rpc;
use crate::types::{AvailSecretKey, RuntimeConfig, State};

use std::sync::{Arc, Mutex};
use tracing::error;

use crate::api::v2::types::{Status, Transaction};

pub async unsafe fn submit_transaction(
	cfg: RuntimeConfig,
	app_id: u32,
	transaction: Transaction,
	private_key: String,
) -> String {
	let avail_secret = AvailSecretKey::try_from(private_key);
	let db = init_db(&cfg.clone().avail_path, true).unwrap();

	let state = Arc::new(Mutex::new(State::default()));
	let (rpc_client, _, _) = rpc::init(db, state, &cfg.full_node_ws);

	match avail_secret {
		Ok(avail_secret) => {
			let submitter = Arc::new(transactions::Submitter {
				node_client: rpc_client,
				app_id,
				pair_signer: Some(AvailSigner::from(avail_secret)),
			});
			let response = submitter.submit(transaction).await.map_err(|error| {
				error!(%error, "Submit transaction failed");

				Error::internal_server_error(error)
			});
			match response {
				Ok(response) => response.hash.to_string(),
				Err(err) => err.cause.unwrap().root_cause().to_string(),
			}
		},
		Err(_) => "Secret Key error".to_string(),
	}
}

pub async fn get_startus_v2(cfg: RuntimeConfig) -> String {
	let db = init_db(&cfg.clone().avail_path, true).unwrap();

	let state = Arc::new(Mutex::new(State::default()));
	let (rpc_client, _, _) = rpc::init(db.clone(), state, &cfg.full_node_ws);
	let node = rpc_client.get_connected_node().await.unwrap();

	let status = Status::new_from_db(&cfg, &node, db);
	return object_to_str(&status);
}

pub fn get_confidence_message_list(cfg: RuntimeConfig) -> String {
	let db = init_db(&cfg.clone().avail_path, true).unwrap();
	match get_confidence_achieved_message_from_db(db) {
		Ok(message_list_option) => match message_list_option {
			Some(message_list) => message_list,
			None => "{\'message_list\':[]}".to_string(),
		},
		Err(err) => string_to_error_resp_json(err.root_cause().to_string()),
	}
}

pub fn get_data_verified_message_list(cfg: RuntimeConfig) -> String {
	let db = init_db(&cfg.clone().avail_path, true).unwrap();
	match get_data_verified_message_from_db(db) {
		Ok(message_list_option) => match message_list_option {
			Some(message_list) => message_list,
			None => "{\'message_list\':[]}".to_string(),
		},
		Err(err) => string_to_error_resp_json(err.root_cause().to_string()),
	}
}
pub fn get_header_verified_message_list(cfg: RuntimeConfig) -> String {
	let db = init_db(&cfg.clone().avail_path, true).unwrap();
	match get_header_verified_message_from_db(db) {
		Ok(message_list_option) => match message_list_option {
			Some(message_list) => message_list,
			None => "{\'message_list\':[]}".to_string(),
		},
		Err(err) => string_to_error_resp_json(err.root_cause().to_string()),
	}
}
