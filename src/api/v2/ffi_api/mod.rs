use super::transactions::{self, AvailSigner, Submit};
use super::{super::common::str_ptr_to_config, types::PublishMessage};
use crate::api::v2::types::Error;
use crate::consts::EXPECTED_NETWORK_VERSION;
use crate::light_client_commons::run;
use crate::light_client_commons::FfiCallback;
use crate::rpc;
use crate::types::AvailSecretKey;
use anyhow::anyhow;
use std::ffi::CString;
use std::fmt::Display;
use std::sync::Arc;
use tokio::sync::broadcast;
use tokio::sync::mpsc::channel;
use tracing::error;

use super::types::{Topic, Transaction};

#[allow(non_snake_case)]
#[no_mangle]
#[tokio::main]
pub async unsafe extern "C" fn start_light_node_with_callbacks(
	cfg: *mut u8,
	ffi_callback: *mut FfiCallback,
) -> bool {
	let cfg = str_ptr_to_config(cfg);

	let (error_sender, mut error_receiver) = channel::<anyhow::Error>(1);

	let res = run(error_sender, cfg, false, true, false, Some(ffi_callback)).await;

	if let Err(error) = res {
		error!("{error}");
	} else {
		return true;
	};

	let error = match error_receiver.recv().await {
		Some(error) => error,
		None => anyhow!("Failed to receive error message"),
	};
	error!("Error: {}", error);
	return false;
}

pub async fn call_callbacks<T: Clone + TryInto<PublishMessage>>(
	topic: Topic,
	mut receiver: broadcast::Receiver<T>,
	callback: FfiCallback,
) where
	<T as TryInto<PublishMessage>>::Error: Display,
{
	loop {
		let message = match receiver.recv().await {
			Ok(value) => value,
			Err(error) => {
				error!(?topic, "Cannot receive message: {error}");
				return;
			},
		};
		let message: PublishMessage = match message.try_into() {
			Ok(message) => message,
			Err(error) => {
				error!(?topic, "Cannot create message: {error}");
				continue;
			},
		};

		let json_message = match serde_json::to_string(&message) {
			Ok(json_message) => json_message,
			Err(error) => {
				error!(?topic, "Cannot create message: {error}");
				continue;
			},
		};
		let json_topic = match serde_json::to_string(&topic) {
			Ok(json_topic) => json_topic,
			Err(error) => {
				error!(?topic, "Cannot create message: {error}");
				continue;
			},
		};
		let topic_ptr = json_topic.as_ptr();
		let message_ptr = json_message.as_ptr();
		callback(topic_ptr, message_ptr);
	}
}
#[allow(non_snake_case)]
#[no_mangle]
#[tokio::main]
pub async fn submit_transaction(
	private_key: *mut i8,
	app_id: u32,
	cfg: *mut u8,
	transaction: *mut i8,
) -> *const u8 {
	let cfg = str_ptr_to_config(cfg);
	let c_str_trx = unsafe { CString::from_raw(transaction).to_str().unwrap() };
	let transaction: Transaction = serde_json::from_str(c_str_trx).unwrap();
	let private_key: CString = unsafe { CString::from_raw(private_key) };
	let avail_secret = AvailSecretKey::try_from(private_key.to_str().unwrap().to_owned());
	let rpc_client_result =
		rpc::connect_to_the_full_node(&cfg.full_node_ws, None, EXPECTED_NETWORK_VERSION).await;
	let rpc_client = rpc_client_result.unwrap().0;
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
				Ok(response) => response.hash.as_ptr(),
				Err(err) => err.message.as_ptr(),
			}
		},
		Err(err) => "Secret Key error".as_ptr(),
	}
}