use crate::{mock::*, Error};
use frame_support::{assert_noop, assert_ok};
use super::*;

#[test]
fn create_kitty_works() {
	new_test_ext().execute_with(|| {
		let who = 1;
		let balance_before = Balances::free_balance(&who);
		let fee = KittyCreateFee::<Test>::get();
		assert_eq!(fee, 5);	// 默认5
		assert_ok!(KittiesModule::create(Origin::signed(who)));

		assert!(Kitties::<Test>::get(1).is_some());
		assert_eq!(Owner::<Test>::get(1), Some(who));
		assert_eq!(KittiesCount::<Test>::get(), 1);
		assert_eq!(Balances::free_balance(&1), balance_before - fee);
	});
}

#[test]
fn create_kitty_fails_when_exceeds_balance() {
	new_test_ext().execute_with(|| {
		assert_ok!(Balances::set_balance(Origin::root(), 1, 0, 0));
		assert_noop!(
			KittiesModule::create(Origin::signed(1)),
			Error::<Test>::PayFeeError
		);
	});
}

#[test]
fn validate_kitty_count() {
	new_test_ext().execute_with(|| {
		assert_eq!(KittiesCount::<Test>::get(), 0);
		for n in 1..=10 {
			assert_ok!(KittiesModule::create(Origin::signed(1)));
			assert_eq!(KittiesCount::<Test>::get(), n);
		}
	});
}

#[test]
fn create_kitty_fails_when_kitty_count_overflow() {
	new_test_ext().execute_with(|| {
		KittiesCount::<Test>::put(u64::MAX);
		assert_noop!(
			KittiesModule::create(Origin::signed(1)),
			Error::<Test>::KittiesCountOverflow
		);
	});
}

#[test]
fn transfer_kitty_works() {
	new_test_ext().execute_with(|| {
		let (sender, receiver) = (1, 2);
		assert_ok!(KittiesModule::create(Origin::signed(sender)));
		assert_ok!(KittiesModule::transfer(Origin::signed(sender), receiver, 1));
		assert_eq!(Owner::<Test>::get(1), Some(receiver));

		let (sender, receiver) = (2, 3);
		assert_ok!(KittiesModule::create(Origin::signed(sender)));
		assert_ok!(KittiesModule::transfer(Origin::signed(sender), receiver, 2));
		assert_eq!(Owner::<Test>::get(2), Some(receiver));
	});
}

#[test]
fn transfer_kitty_fails_when_not_exists() {
	new_test_ext().execute_with(|| {
		let (sender, receiver) = (1, 2);
		assert_noop!(
			KittiesModule::transfer(Origin::signed(sender), receiver, 1),
			Error::<Test>::KittyNotExist
		);
	});
}

#[test]
fn transfer_kitty_when_not_owner() {
	new_test_ext().execute_with(|| {
		let (sender, receiver) = (1, 2);
		assert_ok!(KittiesModule::create(Origin::signed(sender)));
		assert_noop!(
			KittiesModule::transfer(Origin::signed(receiver), 3, 1),
			Error::<Test>::NotKittyOwner
		);
	});
}

#[test]
fn breed_kitty_works() {
	new_test_ext().execute_with(|| {
		let who = 1;
		assert_ok!(KittiesModule::create(Origin::signed(who)));
		assert_ok!(KittiesModule::create(Origin::signed(who)));

		let _count_before = KittiesCount::<Test>::get();
		assert_ok!(KittiesModule::breed(Origin::signed(who), 1, 2));

		let _count_after = _count_before + 1;
		assert!(Kitties::<Test>::get(_count_after).is_some());
		assert_eq!(Owner::<Test>::get(_count_after), Some(who));
		assert_eq!(KittiesCount::<Test>::get(), _count_after);
	});
}

#[test]
fn breed_kitty_fails_when_not_owner() {
	new_test_ext().execute_with(|| {
		let (owner, other) = (1, 2);
		assert_ok!(KittiesModule::create(Origin::signed(owner)));
		assert_ok!(KittiesModule::create(Origin::signed(owner)));
		assert_ok!(KittiesModule::create(Origin::signed(other)));

		let (index_1, index_2, index_not_owner, index_not_exists) = (1, 2, 3, 4);
		assert_eq!(Owner::<Test>::get(index_1), Some(owner));
		assert_eq!(Owner::<Test>::get(index_2), Some(owner));
		assert_eq!(Owner::<Test>::get(index_not_owner), Some(other));

		assert_noop!(
			KittiesModule::breed(Origin::signed(owner), index_1, index_not_owner),
			Error::<Test>::NotKittyOwner
		);
		assert_noop!(
			KittiesModule::breed(Origin::signed(owner), index_2, index_not_exists),
			Error::<Test>::NotKittyOwner
		);
	});
}

#[test]
fn breed_kitty_fails_when_same_parent() {
	new_test_ext().execute_with(|| {
		let owner = 1;
		assert_ok!(KittiesModule::create(Origin::signed(owner)));

		assert_noop!(
			KittiesModule::breed(Origin::signed(owner), 1, 1),
			Error::<Test>::SameParentIndex
		);
	})
}

#[test]
fn breed_kitty_fails_when_kitty_count_overflow() {
	new_test_ext().execute_with(|| {
		let owner = 1;
		KittiesCount::<Test>::put(u64::MAX - 2);
		assert_ok!(KittiesModule::create(Origin::signed(owner)));
		assert_ok!(KittiesModule::create(Origin::signed(owner)));

		assert_noop!(
			KittiesModule::breed(Origin::signed(owner), u64::MAX - 1, u64::MAX),
			Error::<Test>::KittiesCountOverflow
		);
	});
}

#[test]
fn for_sale_works() {
	new_test_ext().execute_with(|| {
		let who = 1;
		assert_ok!(KittiesModule::create(Origin::signed(who)));
		let price = 10;
		assert_ok!(KittiesModule::for_sale(Origin::signed(who), 1, price));
		assert_eq!(PriceOf::<Test>::get(1), Some(10));

		assert_ok!(KittiesModule::for_sale(Origin::signed(who), 1, 0));
		assert_eq!(PriceOf::<Test>::get(1), None);
	});
}

#[test]
fn for_sale_fails_when_not_owner_or_not_exists() {
	new_test_ext().execute_with(|| {
		let (owner, other) = (1, 2);
		assert_ok!(KittiesModule::create(Origin::signed(owner)));
		let price = 10;

		assert_noop!(
			KittiesModule::for_sale(Origin::signed(other), 1, price),
			Error::<Test>::NotKittyOwner
		);

		assert_noop!(
			KittiesModule::for_sale(Origin::signed(other), 2, price),
			Error::<Test>::NotKittyOwner
		);
	});
}

#[test]
fn buy_kitty_works() {
	new_test_ext().execute_with(|| {
		let (buyer, seller) = (1, 2);
		assert_ok!(KittiesModule::create(Origin::signed(seller)));
		let price = 10;
		assert_ok!(KittiesModule::for_sale(Origin::signed(seller), 1, price));
		let seller_balance_before = Balances::free_balance(&seller);

		assert_ok!(KittiesModule::buy_kitty(Origin::signed(buyer), 1));
		assert_eq!(Owner::<Test>::get(1), Some(buyer));
		assert_eq!(Balances::free_balance(&seller), price + seller_balance_before);
		assert!(PriceOf::<Test>::get(1).is_none());
	});
}

#[test]
fn buy_kitty_fails_when_not_for_sale_or_not_exists() {
	new_test_ext().execute_with(|| {
		let (buyer, seller) = (1, 2);
		assert_ok!(KittiesModule::create(Origin::signed(seller)));

		assert_noop!(
			KittiesModule::buy_kitty(Origin::signed(buyer), 1),
			Error::<Test>::KittyNotForSale
		);
		assert_noop!(
			KittiesModule::buy_kitty(Origin::signed(buyer), 2),
			Error::<Test>::KittyNotForSale
		);
	})
}

#[test]
fn buy_kitty_fails_when_exceed_balance(){
	new_test_ext().execute_with(|| {
		let (buyer, seller) = (1, 2);
		assert_ok!(KittiesModule::create(Origin::signed(seller)));
		let price = 10;
		assert_ok!(KittiesModule::for_sale(Origin::signed(seller), 1, price));
		assert_ok!(Balances::set_balance(Origin::root(), buyer, price - 1, 0));

		assert_noop!(
			KittiesModule::buy_kitty(Origin::signed(buyer), 1),
			Error::<Test>::PayFeeError
		);
	});
}
