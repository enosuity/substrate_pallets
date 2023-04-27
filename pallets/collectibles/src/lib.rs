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
	// #[pallet::generate_store(pub(super) trait Store)]
	pub struct Pallet<T>(_);
	
	#[derive(Clone, Encode, Decode, PartialEq, Copy, RuntimeDebug, TypeInfo, MaxEncodedLen)]
	pub enum Color {
		Red,
		Yellow,
		Blue,
		Green
	}

	#[derive(Clone, Encode, Decode, PartialEq, Copy, RuntimeDebug, TypeInfo, MaxEncodedLen)]
	#[scale_info(skip_type_params(T))]
	pub struct Collectible<T: Config> {
		pub unique_id: [u8; 16],
		pub price: Option<BalanceOf<T>>,
		pub color: Color,
		pub owner: T::AccountId,
	}

	#[pallet::config]
	pub trait Config: frame_system::Config {
		type Currency: Currency<Self::AccountId>;
		type CollectionRandomness: Randomness<Self::Hash, Self::BlockNumber>;

		// The aggregated event type of the runtime.
		type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;

		#[pallet::constant]
		type MaximumOwned: Get<u32>;
	}

	type BalanceOf<T> = <<T as Config>::Currency as Currency<<T as frame_system::Config>::AccountId>>::Balance;
	
	#[pallet::storage]
	pub(super) type CollectibleCount<T: Config> = StorageValue<_, u64, ValueQuery>;

	#[pallet::storage]
	pub(super) type CollectibleMap<T: Config> = StorageMap<_, Twox64Concat, [u8; 16], Collectible<T>>;

	#[pallet::storage]
	pub(super) type OwnerOfCollectibles<T: Config> = StorageMap<
		_,
		Twox64Concat,
		T::AccountId,
		BoundedVec<[u8; 16], T::MaximumOwned>,
		ValueQuery,
	>;

	#[pallet::error]
	pub enum Error<T> {
		/// Each collectible must have a unique identifier
		DuplicateCollectible,
		/// An account can't exceed the `MaximumOwned` constant
		MaximumCollectiblesOwned,
		/// The total supply of collectibles can't exceed the u64 limit
		BoundsOverflow,

		/// The Collectible does not exist
		NoCollectible,

		/// you are not owner
		NotOwner,

		/// Trying to transfer a collectible to yourself.
		TransferToSelf,	
		
		/// Bid Price is lower than set price
		BidPriceTooLow,
		
		// when collectible has None price.
		NotForSale,	
	}

	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config> {
		CollectibleCreated {
			collectible: [u8; 16],
			owner: T::AccountId,
		},

		TransferSucceeded {
			from: T::AccountId,
			to:   T::AccountId,
			collectible: [u8;16],
		},

		PriceSet {
			collectible: [u8;16],
			price: Option<BalanceOf<T>>
		},

		Sold {
			to: T::AccountId,
			from: T::AccountId,
			collectible: [u8; 16],
			price: BalanceOf<T>
		},
	}

	// Pallet callable functions
	#[pallet::call]
	impl<T: Config> Pallet<T> {

		#[pallet::weight(0)]
		pub fn create_collectible(
			origin: 	OriginFor<T>
		) -> DispatchResult {
			let to = ensure_signed(origin)?;

			let (collectible_gen_unique_id, color) = Self::gen_unique_id();
			
			Self::mint(collectible_gen_unique_id, color, &to)?;

			Ok(())
		}

		#[pallet::weight(0)]
		pub fn set_price(
			origin: OriginFor<T>,
			unique_id: [u8; 16],
			new_price: Option<BalanceOf<T>>
		 ) -> DispatchResult {
			let sender = ensure_signed(origin)?;

			
			let mut collectible = CollectibleMap::<T>::get(&unique_id)
				.ok_or(Error::<T>::NoCollectible)?;

			ensure!(sender == collectible.owner, Error::<T>::NotOwner);
			collectible.price = new_price;

			CollectibleMap::<T>::insert(&unique_id, collectible);

			Self::deposit_event(Event::PriceSet { collectible: unique_id, price: new_price });

			Ok(())
		}

		#[pallet::weight(0)]
		pub fn transfer(
			origin: 	OriginFor<T>,
			to: 			T::AccountId,
			unique_id: [u8; 16] 
		) -> DispatchResult{
				let from = ensure_signed(origin)?;

				let collectible = CollectibleMap::<T>::get(&unique_id)
					.ok_or(Error::<T>::NoCollectible)?;

				ensure!(collectible.owner == from, Error::<T>::NotOwner);
				
				Self::do_transfer(collectible.unique_id, to)?;

				Ok(())
		}
		
		#[pallet::weight(0)]
		pub fn buy_collectible(
			origin: OriginFor<T>,
			unique_id: [u8; 16],
			price: BalanceOf<T>
		) -> DispatchResult {

			let buyer = ensure_signed(origin)?;

			Self::do_buy_collectible(buyer, price, unique_id)?;	

			Ok(())
		}
	}



	impl <T: Config> Pallet<T> {
		fn gen_unique_id() -> ([u8; 16], Color) {			
			
			// String being used for randomness.
			let subject = &b"Anuj Dhiman"[..];
			// Creating random 
			let random = T::CollectionRandomness::random(subject).0;

			let extrinsic_index = frame_system::Pallet::<T>::extrinsic_index().unwrap_or_default();

			let block_number = frame_system::Pallet::<T>::block_number();

			let unique_payload = (random, extrinsic_index, block_number);
			let encode_payload = unique_payload.encode();

			let hash = frame_support::Hashable::blake2_128(&encode_payload);
			log::info!("gen_unique_id hash ============> {:?}", hash);

			if hash[0] % 2 == 0 {
				(hash, Color::Red)
			} else {
				(hash, Color::Yellow)
			}
		}

		fn mint(
			unique_id: [u8; 16],
			color: Color,
			owner: &T::AccountId,
		) -> Result<[u8; 16], DispatchError> {
			let collectible = Collectible::<T> {unique_id, price: None,	color, owner: owner.clone()	};

			ensure!(
				!CollectibleMap::<T>::contains_key(&collectible.unique_id),
				Error::<T>::DuplicateCollectible
			);

			let count = CollectibleCount::<T>::get();
			let new_count = count.checked_add(1).ok_or(Error::<T>::BoundsOverflow)?;

			OwnerOfCollectibles::<T>::try_append(
				&owner,
				collectible.unique_id
			)
			.map_err(|_| Error::<T>::MaximumCollectiblesOwned)?;

			CollectibleMap::<T>::insert(collectible.unique_id, collectible);
			CollectibleCount::<T>::put(new_count);

			// Deposit the CollectibleCreated Event
			Self::deposit_event(Event::CollectibleCreated {
				collectible: unique_id,
				owner: owner.clone(),
			});


			Ok(unique_id)
		}

		fn do_buy_collectible(
			to: T::AccountId,
			bid_price: BalanceOf<T>,
			unique_id: [u8; 16],
		) -> DispatchResult {

			let mut collectible = CollectibleMap::<T>::get(&unique_id)
				.ok_or(Error::<T>::NoCollectible)?;

			// ensure!(collectible.price != None, Error::<T>::NotForSale);

			let from = collectible.owner;

			ensure!(to != from, Error::<T>::TransferToSelf);

			let mut from_owned = OwnerOfCollectibles::<T>::get(&from);

			if let Some(ind) = from_owned.iter().position(|&id| id == unique_id) {
				from_owned.swap_remove(ind);
			}
			else {
				return Err(Error::<T>::NoCollectible.into());
			} 

			let mut to_owned = OwnerOfCollectibles::<T>::get(&to);

			to_owned.try_push(unique_id).map_err(|_| Error::<T>::MaximumCollectiblesOwned)?;

			if let Some(price) = collectible.price {
				ensure!(bid_price >= price, Error::<T>::BidPriceTooLow);
				
				T::Currency::transfer(&to, &from,	price, frame_support::traits::ExistenceRequirement::KeepAlive)?;

				Self::deposit_event(Event::Sold {
					to: to.clone(),
					from: from.clone(),
					collectible: unique_id,
					price: price, 
				});
			} else {
				return Err(Error::<T>::NotForSale.into());
			}

			collectible.owner = to.clone();
			collectible.price = None;


			CollectibleMap::<T>::insert(&unique_id, collectible);
			OwnerOfCollectibles::<T>::insert(&from, from_owned);
			OwnerOfCollectibles::<T>::insert(&to, to_owned);

			Self::deposit_event(Event::TransferSucceeded { 
				from: from,
				to: to,
				collectible: unique_id
			});

			Ok(())
		}

		fn do_transfer(
			collectible_id: [u8; 16],
			to: T::AccountId
		) -> DispatchResult {

			let mut collectible = CollectibleMap::<T>::get(collectible_id)
				.ok_or(Error::<T>::NoCollectible)?;
			let from = collectible.owner;

			ensure!(to != from, Error::<T>::TransferToSelf);


			let mut from_owned = OwnerOfCollectibles::<T>::get(&from);

			if let Some(ind) = from_owned.iter().position(|&id| id == collectible_id) {
				from_owned.swap_remove(ind);
			} else {
				return Err(Error::<T>::NoCollectible.into())
			}

			let mut to_owned = OwnerOfCollectibles::<T>::get(&to);
			to_owned.try_push(collectible_id).map_err(|_id| Error::<T>::MaximumCollectiblesOwned)?;

			collectible.owner = to.clone();
			collectible.price = None;

			CollectibleMap::<T>::insert(&collectible_id, collectible);
			OwnerOfCollectibles::<T>::insert(&to, to_owned);
			OwnerOfCollectibles::<T>::insert(&from, from_owned);

			Self::deposit_event(Event::TransferSucceeded { from, to, collectible: collectible_id});

			Ok(())
		}
	}

}


