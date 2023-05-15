
use super::*;


use crate as pallet_lottery;

use sp_runtime::{
  testing::Header,
  traits::{BlakeTwo256, IdentityLookup},
  Perbill
};
use frame_support::{
  parameter_types,
  traits::{
    ConstU32,
    ConstU64,
    OnFinalize,
    OnInitialize, 
  }
};

use frame_system::EnsureRoot;
use frame_support_test::TestRandomness;

use sp_core::H256;

type UncheckedExtrinsic = frame_system::mocking::MockUncheckedExtrinsic<Test>;
type Block = frame_system::mocking::MockBlock<Test>;

frame_support::construct_runtime!(
  pub enum Test where
    Block = Block,
    NodeBlock = Block,
    UncheckedExtrinsic = UncheckedExtrinsic,
  {
    System: frame_system::{Pallet, Call, Config, Storage, Event<T>},
		Balances: pallet_balances::{Pallet, Call, Storage, Config<T>, Event<T>},
		Lottery: pallet_lottery::{Pallet, Call, Storage, Event<T>},
  }
);

parameter_types! {
	pub const AvailableBlockRatio: Perbill = Perbill::one();
}

impl frame_system::Config for Test {
	type BaseCallFilter = frame_support::traits::Everything;
	type BlockWeights = ();
	type BlockLength = ();
	type DbWeight = ();
	type RuntimeOrigin = RuntimeOrigin;
	type Index = u64;
	type RuntimeCall = RuntimeCall;
	type BlockNumber = u64;
	type Hash = H256;
	type Hashing = BlakeTwo256;
	type AccountId = u64;
	type Lookup = IdentityLookup<Self::AccountId>;
	type Header = Header;
	type RuntimeEvent = RuntimeEvent;
	type BlockHashCount = ConstU64<250>;
	type Version = ();
	type PalletInfo = PalletInfo;
	type AccountData = pallet_balances::AccountData<u64>;
	type OnNewAccount = ();
	type OnKilledAccount = ();
	type SystemWeightInfo = ();
	type SS58Prefix = ();
	type OnSetCode = ();
	type MaxConsumers = ConstU32<16>;
}

impl pallet_balances::Config for Test {
	type MaxLocks = ();
	type MaxReserves = ();
	type ReserveIdentifier = [u8; 8];
	type Balance = u64;
	type RuntimeEvent = RuntimeEvent;
	type DustRemoval = ();
	type ExistentialDeposit = ConstU64<1>;
	type AccountStore = System;
	type WeightInfo = ();
}

parameter_types! {
	pub const LotteryPalletId: PalletId = PalletId(*b"eno/loto");
}

impl pallet_lottery::Config for Test {
  type Currency =  Balances;
  type Randomness = TestRandomness<Self>;
  type RuntimeEvent = RuntimeEvent;
  type PalletId = LotteryPalletId;
  type ManagerOrigin = EnsureRoot<u64>;
  type MaxCalls = ConstU32<2>;
  type RuntimeCall = RuntimeCall;
  type ValidateCall = Lottery;
  type MaxGenerateRandom = ConstU32<10>;
}

pub type SystemCall = frame_system::Call<Test>;
pub type BalancesCall = pallet_balances::Call<Test>;

pub fn new_test_ext() -> sp_io::TestExternalities {
  let mut t = frame_system::GenesisConfig::default().build_storage::<Test>().unwrap();

  pallet_balances::GenesisConfig::<Test> {
    balances: vec![(1, 100), (2, 100), (3, 100), (4, 100), (5, 100)],
  }
  .assimilate_storage(&mut t)
  .unwrap();
  t.into()
}

pub fn run_to_block(n: u64) {
  while System::block_number() < n {
    if System::block_number() > 1 {
      Lottery::on_finalize(System::block_number());
      System::on_finalize(System::block_number());
    }

    System::set_block_number(System::block_number()  + 1);
    System::on_initialize(System::block_number());
    Lottery::on_initialize(System::block_number());
  }  
}


