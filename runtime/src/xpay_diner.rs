use support::{StorageValue, StorageMap, dispatch::Result,ensure};
use super::xpay::Trait;
use super::xpay;
use runtime_primitives::traits::{CheckedAdd};
use rstd::vec::Vec;
pub fn insert_into_diner<T:Trait>(item_id:T::ItemId, diner: u32)->Result{
  if <xpay::DinerItemIds<T>>::exists(diner){
    <xpay::DinerItemIds<T>>::mutate(diner,|q|{
      for (v,_,unpaid) in q.iter_mut(){
        if *v ==item_id{
          *unpaid=unpaid.clone()+1;
        }
      }
    });
  }else{
    let mut v:Vec<(T::ItemId,usize,usize)> = Vec::new();
    v.push((item_id,0,1));
    <xpay::DinerItemIds<T>>::insert(diner,v);
  }
  Ok(())
}
pub fn remove_from_diner<T:Trait>(item_id:T::ItemId,diner:u32)->Result{
		let mut fail = false;
		let mut fail_unpaid = false;
		if <xpay::DinerItemIds<T>>::exists(diner){
			<xpay::DinerItemIds<T>>::mutate(diner, |q| {
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
pub fn purchase_for_diner<T:Trait>(item_id:T::ItemId,diner:u32)->Result{
  let mut fail = false;
  let mut fail_unpaid = false;
  if <xpay::DinerItemIds<T>>::exists(diner){
    <xpay::DinerItemIds<T>>::mutate(diner, |q| {
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