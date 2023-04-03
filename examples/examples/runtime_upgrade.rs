/*
	Copyright 2019 Supercomputing Systems AG
	Licensed under the Apache License, Version 2.0 (the "License");
	you may not use this file except in compliance with the License.
	You may obtain a copy of the License at

		http://www.apache.org/licenses/LICENSE-2.0

	Unless required by applicable law or agreed to in writing, software
	distributed under the License is distributed on an "AS IS" BASIS,
	WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
	See the License for the specific language governing permissions and
	limitations under the License.
*/

//! Example that shows how to subscribe to events and do some action
//! upon encountering them.

use log::debug;
use serde::de::DeserializeOwned;
use sp_core::H256 as Hash;
use substrate_api_client::{
	ac_primitives::PlainTipExtrinsicParams, api_client::UpdateRuntime, rpc::JsonrpseeClient, Api,
	SubscribeEvents,
};

// This module depends on the specific node runtime.
// Replace this crate by your own if you run a custom substrate node to get your custom events.
use codec::Decode;
use kitchensink_runtime::{Runtime, RuntimeEvent};
use std::{
	sync::{Arc, Mutex},
	thread,
};
use substrate_api_client::rpc::Subscribe;

fn auto_runtime_upgrade<Api, Client, Hash>(api_clone: Arc<Mutex<Api>>) -> thread::JoinHandle<()>
where
	Api: SubscribeEvents<Client, Hash> + UpdateRuntime + Send + 'static,
	Client: Subscribe,
	Hash: DeserializeOwned + Decode,
{
	let receiver = thread::spawn(move || {
		let mut subscription = {
			println!("Thread lock try");
			let lock = api_clone.lock().unwrap();
			lock.subscribe_events().unwrap()
		};
		for _ in 0..5 {
			let event_records = subscription.next_event::<RuntimeEvent, Hash>().unwrap().unwrap();
			for event_record in &event_records {
				println!("decoded: {:?} {:?}", event_record.phase, event_record.event);
				match &event_record.event {
					RuntimeEvent::System(system_event) => {
						println!("********** system event: {:?}", system_event);
						let mut lock = api_clone.lock().unwrap();
						println!("Thread lock");
						lock.update_runtime().unwrap();
						println!("********** runtime update finished");
						match &system_event {
							frame_system::Event::ExtrinsicSuccess { dispatch_info } => {
								println!("DispatchInfo: {:?}", dispatch_info);
							},
							_ => {
								debug!("ignoring unsupported system event");
							},
						}
					},
					_ => debug!("ignoring unsupported module event: {:?}", event_record.event),
				}
			}
		}
	});
	receiver
}

#[tokio::main]
async fn main() {
	env_logger::init();

	// Initialize the api.
	let client = JsonrpseeClient::with_default_url().unwrap();
	let api = Arc::new(Mutex::new(
		Api::<(), _, PlainTipExtrinsicParams<Runtime>, Runtime>::new(client).unwrap(),
	));

	let receiver = auto_runtime_upgrade(Arc::clone(&api));

	let mut subscription = {
		println!("Subscribe to events");
		let lock = api.lock().unwrap();
		lock.subscribe_events().unwrap()
	};

	// Wait for event callbacks from the node, which are received via subscription.
	for _ in 0..5 {
		let event_records = subscription.next_event::<RuntimeEvent, Hash>().unwrap().unwrap();
		for event_record in &event_records {
			println!("decoded: {:?} {:?}", event_record.phase, event_record.event);
			match &event_record.event {
				RuntimeEvent::Balances(balances_event) => {
					println!(">>>>>>>>>> balances event: {:?}", balances_event);
					match &balances_event {
						pallet_balances::Event::Transfer { from, to, amount } => {
							println!("Transactor: {:?}", from);
							println!("Destination: {:?}", to);
							println!("Value: {:?}", amount);
						},
						_ => {
							debug!("ignoring unsupported balances event");
						},
					}
				},
				RuntimeEvent::System(system_event) => {
					println!(">>>>>>>>>> system event: {:?}", system_event);
					match &system_event {
						frame_system::Event::ExtrinsicSuccess { dispatch_info } => {
							println!("DispatchInfo: {:?}", dispatch_info);
						},
						_ => {
							debug!("ignoring unsupported system event");
						},
					}
				},
				_ => debug!("ignoring unsupported module event: {:?}", event_record.event),
			}
		}
	}

	// After we finished whatever we wanted, unusubscribe from the subscription,
	// to ensure, that the node does not keep sending us events.
	//subscription.unsubscribe().unwrap();
	receiver.join().unwrap();
}
