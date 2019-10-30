use support::{decl_module, decl_storage, decl_event, StorageValue, StorageMap, dispatch::Result, Parameter, ensure};
use runtime_primitives::traits::{CheckedAdd, CheckedMul, As};
use system::ensure_signed;
use parity_codec::{Encode, Decode};
use rstd::vec::Vec;
use super::xpay_floorplan;
use super::xpay_diner;
use super::xpay_profile;
pub trait Trait: cennzx_spot::Trait {
	type Item: Parameter;
	type ItemId: Parameter + CheckedAdd + Default + From<u8>;
	type Event: From<Event<Self>> + Into<<Self as system::Trait>::Event>;
}

#[derive(Encode, Decode, Default, Clone, PartialEq)]
#[cfg_attr(feature = "std", derive(Debug))]
pub struct Item {
	pub	desc: Vec<u8>,
	pub	image: Vec<u8>,
	pub	ipfs: Vec<u8>
}

#[derive(Encode, Decode, Default, Clone, PartialEq)]
#[cfg_attr(feature = "std", derive(Debug))]
pub struct Floorplan {
	pub address: Vec<u8>,
	pub	cubes: Vec<(usize,i16,i16,i16)>, 
	pub	desc: Vec<u8>,
	pub	image: Vec<u8>,
	pub	ipfs: Vec<u8>
}
#[derive(Encode, Decode, Default, Clone, PartialEq)]
#[cfg_attr(feature = "std", derive(Debug))]
pub struct Profile {
	pub	profile_name: Vec<u8>,
	pub	image: Vec<u8>,
	pub	ipfs: Vec<u8>
}

pub type BalanceOf<T> = <T as generic_asset::Trait>::Balance;
pub type AssetIdOf<T> = <T as generic_asset::Trait>::AssetId;
pub type PriceOf<T> = (AssetIdOf<T>, BalanceOf<T>);

decl_storage! {
	trait Store for Module<T: Trait> as XPay {
		pub Items get(item): map T::ItemId => Option<Item>;
		pub NextItemId get(next_item_id): T::ItemId;
		pub OwnerItemIds get(owner_itemids): map T::AccountId => Vec<T::ItemId>;
		pub DinerItemIds get(diner_items): linked_map u32 => Vec<(T::ItemId,usize,usize)>;
		pub ItemPrices get(item_price): map T::ItemId => Option<PriceOf<T>>;
		pub Floorplans get(floorplan): map T::ItemId =>Option<Floorplan>; //image,desc
		pub OwnerFloormapIds get(owner_floormap_ids): map T::AccountId => Vec<T::ItemId>;
		pub FloorplanNextItemId get(floorplan_next_item_id): T::ItemId;
		pub Profiles get(profile): map T::AccountId =>Option<Profile>;
		
		//pub Floorplan get(floorplanold): linked_map T::AccountId => Vec<(Vec<u8>,Vec<(usize,i16,i16,i16)>)>;
	}
}

decl_module! {
	pub struct Module<T: Trait> for enum Call where origin: T::Origin {
		fn deposit_event<T>() = default;

		pub fn create_item(origin, desc:Vec<u8>,price_asset_id: AssetIdOf<T>,price_amount: BalanceOf<T>,image:Vec<u8>,ipfs:Vec<u8>) -> Result {
			let origin = ensure_signed(origin)?;

			let item_id = Self::next_item_id();
			let item = Item{
				desc,	image,ipfs
			};
			// The last available id serves as the overflow mark and won't be used.
			let next_item_id = item_id.checked_add(&1.into()).ok_or_else(||"No new item id is available.")?;

			<NextItemId<T>>::put(next_item_id);
			let price = (price_asset_id, price_amount);

			<Items<T>>::insert(item_id.clone(), item.clone());
			if <OwnerItemIds<T>>::exists(origin.clone()){
				<OwnerItemIds<T>>::mutate(origin.clone(), |q| {
					q.push(item_id.clone());
				});
			}else{
				let mut v:Vec<T::ItemId> = Vec::new();
				v.push(item_id.clone());
				<OwnerItemIds<T>>::insert(origin.clone(),v);
			}
			<ItemPrices<T>>::insert(item_id.clone(), price.clone());

			Self::deposit_event(RawEvent::ItemCreated(origin, item_id, price));

			Ok(())
		}

		pub fn add_item(origin, item_id: T::ItemId, diner:u32, quantity: u32) -> Result {
			let origin = ensure_signed(origin)?;

			xpay_diner::insert_into_diner::<T>(item_id.clone(),diner)?;
			Self::deposit_event(RawEvent::ItemAdded(origin, item_id.clone(), diner));

			Ok(())
		}

		pub fn remove_item(origin, item_id: T::ItemId,diner:u32, quantity: u32 ) -> Result {
			let origin = ensure_signed(origin)?;
			xpay_diner::remove_from_diner::<T>(item_id.clone(),diner)?;
			Self::deposit_event(RawEvent::ItemRemoved(origin, item_id.clone(), diner));

			Ok(())
		}

		pub fn update_item(origin, item_id: T::ItemId,desc:Vec<u8>,price_asset_id:AssetIdOf<T>, price_amount: BalanceOf<T>, image:Vec<u8>,ipfs:Vec<u8>) -> Result {
			let origin = ensure_signed(origin)?;

			ensure!(<Items<T>>::exists(item_id.clone()), "Item did not exist");
			let price = (price_asset_id, price_amount);
			let item = Item{
				image,desc,ipfs
			};
			<Items<T>>::insert(item_id.clone(),item);
			<ItemPrices<T>>::insert(item_id.clone(), price.clone());

			Self::deposit_event(RawEvent::ItemUpdated(origin, item_id, price));

			Ok(())
		}

		pub fn purchase_item(origin,seller:T::AccountId, item_id: T::ItemId, diner:u32, paying_asset_id: AssetIdOf<T>, paying_amount_max: BalanceOf<T>,quantity: u32) -> Result {
			let origin = ensure_signed(origin)?;
			let item_price = Self::item_price(item_id.clone()).ok_or_else(||"No item price")?;
			let seller_items = <OwnerItemIds<T>>::get(seller.clone());
			seller_items.iter().position(|r| r == &item_id.clone()).ok_or_else(||"Seller has no such item")?;
			let total_price_amount = item_price.1.checked_mul(&As::sa(quantity as u64)).ok_or_else(||"Total price overflow")?;

			if item_price.0 == paying_asset_id {
				// Same asset, GA transfer
				ensure!(total_price_amount <= paying_amount_max, "User paying price too low");
				xpay_diner::purchase_for_diner::<T>(item_id.clone(),diner)?;
				<generic_asset::Module<T>>::make_transfer_with_event(&item_price.0, &origin, &seller, total_price_amount)?;
			} else {
				// Different asset, CENNZX-Spot transfer

				<cennzx_spot::Module<T>>::make_asset_swap_output(
					&origin,                  // buyer
					&seller,                  // recipient
					&paying_asset_id,         // asset_sold
					&item_price.0,            // asset_bought
					total_price_amount,       // buy_amount
					paying_amount_max,  // max_paying_amount
					<cennzx_spot::Module<T>>::fee_rate() // fee_rate
				)?;
			}

			Self::deposit_event(RawEvent::ItemSold(origin, item_id, quantity));

			Ok(())
		}
		pub fn add_floorplan(origin,acc_to_edit:T::AccountId,address:Vec<u8>,cubes:Vec<(usize,i16,i16,i16)>,desc:Vec<u8>,image:Vec<u8>,ipfs:Vec<u8>)->Result{
			let origin = ensure_signed(origin)?;
			ensure!(origin == acc_to_edit, "No permission to add floorplan for other account");
			let item_id = Self::floorplan_next_item_id();
			xpay_floorplan::add_floorplan::<T>(acc_to_edit,item_id,address,cubes,desc,image,ipfs)
		}
		pub fn remove_floorplan(origin,acc_to_edit:T::AccountId,item_id:T::ItemId)->Result{
			let origin = ensure_signed(origin)?;
			ensure!(origin == acc_to_edit, "No permission to remove floorplan for other account");
			xpay_floorplan::remove_floorplan::<T>(origin,item_id)
		}
		pub fn change_floorplan(origin,acc_to_edit:T::AccountId,item_id:T::ItemId,address:Vec<u8>,cubes:Vec<(usize,i16,i16,i16)>,desc:Vec<u8>,image:Vec<u8>,ipfs:Vec<u8>)->Result{
			let origin = ensure_signed(origin)?;
			ensure!(origin == acc_to_edit, "No permission to change floorplan for other account");
			xpay_floorplan::change_floorplan::<T>(item_id,address,cubes,desc,image,ipfs)
		}
		pub fn save_profile(origin,acc_to_edit:T::AccountId,profile_name:Vec<u8>,image:Vec<u8>,ipfs:Vec<u8>)->Result{
			let origin = ensure_signed(origin)?;
			ensure!(origin == acc_to_edit, "No permission to save profile for other account");
			xpay_profile::save_profile::<T>(acc_to_edit,profile_name,image,ipfs)
		}
		pub fn remove_profile(origin,acc_to_edit:T::AccountId)->Result{
			let origin = ensure_signed(origin)?;
			ensure!(origin == acc_to_edit, "No permission to remove profile for other account");
			xpay_profile::remove_profile::<T>(acc_to_edit)
		}
	}
}

decl_event!(
	pub enum Event<T> where
		<T as system::Trait>::AccountId,
		<T as Trait>::ItemId,
		Price = PriceOf<T>,
	{
		/// New item created. (transactor, item_id, quantity, item, price)
		ItemCreated(AccountId, ItemId, Price),
		/// More items added. (transactor, item_id, new_quantity, diner)
		ItemAdded(AccountId, ItemId, u32),
		/// Items removed. (transactor, item_id, new_quantity, diner)
		ItemRemoved(AccountId, ItemId, u32),
		/// Item updated. (transactor, item_id, new_quantity, new_price)
		ItemUpdated(AccountId, ItemId, Price),
		/// Item sold. (transactor, item_id, quantity)
		ItemSold(AccountId, ItemId, u32),
	}
);
