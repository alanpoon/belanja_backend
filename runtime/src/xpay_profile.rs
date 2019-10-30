use support::{StorageValue, StorageMap, dispatch::Result,ensure};
use super::xpay::Trait;
use super::xpay;
use runtime_primitives::traits::{CheckedAdd};
use rstd::vec::Vec;
pub fn save_profile<T:Trait>(acc_to_edit:T::AccountId,profile_name:Vec<u8>,image:Vec<u8>,ipfs:Vec<u8>)->Result{
  if <xpay::Profile<T>>::exists(acc_to_edit){
    <xpay::Floorplans<T>>::mutate(acc_to_edit,|q|{
    let fp=xpay::Profile{
      profile_name,image,ipfs
    };
    *q = Some(fp);
  });
  }else{
    let fp = xpay::Profile{
      profile_name,image,ipfs
      };
    <xpay::Profile<T>>::insert(acc_to_edit, fp);
  }
  Ok(())
}
pub fn remove_profile<T:Trait>(acc_to_edit:T::AccountId)->Result{
  <xpay::Profile<T>>::remove(acc_to_edit);
  Ok(())
}