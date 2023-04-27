#![cfg_attr(not(feature = "std"), no_std)]

pub use pallet::*;

#[frame_support::pallet]
pub mod pallet {
	use frame_support::{
		pallet_prelude::{*, ValueQuery},
		traits::{Currency, Randomness},
	};
	use frame_system::{pallet_prelude::{*, OriginFor}, ensure_signed};

	#[pallet::pallet]
	#[pallet::generate_store(pub(super) trait Store)]
	pub struct Pallet<T>(_);
	
	

	#[pallet::config]
	pub trait Config: frame_system::Config {
		type Currency: Currency<Self::AccountId>;
		type CollectionRandomness: Randomness<Self::Hash, Self::BlockNumber>;

		// The aggregated event type of the runtime.
		type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;
		
	}

	type BalanceOf<T> = <<T as Config>::Currency as Currency<<T as frame_system::Config>::AccountId>>::Balance;
	


	#[pallet::error]
	pub enum Error<T> {
		
	}

	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config> {
		
	}

	// Pallet callable functions
	#[pallet::call]
	impl<T: Config> Pallet<T> {

		#[pallet::weight(0)]
		pub fn set_price(
			origin: OriginFor<T>,			
		 ) -> DispatchResult {
			let sender = ensure_signed(origin)?;

			Ok(())
		}

	}



	impl <T: Config> Pallet<T> {

	}

}


