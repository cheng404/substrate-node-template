use crate::{mock::*, Error};
use frame_support::{assert_noop, assert_ok};
use super::*;

#[test]
fn create_kitty_works() {
	new_test_ext().execute_with(|| {
		let balance_befor = Balances::free_balance(&1);
		let fee = KittyReveseFee::<Test>::get();
		assert_eq!(fee, 5);	// 默认5
		assert_eq!(balance_befor, 1000);
		assert_ok!(KittiesModule::create(Origin::signed(1)));
		assert_eq!(Balances::free_balance(&1), balance_befor - fee);
	});
}

#[test]
fn create_kitty_fails_when_exceeds_balance() {
	new_test_ext().execute_with(|| {
		Balances::set_balance(Origin::root(), 1, 0, 0).ok();
		assert_noop!(
			KittiesModule::create(Origin::signed(1)),
			Error::<Test>::TranferError
		);
	});
}
