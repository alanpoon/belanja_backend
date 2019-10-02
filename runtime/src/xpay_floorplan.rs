use support::{StorageValue, StorageMap, dispatch::Result,ensure};
use super::xpay::Trait;
use super::xpay;
use runtime_primitives::traits::{CheckedAdd};
use rstd::vec::Vec;
pub fn add_floorplan<T:Trait>(origin: T::AccountId,item_id:T::ItemId,image:Vec<u8>,description:Vec<u8>,ipfs:Vec<u8>,floorplan:Vec<(usize,i16,i16,i16)>)->Result{
  let next_item_id = item_id.checked_add(&1.into()).ok_or_else(||"No new item id is available.")?;
	<xpay::FloorplanNextItemId<T>>::put(next_item_id);
  let fp = xpay::Floorplan::new(image,description,ipfs,floorplan);
	<xpay::Floorplans<T>>::insert(item_id.clone(), fp.clone());
	<xpay::FloorplanOwners<T>>::insert(item_id, origin);
  Ok(())
}
pub fn remove_floorplan<T:Trait>(item_id:T::ItemId)->Result{
  <xpay::Floorplans<T>>::remove(item_id);
  Ok(())
}

pub fn change_floorplan<T:Trait>(item_id:T::ItemId,image:Vec<u8>,description:Vec<u8>,ipfs:Vec<u8>,floorplan:Vec<(usize,i16,i16,i16)>)->Result{
  <xpay::Floorplans<T>>::mutate(item_id,|q|{
    let fp = xpay::Floorplan::new(image,description,ipfs,floorplan);
    *q = Some(fp);
  });
  Ok(())
}