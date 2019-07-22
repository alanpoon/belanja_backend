use support::{decl_module, decl_storage, decl_event, StorageValue, EnumerableStorageMap, StorageMap, dispatch::Result, Parameter, ensure};
use runtime_primitives::traits::{CheckedAdd, CheckedMul, As};
use system::ensure_signed;
use rstd::vec::Vec;
pub trait Trait: cennzx_spot::Trait {
	type Item: Parameter;
	type ItemId: Parameter + CheckedAdd + Default + From<u8>;
	type Event: From<Event<Self>> + Into<<Self as system::Trait>::Event>;
}

pub type BalanceOf<T> = <T as generic_asset::Trait>::Balance;
pub type AssetIdOf<T> = <T as generic_asset::Trait>::AssetId;
pub type PriceOf<T> = (AssetIdOf<T>, BalanceOf<T>);

decl_storage! {
	trait Store for Module<T: Trait> as XPay {
		pub Items get(item): map T::ItemId => Option<T::Item>;
		pub ItemOwners get(item_owner): map T::ItemId => Option<T::AccountId>; //insert(item_id.clone(), origin.clone())
		pub DinerItemIds get(diner_items): linked_map u32 => Vec<(T::ItemId,usize,usize)>;
		pub ItemQuantities get(item_quantity): map T::ItemId => u32;
		pub ItemPrices get(item_price): map T::ItemId => Option<PriceOf<T>>;
		pub NextItemId get(next_item_id): T::ItemId;
	}
}

decl_module! {
	pub struct Module<T: Trait> for enum Call where origin: T::Origin {
		fn deposit_event<T>() = default;

		pub fn create_item(origin, quantity: u32, item: T::Item, price_asset_id: AssetIdOf<T>, price_amount: BalanceOf<T>) -> Result {
			let origin = ensure_signed(origin)?;

			let item_id = Self::next_item_id();

			// The last available id serves as the overflow mark and won't be used.
			let next_item_id = item_id.checked_add(&1.into()).ok_or_else(||"No new item id is available.")?;

			<NextItemId<T>>::put(next_item_id);

			let price = (price_asset_id, price_amount);

			<Items<T>>::insert(item_id.clone(), item.clone());
			<ItemOwners<T>>::insert(item_id.clone(), origin.clone());
			<ItemQuantities<T>>::insert(item_id.clone(), quantity);
			<ItemPrices<T>>::insert(item_id.clone(), price.clone());

			Self::deposit_event(RawEvent::ItemCreated(origin, item_id, quantity, item, price));

			Ok(())
		}

		pub fn add_item(origin, item_id: T::ItemId, quantity: u32, diner:u32) -> Result {
			let origin = ensure_signed(origin)?;

			<ItemQuantities<T>>::mutate(item_id.clone(), |q| *q = q.saturating_add(quantity));
			Self::insert_into_diner(diner,item_id.clone())?;
			Self::deposit_event(RawEvent::ItemAdded(origin, item_id.clone(), Self::item_quantity(item_id),diner));

			Ok(())
		}

		pub fn remove_item(origin, item_id: T::ItemId, quantity: u32, diner:u32) -> Result {
			let origin = ensure_signed(origin)?;
			<ItemQuantities<T>>::mutate(item_id.clone(), |q| *q = q.saturating_sub(quantity));
			Self::remove_from_diner(diner,item_id.clone())?;
			Self::deposit_event(RawEvent::ItemRemoved(origin, item_id.clone(), Self::item_quantity(item_id),diner));

			Ok(())
		}

		pub fn update_item(origin, item_id: T::ItemId, quantity: u32, price_asset_id: AssetIdOf<T>, price_amount: BalanceOf<T>) -> Result {
			let origin = ensure_signed(origin)?;

			ensure!(<Items<T>>::exists(item_id.clone()), "Item did not exist");

			<ItemQuantities<T>>::insert(item_id.clone(), quantity);

			let price = (price_asset_id, price_amount);
			<ItemPrices<T>>::insert(item_id.clone(), price.clone());

			Self::deposit_event(RawEvent::ItemUpdated(origin, item_id, quantity, price));

			Ok(())
		}

		pub fn purchase_item(origin, quantity: u32, item_id: T::ItemId, paying_asset_id: AssetIdOf<T>, max_total_paying_amount: BalanceOf<T>,diner:u32) -> Result {
			let origin = ensure_signed(origin)?;

			let new_quantity = Self::item_quantity(item_id.clone()).checked_sub(quantity).ok_or_else(||"Not enough quantity")?;
			let item_price = Self::item_price(item_id.clone()).ok_or_else(||"No item price")?;
			let seller = Self::item_owner(item_id.clone()).ok_or_else(||"No item owner")?;

			let total_price_amount = item_price.1.checked_mul(&As::sa(quantity as u64)).ok_or_else(||"Total price overflow")?;

			if item_price.0 == paying_asset_id {
				// Same asset, GA transfer
				ensure!(total_price_amount <= max_total_paying_amount, "User paying price too low");
				Self::purchase_for_diner(diner,item_id.clone())?;
				<generic_asset::Module<T>>::make_transfer_with_event(&item_price.0, &origin, &seller, total_price_amount)?;
			} else {
				// Different asset, CENNZX-Spot transfer

				<cennzx_spot::Module<T>>::make_asset_swap_output(
					&origin,                  // buyer
					&seller,                  // recipient
					&paying_asset_id,         // asset_sold
					&item_price.0,            // asset_bought
					total_price_amount,       // buy_amount
					max_total_paying_amount,  // max_paying_amount
					<cennzx_spot::Module<T>>::fee_rate() // fee_rate
				)?;
			}

			<ItemQuantities<T>>::insert(item_id.clone(), new_quantity);

			Self::deposit_event(RawEvent::ItemSold(origin, item_id, quantity));

			Ok(())
		}
		
	}
}

impl<T: Trait> Module<T> {
	pub fn insert_into_diner(diner: u32,item_id:T::ItemId)->Result{
		if <DinerItemIds<T>>::exists(diner){
			<DinerItemIds<T>>::mutate(diner,|q|{
				for (v,_,unpaid) in q.iter_mut(){
					if *v ==item_id{
						*unpaid=unpaid.clone()+1;
					}
				}
			});
		}else{
			let mut v:Vec<(T::ItemId,usize,usize)> = Vec::new();
			v.push((item_id,0,1));
			<DinerItemIds<T>>::insert(diner,v);
		}
		Ok(())
	}
	pub fn remove_from_diner(diner:u32,item_id:T::ItemId)->Result{
		let mut fail = false;
		let mut fail_unpaid = false;
		if <DinerItemIds<T>>::exists(diner){
			<DinerItemIds<T>>::mutate(diner, |q| {
				if let Some(index) = q.iter().position(|x| (*x).0 == item_id){
					if let Some((_,_,unpaid)) = q.get_mut(index){
						if *unpaid==0{
							fail_unpaid = true;
						}
						*unpaid = unpaid.clone() -1;
					}
				}else{
					fail = true;
				}
				
			});
		}else{
			ensure!(false,"Diner does not exist");
		}
		if fail{
			ensure!(false,"Item does not exist in Diner");
		}
		if fail_unpaid{
			ensure!(false,"There is no unpaid item to remove");
		}
		Ok(())
	}
	pub fn purchase_for_diner(diner:u32,item_id:T::ItemId)->Result{
		let mut fail = false;
		let mut fail_unpaid = false;
		if <DinerItemIds<T>>::exists(diner){
			<DinerItemIds<T>>::mutate(diner, |q| {
				if let Some(index) = q.iter().position(|x| (*x).0 == item_id){
					if let Some((_,paid,unpaid)) = q.get_mut(index){
						if *unpaid ==0{
							fail_unpaid = true;
						}
						*paid = paid.clone()+1;
						*unpaid = unpaid.clone() -1;
					}
				}else{
					fail = true;
				}
				
			});
		}else{
			ensure!(false,"Diner does not exist");
		}
		if fail{
			ensure!(false,"Item does not exist in Diner");
		}
		if fail_unpaid{
			ensure!(false,"There is no unpaid item to pay for");
		}
		Ok(())
	}
}

decl_event!(
	pub enum Event<T> where
		<T as system::Trait>::AccountId,
		<T as Trait>::Item,
		<T as Trait>::ItemId,
		Price = PriceOf<T>,
	{
		/// New item created. (transactor, item_id, quantity, item, price)
		ItemCreated(AccountId, ItemId, u32, Item, Price),
		/// More items added. (transactor, item_id, new_quantity, diner)
		ItemAdded(AccountId, ItemId, u32, u32),
		/// Items removed. (transactor, item_id, new_quantity, diner)
		ItemRemoved(AccountId, ItemId, u32, u32),
		/// Item updated. (transactor, item_id, new_quantity, new_price)
		ItemUpdated(AccountId, ItemId, u32, Price),
		/// Item sold. (transactor, item_id, quantity)
		ItemSold(AccountId, ItemId, u32),
	}
);
