#![cfg_attr(not(feature = "std"), no_std)]

use codec::{Decode, Encode};
use core::{convert::TryInto, fmt};
use sp_std::{prelude::*, collections::vec_deque::VecDeque, str};
use log;


use sp_runtime::{
	traits::{Zero, AtLeast32BitUnsigned, AccountIdConversion, BlockNumberProvider},
	offchain as rt_offchain,
	transaction_validity::{
		InvalidTransaction, TransactionSource, TransactionValidity, ValidTransaction,
	},
	RuntimeDebug,
};

use serde::{Deserialize, Deserializer};

pub use pallet::*;
// pub use sp_core::hashing::blake2_128;


#[frame_support::pallet]
pub mod pallet {
	use super::*;
	use frame_support::{
		pallet_prelude::*,
		dispatch::DispatchResult,
	};
	use frame_system::{
		pallet_prelude::*,
		offchain::{
			AppCrypto, CreateSignedTransaction, SendSignedTransaction, SendUnsignedTransaction,
			SignedPayload, Signer, SigningTypes, SubmitTransaction,
		},
	};
	// use sp_io::hashing::blake2_128;
	use sp_core::{crypto::KeyTypeId};
	use sp_arithmetic::per_things::Permill;

	pub const KEY_TYPE: KeyTypeId = KeyTypeId(*b"demo");
	const NUM_VEC_LEN: usize = 10;
	const UNSIGNED_TXS_PRIORITY: u64 = 100;

	const HTTP_COINCAP_URL: &str = "https://api.coincap.io/v2/assets/polkadot";
	const HTTP_HEADER_USER_AGENT: &str = "ocw";
	const FETCH_TIMEOUT_PERIOD: u64 = 3000;


	pub mod crypto {
		use crate::KEY_TYPE;
		use sp_core::sr25519::Signature as Sr25519Signature;
		use sp_runtime::app_crypto::{app_crypto, sr25519};
		use sp_runtime::{traits::Verify, MultiSignature, MultiSigner};

		app_crypto!(sr25519, KEY_TYPE);

		pub struct TestAuthId;
		// implemented for ocw-runtime
		impl frame_system::offchain::AppCrypto<MultiSigner, MultiSignature> for TestAuthId {
			type RuntimeAppPublic = Public;
			type GenericSignature = sp_core::sr25519::Signature;
			type GenericPublic = sp_core::sr25519::Public;
		}

		impl frame_system::offchain::AppCrypto<<Sr25519Signature as Verify>::Signer, Sr25519Signature>
		for TestAuthId
		{
			type RuntimeAppPublic = Public;
			type GenericSignature = sp_core::sr25519::Signature;
			type GenericPublic = sp_core::sr25519::Public;
		}
	}

	#[derive(Encode, Decode, Clone, PartialEq, Eq, RuntimeDebug)]
	pub struct Payload<Public> {
		number: u64,
		public: Public,
	}

	impl<T: SigningTypes> SignedPayload<T> for Payload<T::Public> {
		fn public(&self) -> T::Public {
			self.public.clone()
		}
	}


	#[derive(Debug, Deserialize, Encode, Decode, Default)]
	struct IndexingData(Vec<u8>, u64);

	#[derive(Deserialize, Encode, Decode, Default)]
	#[serde(rename_all = "camelCase")]
	struct CoinCapData {
		#[serde(deserialize_with = "de_string_to_bytes")]
		price_usd: Vec<u8>
	}

	#[derive(Deserialize, Encode, Decode, Default)]
	struct PriceInfo {
		data: CoinCapData,
		timestamp: u64
	}

	pub fn de_string_to_bytes<'de, D>(de: D) -> Result<Vec<u8>, D::Error>
	where
	D: Deserializer<'de>,
	{
		let s: &str = Deserialize::deserialize(de)?;
		Ok(s.as_bytes().to_vec())
	}

	impl fmt::Debug for PriceInfo {
		// `fmt` converts the vector of bytes inside the struct back to string for
		//   more friendly display.
		fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
			write!(
				f,
				"{{ price: {} }}",
				str::from_utf8(&self.data.price_usd).map_err(|_| fmt::Error)?,
			)
		}
	}

	#[pallet::config]
	pub trait Config: frame_system::Config + CreateSignedTransaction<Call<Self>> {
		type Event: From<Event<Self>> + IsType<<Self as frame_system::Config>::Event>;
		type Call: From<Call<Self>>;
		type AuthorityId: AppCrypto<Self::Public, Self::Signature>;
	}

	#[pallet::pallet]
	#[pallet::generate_store(pub(super) trait Store)]
	pub struct Pallet<T>(_);

	#[pallet::storage]
	#[pallet::getter(fn prices)]
	pub type Prices<T> = StorageValue<_, VecDeque<(u64, Permill)>, ValueQuery>;

	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config> {
		NewNumber(Option<T::AccountId>, u64),
		UpdatePrice(Option<T::AccountId>, u64, Permill)
	}

	// Errors inform users that something went wrong.
	#[pallet::error]
	pub enum Error<T> {
		// Error returned when not sure which ocw function to executed
		UnknownOffchainMux,

		// Error returned when making signed transactions in off-chain worker
		NoLocalAcctForSigning,
		OffchainSignedTxError,

		// Error returned when making unsigned transactions in off-chain worker
		OffchainUnsignedTxError,

		// Error returned when making unsigned transactions with signed payloads in off-chain worker
		OffchainUnsignedTxSignedPayloadError,

		// Error returned when fetching github info
		HttpFetchingError,

		// Json decode error
		JsonDecodeError,
	}

	#[pallet::hooks]
	impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {
		/// Offchain Worker entry point.
		///
		/// By implementing `fn offchain_worker` you declare a new offchain worker.
		/// This function will be called when the node is fully synced and a new best block is
		/// succesfuly imported.
		/// Note that it's not guaranteed for offchain workers to run on EVERY block, there might
		/// be cases where some blocks are skipped, or for some the worker runs twice (re-orgs),
		/// so the code should be able to handle that.
		/// You can use `Local Storage` API to coordinate runs of the worker.
		fn offchain_worker(block_number: T::BlockNumber) {
			log::info!("Hello World from offchain workers!");

			// Here we are showcasing various techniques used when running off-chain workers (ocw)
			// 1. Sending signed transaction from ocw
			// 2. Sending unsigned transaction from ocw
			// 3. Sending unsigned transactions with signed payloads from ocw
			// 4. Fetching JSON via http requests in ocw
			const TX_TYPES: u32 = 5;
			let modu = block_number.try_into().map_or(TX_TYPES, |bn: usize| (bn as u32) % TX_TYPES);
			let result = match modu {
				// 0 => Self::offchain_signed_tx(block_number),
				// 1 => Self::offchain_unsigned_tx(block_number),
				// 2 => Self::offchain_unsigned_tx_signed_payload(block_number),
				// 3 => Self::fetch_github_info(),
				4 => Self::fetch_price_info(),
				_ => Err(Error::<T>::UnknownOffchainMux),
			};

			if let Err(e) = result {
				log::error!("offchain_worker error: {:?}", e);
			}
		}
	}

	#[pallet::validate_unsigned]
	impl<T: Config> ValidateUnsigned for Pallet<T> {
		type Call = Call<T>;

		/// Validate unsigned call to this module.
		///
		/// By default unsigned transactions are disallowed, but implementing the validator
		/// here we make sure that some particular calls (the ones produced by offchain worker)
		/// are being whitelisted and marked as valid.
		fn validate_unsigned(_source: TransactionSource, call: &Self::Call)
		-> TransactionValidity
		{
			let valid_tx = |provide| ValidTransaction::with_tag_prefix("ocw-demo")
			.priority(UNSIGNED_TXS_PRIORITY)
			.and_provides([&provide])
			.longevity(3)
			.propagate(true)
			.build();

			log::info!("validate_unsigned: {:?}", call);

			match call {
				Call::submit_price_unsigned(_a, _b) => valid_tx(b"submit_price_unsigned".to_vec()),
				_ => InvalidTransaction::Call.into(),
			}
		}
	}

	#[pallet::call]
	impl<T: Config> Pallet<T> {
		#[pallet::weight(10000)]
		pub fn submit_price_unsigned(origin: OriginFor<T>, a: u64, b: Permill) -> DispatchResult {

			log::info!("start submit_price_unsigned:");
			let _ = ensure_none(origin)?;
			log::info!("submit_price_unsigned: {:0?}.{:1?}", a, b);
			Self::append_or_replace_price(a, b);
			Self::deposit_event(Event::UpdatePrice(None, a, b));
			Ok(())
		}
	}

	impl<T: Config> Pallet<T> {

		fn append_or_replace_price(a: u64, b: Permill) {
			Prices::<T>::mutate(|prices| {
				if prices.len() == NUM_VEC_LEN {
					let pop_price = prices.pop_front();
					match pop_price {
						Some(i) => log::info!("pop_price: {:0?}.{:1?}", i.0, i.1),
						None => {}
					}
				}
				prices.push_back((a, b));
				log::info!("Prices vector: {:?}", prices);
			});
		}

		fn fetch_price_info() -> Result<(), Error<T>> {
			let (a, b) = Self::fetch_dot_parse().unwrap();
			log::info!("u64: {:0?}, Permill: {:1?}", a, b);
			let call = Call::submit_price_unsigned(a, b);

			SubmitTransaction::<T, Call<T>>::submit_unsigned_transaction(call.into())
				.map_err(|_| {
					log::error!("Failed in offchain_unsigned_tx");
					<Error<T>>::OffchainUnsignedTxError
				})
		}

		fn fetch_dot_parse() -> Result<(u64, Permill), Error<T>> {
			let resp_bytes = Self::fetch_from_remote(HTTP_COINCAP_URL).map_err(|e| {
				log::error!("fetch_from_remote error: {:?}", e);
				<Error<T>>::HttpFetchingError
			})?;
			let resp_str = str::from_utf8(&resp_bytes).map_err(|_| <Error<T>>::HttpFetchingError)?;

			let price_info: PriceInfo = serde_json::from_str(&resp_str).map_err(|_| <Error<T>>::JsonDecodeError)?;

			let price_str = str::from_utf8(&price_info.data.price_usd).unwrap();
			let price_vec = price_str.split(".").collect::<Vec<&str>>();
			log::info!("{:?}", price_vec);

			let price_u64 = price_vec[0].parse::<u64>().unwrap();

			let price_a = price_vec[1].as_bytes().to_vec()[0..6].to_vec();
			let price_b = str::from_utf8(&price_a).unwrap();
			let price_decimal = price_b.parse::<u32>().unwrap();

			Ok((price_u64, Permill::from_parts(price_decimal)))
		}

		fn fetch_from_remote(url: &str) -> Result<Vec<u8>, Error<T>> {
			log::info!("sending request to: {}", url);

			// Initiate an external HTTP GET request. This is using high-level wrappers from `sp_runtime`.
			let request = rt_offchain::http::Request::get(url);

			// Keeping the offchain worker execution time reasonable, so limiting the call to be within 3s.
			let timeout = sp_io::offchain::timestamp()
			.add(rt_offchain::Duration::from_millis(FETCH_TIMEOUT_PERIOD));

			// For github API request, we also need to specify `user-agent` in http request header.
			//   See: https://developer.github.com/v3/#user-agent-required
			let pending = request.add_header("User-Agent", HTTP_HEADER_USER_AGENT)
				.deadline(timeout) // Setting the timeout time
				.send() // Sending the request out by the host
				.map_err(|_| <Error<T>>::HttpFetchingError)?;

			// By default, the http request is async from the runtime perspective. So we are asking the
			//   runtime to wait here.
			// The returning value here is a `Result` of `Result`, so we are unwrapping it twice by two `?`
			//   ref: https://substrate.dev/rustdocs/v2.0.0/sp_runtime/offchain/http/struct.PendingRequest.html#method.try_wait
			let response = pending
			.try_wait(timeout)
			.map_err(|_| <Error<T>>::HttpFetchingError)?
			.map_err(|_| <Error<T>>::HttpFetchingError)?;

			if response.code != 200 {
				log::error!("Unexpected http request status code: {}", response.code);
				return Err(<Error<T>>::HttpFetchingError);
			}

			// Next we fully read the response body and collect it to a vector of bytes.
			Ok(response.body().collect::<Vec<u8>>())
		}
	}

	impl<T: Config> BlockNumberProvider for Pallet<T> {
		type BlockNumber = T::BlockNumber;

		fn current_block_number() -> Self::BlockNumber {
			<frame_system::Pallet<T>>::block_number()
		}
	}
}
