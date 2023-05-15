#![cfg_attr(not(feature = "std"), no_std)]

pub use pallet::*;

#[cfg(test)]
mod mock;
#[cfg(test)]
mod tests;

mod benchmarking;

pub mod weights;
pub use weights::WeightInfo;

use codec::{ Decode, Encode };
use log::info;

use frame_support::{
	dispatch::{Dispatchable, DispatchResult, GetDispatchInfo},
	pallet_prelude::{ValueQuery, MaxEncodedLen},
	traits::{ReservableCurrency, Randomness, Currency, ExistenceRequirement::KeepAlive},
	PalletId,
	storage::bounded_vec::BoundedVec,
	inherent::Vec
};
use frame_system::{pallet_prelude::{OriginFor}, ensure_signed};

use frame_support::sp_runtime::{
	traits::{AccountIdConversion, Saturating, Zero},
	ArithmeticError, DispatchError,
};
use scale_info::prelude::boxed::Box;

type BalanceOf<T> =
	<<T as Config>::Currency as Currency<<T as frame_system::Config>::AccountId>>::Balance;

type CallIndex = (u8, u8);


#[derive(Encode, Decode,Debug, Default, PartialEq, MaxEncodedLen, scale_info::TypeInfo)]
pub struct LotteryConfig<BlockNumber, Balance> {
	price: 	Balance,
	start: 	BlockNumber,
	length: BlockNumber,
	delay: 	BlockNumber,
	repeat: bool, 
}

#[frame_support::pallet()]	
pub mod pallet {
	use super::*;
	use frame_support::{pallet_prelude::{*, DispatchResult}, weights::Weight, Twox64Concat};
	use frame_system::{pallet_prelude::*};

	#[pallet::pallet]
	#[pallet::generate_store(pub(super) trait Store)]
	pub struct Pallet<T>(_);	

	#[pallet::config]
	pub trait Config: frame_system::Config {
		/// The Lottery's pallet id
		#[pallet::constant]
		type PalletId: Get<PalletId>;

		type Currency: ReservableCurrency<Self::AccountId>;
		type Randomness: Randomness<Self::Hash, Self::BlockNumber>;
		
		// The aggregated event type of the runtime.
		type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;

		
		type ManagerOrigin: EnsureOrigin<Self::RuntimeOrigin>;
		type RuntimeCall: Parameter + 
			Dispatchable<RuntimeOrigin = Self::RuntimeOrigin> +
			GetDispatchInfo +
			From<frame_system::Call<Self>>;

		#[pallet::constant]
		type MaxCalls: Get<u32>;

		type ValidateCall: ValidateCall<Self>;
		type MaxGenerateRandom: Get<u32>;

		type WeightInfo: WeightInfo;
	}

	pub trait ValidateCall<T: Config> {
		fn validate_call(call: &<T as Config>::RuntimeCall) -> bool;		
	}

	impl <T: Config> ValidateCall<T> for () {
		fn validate_call(_: &<T as Config>::RuntimeCall) -> bool {
			false
		}
	}
	
	impl <T: Config> ValidateCall<T> for Pallet<T>{
		fn validate_call(call: &<T as Config>::RuntimeCall) -> bool {
			let valid_calls = CallIndices::<T>::get();
			let call_index = match Self::call_to_index(call) {
				Ok(c) => c,
				Err(_) => return false, 
			};
			
			valid_calls.iter().any(|item| *item == call_index)
		}		
	}

	#[pallet::type_value]
	pub(super) fn MyDefault<T: Config>() -> u32 { 100u32 }

	#[pallet::storage]
	#[pallet::getter(fn heloo)]
	pub(super) type DummyBalance<T: Config> = 
		StorageValue<Value= u32, QueryKind = ValueQuery, OnEmpty= MyDefault<T>>;

	/// The configuration for the current lottery.
	#[pallet::storage]
	pub(crate) type Lottery<T: Config> =
		StorageValue<_, LotteryConfig<T::BlockNumber, BalanceOf<T>>>;

	/// The configuration for the current lottery.
	#[pallet::storage]
	pub(crate) type LotteryIndex<T: Config> =
		StorageValue<_, u32, ValueQuery>;

	// /// The configuration for the current lottery.
	#[pallet::storage]
	pub(crate) type Participants<T: Config> =	StorageMap<
		_,
		Twox64Concat,
		T::AccountId,
		(u32, BoundedVec<CallIndex, T::MaxCalls>),
		ValueQuery
	>;
		/// The configuration for the current lottery.
	#[pallet::storage]
	pub(crate) type Tickets<T: Config> =	StorageMap<
		_,
		Twox64Concat,
		u32,
		T::AccountId,
	>;

	#[pallet::storage]
	pub(crate) type TicketsCount<T: Config> =	StorageValue<_, u32, ValueQuery>;

	#[pallet::storage]
	pub(crate) type CallIndices<T: Config> =
		StorageValue<_, BoundedVec<CallIndex, T::MaxCalls>, ValueQuery>;

	#[pallet::hooks]
	impl <T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {
		fn on_initialize(n: T::BlockNumber) -> Weight {					
			info!(target: "on_initialize", "=========== Starting new block: ======= ");
			info!(target: "on initialize", "DummyBalance =====> {}", DummyBalance::<T>::get());

			Lottery::<T>::mutate(|mut lottery| -> Weight {
				if let Some(config) = &mut lottery {					
					info!(target: "on_initialize", "Lottery config  =====> {:?}", config);
					let payout_block = config.start.saturating_add(config.length)
																																		 .saturating_add(config.delay);
					let (lottery_account, lottery_balance) = Self::pot();																																		
					if n >= payout_block {
						let winner = Self::lucky_draw().unwrap_or(lottery_account.clone());

						let res = T::Currency::transfer(
							&lottery_account,
							&winner,
							lottery_balance,
							KeepAlive,
						);

						info!("Lottery balance {} been transferred",
						 if res.is_ok() {"has" } else { "has not" }
						);

						Self::deposit_event(Event::Winner { winner, balance: lottery_balance.clone() });


						TicketsCount::<T>::kill();

						if config.repeat {
							config.start = n;
							LotteryIndex::<T>::mutate(|index| *index = index.saturating_add(1));
							return  T::WeightInfo::on_initialize_repeat()						
						} else {
							*lottery = None;
							return  T::WeightInfo::on_initialize_end()
						}

					}																																
				}

				T::DbWeight::get().reads(1)
			})			
		}
	}

	#[pallet::error]
	pub enum Error<T> {
		NotConfigured,
		InProgress,
		AlreadyParticipating,
		EncodingFailed,
		TooManyCalls,
		AlreadyEnded,
		InvalidCall,			
	}

	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config> {
		NotConfigured,

		LotterStarted {
			lottery_id: T::AccountId,
			balance: BalanceOf<T>,
		},

		CallsUpdated,
		TicketBought {
			caller: T::AccountId,
			ticket_number: u32,
		},
		Winner {
			winner: T::AccountId,
			balance: BalanceOf<T>,
		}			
	}

	// Pallet callable functions
	#[pallet::call]
	impl<T: Config> Pallet<T> {

		#[pallet::weight(
			T::WeightInfo::buy_ticket()
			.saturating_add(call.get_dispatch_info().weight)
		)]
		pub fn buy_ticket(
			origin: OriginFor<T>,
			call: Box<<T as Config>::RuntimeCall>,
		)-> DispatchResult {

			info!(target: "buy_ticket", " ============== First ========== ");

			let caller = ensure_signed(origin.clone())?;
			info!(target: "buy_ticket", " ============== Second ========== ");
			call.clone().dispatch(origin).map_err(|e| e.error)?;

			info!(target: "buy_ticket", " ============== Third ========== ");

			info!(target: "buy_ticket", "=========== caller =======> {:?} ", caller);

			Self::do_buy_ticket(&caller, &call)
		}

		#[pallet::weight(T::WeightInfo::set_calls(calls.len() as u32))]
		pub fn set_calls(
			origin: OriginFor<T>,
			calls: Vec<<T as Config>::RuntimeCall>,
		)-> DispatchResult {			
			T::ManagerOrigin::ensure_origin(origin)?;
			
			ensure!(calls.len() <= T::MaxCalls::get() as usize, Error::<T>::TooManyCalls);

			if calls.len() < 0  {
				CallIndices::<T>::kill();
			} else {
				let listing = Self::calls_to_indices(&calls)?;
				CallIndices::<T>::put(listing);
			}

			Self::deposit_event(Event::<T>::CallsUpdated);

			Ok(())
		}

		#[pallet::weight(T::WeightInfo::start_lottery())]
		pub fn start_lottery(
			origin: OriginFor<T>,
			price: BalanceOf<T>,
			length: T::BlockNumber,
			delay: T::BlockNumber,
			repeat: bool,	
		)-> DispatchResult {
			T::ManagerOrigin::ensure_origin(origin)?;

			Lottery::<T>::try_mutate(|lottery| -> DispatchResult {
				ensure!(lottery.is_none(), Error::<T>::InProgress);

				let index = LotteryIndex::<T>::get();
				let new_index = index.checked_add(1).ok_or(ArithmeticError::Overflow)?;
				let start = frame_system::Pallet::<T>::block_number();

				*lottery = Some(LotteryConfig {
										price,
										start,
										length,
										delay,
										repeat
									});


				LotteryIndex::<T>::put(new_index);
				Ok(())
			})?;

			let lottery_account_id = Self::account_id();

			if T::Currency::total_balance(&lottery_account_id).is_zero() {
				T::Currency::deposit_creating(&lottery_account_id, T::Currency::minimum_balance());
			}

			Self::deposit_event(
				Event::LotterStarted {
					lottery_id: lottery_account_id.clone(),
					balance: T::Currency::total_balance(&lottery_account_id)
				}
			);
			
			Ok(())
		}

		#[pallet::weight(T::WeightInfo::stop_lottery())]
		pub fn stop_lottery(
			origin: OriginFor<T>,
		)-> DispatchResult {
			T::ManagerOrigin::ensure_origin(origin)?;

			Lottery::<T>::mutate(|mut lottery| {
				if let Some(config) = &mut lottery {
					config.repeat = false
				}
			});

			Ok(())
		}

		#[pallet::weight(0)]
		pub fn try_me(
			origin: OriginFor<T>,			
		 ) -> DispatchResult {

			let owner = ensure_signed(origin)?;
			// let sender = ensure_signed(origin)?;
			let account_id = Self::account_id();			

			let free_bal = T::Currency::free_balance(&account_id);
			let total = T::Currency::total_balance(&account_id);

			info!(target: "on_initialize", "=========== free_bal =======> {:?} ", free_bal);
			info!(target: "on_initialize", "=========== total =======> {:?} ", total);

			
			Ok(())
		}
	}

	impl <T: Config> Pallet<T> {
		// Get Pallet Id into AccountId
		pub fn account_id() -> T::AccountId {
			T::PalletId::get().into_account_truncating()
		}

		pub fn pot() -> (T::AccountId, BalanceOf<T>) {
			let account_id = Self::account_id();
			let balance =
					T::Currency::free_balance(&account_id).saturating_sub(T::Currency::minimum_balance());

			(account_id, balance)
		}

		pub fn do_buy_ticket(caller: &T::AccountId, call: &<T as Config>::RuntimeCall) -> DispatchResult {
			info!(target: "do_buy_ticket", " ====  Started do_buy_ticket =========== ");

			let config = Lottery::<T>::get().ok_or(Error::<T>::NotConfigured)?;

			let block_number = frame_system::Pallet::<T>::block_number();

			info!(target: "do_buy_ticket", " ====  block_number =========== {:?} ", block_number);

			ensure!(
				block_number < config.start.saturating_add(config.length),
				Error::<T>::AlreadyEnded
			);

			ensure!(T::ValidateCall::validate_call(call), Error::<T>::InvalidCall);

			let call_index = Self::call_to_index(call)?;
			let tickets_count = TicketsCount::<T>::get();

			let next_tickets_count = tickets_count.checked_add(1).ok_or(ArithmeticError::Overflow)?;
			info!(target: "do_buy_ticket", " ====  next_tickets_count =========== {:?} ", next_tickets_count);

			Participants::<T>::try_mutate(
				&caller,
				|(lottery_index, participants_calls)| -> DispatchResult {
					let index = LotteryIndex::<T>::get();

					if *lottery_index != index {
						*lottery_index	= index;
						*participants_calls = Default::default();
					} else {
						ensure!(
							!participants_calls.iter().any(|c|*c == call_index),
							Error::<T>::AlreadyParticipating
						)
					}

					info!(target: "do_buy_ticket", " ====  participants_calls =========== {:?} ", participants_calls);

					participants_calls.try_push(call_index).map_err(|_| Error::<T>::TooManyCalls)?;

					T::Currency::transfer(
						caller,
						&Self::account_id(),
						config.price,
						KeepAlive
					)?;

					TicketsCount::<T>::put(next_tickets_count);
					Tickets::<T>::insert(tickets_count, &caller);		

					Ok(())
				}
			)?;

			Self::deposit_event(Event::TicketBought { caller: caller.clone(), ticket_number: tickets_count });

			Ok(())
		}

		pub fn calls_to_indices(calls: &[<T as Config>::RuntimeCall]) -> Result<BoundedVec<CallIndex, T::MaxCalls>, DispatchError> {
			let mut indices = BoundedVec::<CallIndex, T::MaxCalls>::with_bounded_capacity(calls.len());

			info!("Before indices length ====> {:?}", indices);

			for c in calls.iter() {
				let index = Self::call_to_index(c)?;
				indices.try_push(index).map_err(|_| Error::<T>::TooManyCalls)?;
			}
			info!("After indices length ====> {:?}", indices);
			Ok(indices)

		}

		// basic call type params value comes with pallet name and function - it return index values of both.

		pub fn call_to_index(call: &<T as Config>::RuntimeCall) -> Result<CallIndex, DispatchError> {
			let encoded_call = call.encode();
			if encoded_call.len() < 2 {
				return Err(Error::<T>::EncodingFailed.into())
			}

			Ok((encoded_call[0], encoded_call[1]))
		}

		pub fn lucky_draw() -> Option<T::AccountId> {
			match Self::pick_random_ticket(TicketsCount::<T>::get()) {
				None => None,
				Some(ticket) => Tickets::<T>::get(ticket),
			}
		}

		pub fn pick_random_ticket(total: u32) -> Option<u32> {
			if total == 0 {
				return None
			}

			let mut random_number = Self::generate_random_number(0);

			for i in 1..T::MaxGenerateRandom::get() {
				if random_number < <u32>::MAX - <u32>::MAX % total {
					break;
				}
				random_number = Self::generate_random_number(i);
			}

			Some(random_number % total)
		}



		pub fn generate_random_number(seed: u32) -> u32 {
			let (random_seed, _) = T::Randomness::random(&(T::PalletId::get(), seed).encode());
			let random  = <u32>::decode(&mut random_seed.as_ref())
				.expect("secure hashes should always be bigger than u32; qed");
			info!(target: "random number", "random {:?} > {:?} => {}", random, u32::MAX, (random > u32::MAX));
			random
		}

	}

}


