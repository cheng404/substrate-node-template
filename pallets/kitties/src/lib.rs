#![cfg_attr(not(feature = "std"), no_std)]

use codec::{Decode, Encode};
use sp_std::{prelude::*, convert::TryInto};

use frame_support::{
	PalletId,
	traits::{
		Currency, ReservableCurrency, ExistenceRequirement::KeepAlive,
		Randomness,
	},
};
use sp_runtime::{
	traits::{Zero, AtLeast32BitUnsigned, AccountIdConversion}
};
pub use pallet::*;
// pub use sp_core::hashing::blake2_128;

#[cfg(test)]
mod mock;

#[cfg(test)]
mod tests;

type BalanceOf<T> = <<T as Config>::Currency as Currency<<T as frame_system::Config>::AccountId>>::Balance;

#[frame_support::pallet]
pub mod pallet {
	use super::*;
	use frame_support::{
		pallet_prelude::*,
		dispatch::DispatchResult,
	};
	use frame_system::pallet_prelude::*;
	use sp_io::hashing::blake2_128;
	// use sp_core::hashing::blake2_128;
	#[derive(Encode, Decode)]
	pub struct Kitty(pub [u8; 16]);

	// type KittyIndex = u32;
	// type KittyPrice = u32;

	#[pallet::config]
	pub trait Config: frame_system::Config {
		#[pallet::constant]
		type PalletId: Get<PalletId>;
		type Event: From<Event<Self>> + IsType<<Self as frame_system::Config>::Event>;
		type Randomness: Randomness<Self::Hash, Self::BlockNumber>;
		type Currency: ReservableCurrency<Self::AccountId>;
		type KittyIndex: Member + Parameter + AtLeast32BitUnsigned + Default + Copy + MaxEncodedLen;
	}

	#[pallet::pallet]
	#[pallet::generate_store(pub(super) trait Store)]
	pub struct Pallet<T>(_);

	#[pallet::storage]
	#[pallet::getter(fn kitties_count)]
	pub type KittiesCount<T: Config> = StorageValue<_, u64, ValueQuery>;

	// 创建kitty默认费用
	pub struct KittyCreateFeeDefault;
	impl Get<u64> for KittyCreateFeeDefault {
		fn get() -> u64 {
			5
		}
	}

	/// 创建kitty需要花费的金额
	#[pallet::storage]
	#[pallet::getter(fn kitty_create_fee)]
	pub type KittyCreateFee<T: Config> = StorageValue<_, u64, ValueQuery, KittyCreateFeeDefault>;

	#[pallet::storage]
	#[pallet::getter(fn kitties)]
	pub type Kitties<T: Config> = StorageMap<_, Blake2_128Concat, T::KittyIndex, Option<Kitty>, ValueQuery>;

	#[pallet::storage]
	#[pallet::getter(fn owner)]
	pub type Owner<T: Config> = StorageMap<_, Blake2_128Concat, T::KittyIndex, Option<T::AccountId>, ValueQuery>;

	#[pallet::storage]
	#[pallet::getter(fn price_of)]
	pub type PriceOf<T: Config> = StorageMap<_, Blake2_128Concat, T::KittyIndex, Option<BalanceOf<T>>, ValueQuery>;

	#[pallet::event]
	#[pallet::metadata(T::AccountId = "AccountId", BalanceOf<T> = "Balance")]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config> {
		KittyCreate(T::AccountId, T::KittyIndex),
		KittyTransfer(T::AccountId, T::AccountId, T::KittyIndex),
		KittyBuy(T::AccountId, T::KittyIndex, BalanceOf<T>),
		KittyForSale(T::AccountId, T::KittyIndex, BalanceOf<T>),
	}

	#[pallet::error]
	pub enum Error<T> {
		KittiesCountOverflow,
		KittyNotExist,
		NotKittyOwner,
		SameParentIndex,
		InvalidKittyIndex,
		// InvalidKittyPrice,
		KittyNotForSale,
		PayFeeError,
	}

	#[pallet::call]
	impl<T: Config> Pallet<T> {
		#[pallet::weight(0)]
		pub fn create(origin: OriginFor<T>) -> DispatchResult {
			let who = ensure_signed(origin)?;

			let kitty_id = Self::new_kitty_id()?;
			// let _s: u64 = TryInto::<u64>::try_into(kitty_id).ok().unwrap();
			let dna = Self::random_value(&who);
			// let fee = <u64 as TryInto<BalanceOf<T>>>::try_into(KittyCreateFee::<T>::get()).ok().unwrap();
			let fee = KittyCreateFee::<T>::get().try_into().ok().unwrap();
			let transfer_res = T::Currency::transfer(&who, &Self::account_id(), fee, KeepAlive);
			ensure!(transfer_res.is_ok(), Error::<T>::PayFeeError);

			debug_assert!(transfer_res.is_ok());
			Kitties::<T>::insert(kitty_id, Some(Kitty(dna)));
			Owner::<T>::insert(kitty_id, Some(who.clone()));
			KittiesCount::<T>::mutate(|t| {*t = *t + 1});

			Self::deposit_event(Event::<T>::KittyCreate(who.clone(), kitty_id));

			Ok(())
		}

		#[pallet::weight(0)]
		pub fn transfer(origin: OriginFor<T>, new_owner: T::AccountId, kitty_id: T::KittyIndex) -> DispatchResult {
			let who = ensure_signed(origin)?;

			let _owner = Owner::<T>::get(kitty_id);
			ensure!(_owner.is_some(), Error::<T>::KittyNotExist);
			ensure!(Some(who.clone()) == _owner, Error::<T>::NotKittyOwner);

			Owner::<T>::insert(kitty_id, Some(new_owner.clone()));

			Self::deposit_event(Event::<T>::KittyTransfer(who, new_owner, kitty_id));
			Ok(())
		}

		#[pallet::weight(0)]
		pub fn breed(origin: OriginFor<T>, kitty_id_1: T::KittyIndex, kitty_id_2: T::KittyIndex) -> DispatchResult {
			let who = ensure_signed(origin)?;
			ensure!(kitty_id_1 != kitty_id_2, Error::<T>::SameParentIndex);
			ensure!(
				Some(who.clone()) == Owner::<T>::get(kitty_id_1),
				Error::<T>::NotKittyOwner
			);
			ensure!(
				Some(who.clone()) == Owner::<T>::get(kitty_id_2),
				Error::<T>::NotKittyOwner
			);

			let kitty1 = Self::kitties(kitty_id_1).ok_or(Error::<T>::InvalidKittyIndex)?;
			let kitty2 = Self::kitties(kitty_id_2).ok_or(Error::<T>::InvalidKittyIndex)?;

			let kitty_id = Self::new_kitty_id()?;

			let dna_1 = kitty1.0;
			let dna_2 = kitty2.0;
			let selector = Self::random_value(&who);
			let mut new_dna = [0u8; 16];
			for i in 0..dna_1.len() {
				new_dna[i] = (selector[i] & dna_1[i]) | (!selector[i] & dna_2[i]);
			}

			Kitties::<T>::insert(kitty_id, Some(Kitty(new_dna)));
			Owner::<T>::insert(kitty_id, Some(who.clone()));
			KittiesCount::<T>::mutate(|t| {*t = *t + 1});

			Self::deposit_event(Event::<T>::KittyCreate(who, kitty_id));

			Ok(())
		}

		#[pallet::weight(0)]
		pub fn buy_kitty(origin: OriginFor<T>, kitty_id: T::KittyIndex) -> DispatchResult {
			let who = ensure_signed(origin)?;
			let price = PriceOf::<T>::get(kitty_id).ok_or(Error::<T>::KittyNotForSale)?;
			ensure!(
				!price.is_zero(),
				Error::<T>::KittyNotForSale
			);

			let old_owner = Owner::<T>::get(kitty_id).ok_or(Error::<T>::NotKittyOwner)?;

			let transfer_res = T::Currency::transfer(&who, &old_owner, price, KeepAlive);
			ensure!(transfer_res.is_ok(), Error::<T>::PayFeeError);

			Owner::<T>::insert(kitty_id, Some(who.clone()));
			// 将kitty标记为不出售
			PriceOf::<T>::remove(kitty_id);

			Self::deposit_event(Event::<T>::KittyTransfer(old_owner, who.clone(), kitty_id));
			Self::deposit_event(Event::<T>::KittyBuy(who, kitty_id, price));
			Ok(())
		}

		/// 设置kitty是否出售，price为0时不出售
		#[pallet::weight(0)]
		pub fn for_sale(origin: OriginFor<T>, kitty_id: T::KittyIndex, price: BalanceOf<T>) -> DispatchResult {
			let who = ensure_signed(origin)?;
			ensure!(
				Some(who.clone()) == Owner::<T>::get(kitty_id),
				Error::<T>::NotKittyOwner
			);
			// ensure!(!price.is_zero(), Error::<T>::InvalidKittyPrice);
			if price.is_zero() {
				PriceOf::<T>::remove(kitty_id);
			}
			else{
				PriceOf::<T>::insert(kitty_id, Some(price))
			}

			Self::deposit_event(Event::<T>::KittyForSale(who, kitty_id, price));

			Ok(())
		}
	}

	impl<T: Config> Pallet<T> {
		pub fn account_id() -> T::AccountId {
			T::PalletId::get().into_account()
		}

		pub fn new_kitty_id() -> Result<T::KittyIndex, Error<T>> {
			let kitty_count = Self::kitties_count();
			let res = kitty_count.checked_add(1).ok_or(Error::<T>::KittiesCountOverflow)?;
			Ok(res.try_into().ok().unwrap())
		}

		pub fn random_value(sender: &T::AccountId) -> [u8; 16] {
			let pay_load = (
				T::Randomness::random_seed(),
				&sender,
				<frame_system::Pallet<T>>::extrinsic_index(),
			);
			pay_load.using_encoded(blake2_128)
		}
	}
}
