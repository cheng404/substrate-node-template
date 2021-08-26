use crate::{mock::*, Error};
use frame_support::{assert_noop, assert_ok};
use super::*;

#[test]
fn create_kitty_works() {
	new_test_ext().execute_with(|| {
		let balance_before = Balances::free_balance(&1);
		let fee = KittyCreateFee::<Test>::get();
		assert_eq!(fee, 5);	// 默认5
		assert_ok!(KittiesModule::create(Origin::signed(1)));
		assert_eq!(Balances::free_balance(&1), balance_before - fee);

		assert_eq!(Owner::<Test>::get(1), Some(1));
		assert_eq!(KittiesCount::<Test>::get(), 1);
	});
}

#[test]
fn create_kitty_fails_when_exceeds_balance() {
	new_test_ext().execute_with(|| {
		Balances::set_balance(Origin::root(), 1, 0, 0).ok();
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
		assert_eq!(Owner::<Test>::get(_count_before + 1), Some(who))
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
