use alkanes_support::id::AlkaneId;
use alkanes_support::parcel::AlkaneTransferParcel;
use alkanes_support::storage::StorageMap;
use alkanes_support::utils::overflow_error;
use anyhow::{anyhow, Result};
use bitcoin::OutPoint;
use metashrew_core::index_pointer::{AtomicPointer, IndexPointer};
#[allow(unused_imports)]
use metashrew_core::{
    println,
    stdio::{stdout, Write},
};
use metashrew_support::index_pointer::KeyValuePointer;
use protorune_support::rune_transfer::RuneTransfer;
use protorune_support::utils::consensus_decode;
use std::io::Cursor;
use std::sync::Arc;

pub fn from_protobuf(v: alkanes_support::proto::alkanes::AlkaneId) -> AlkaneId {
    AlkaneId {
        block: v.block.unwrap().into(),
        tx: v.tx.unwrap().into(),
    }
}

pub fn balance_pointer(
    atomic: &mut AtomicPointer,
    who: &AlkaneId,
    what: &AlkaneId,
) -> AtomicPointer {
    let who_bytes: Vec<u8> = who.clone().into();
    let what_bytes: Vec<u8> = what.clone().into();
    let ptr = atomic
        .derive(&IndexPointer::default())
        .keyword("/alkanes/")
        .select(&what_bytes)
        .keyword("/balances/")
        .select(&who_bytes);
    if ptr.get().len() != 0 {
        alkane_inventory_pointer(who).append(Arc::new(what_bytes));
    }
    ptr
}

pub fn alkane_inventory_pointer(who: &AlkaneId) -> IndexPointer {
    let who_bytes: Vec<u8> = who.clone().into();
    let ptr = IndexPointer::from_keyword("/alkanes")
        .select(&who_bytes)
        .keyword("/inventory/");
    ptr
}

pub fn alkane_id_to_outpoint(alkane_id: &AlkaneId) -> Result<OutPoint> {
    let alkane_id_bytes: Vec<u8> = alkane_id.clone().into();
    let outpoint_bytes = IndexPointer::from_keyword("/alkanes_id_to_outpoint/")
        .select(&alkane_id_bytes)
        .get()
        .as_ref()
        .clone();
    if outpoint_bytes.len() == 0 {
        return Err(anyhow!("No creation outpoint for alkane id"));
    }
    let outpoint = consensus_decode::<OutPoint>(&mut Cursor::new(outpoint_bytes))?;
    Ok(outpoint)
}

pub fn credit_balances(
    atomic: &mut AtomicPointer,
    to: &AlkaneId,
    runes: &Vec<RuneTransfer>,
) -> Result<()> {
    for rune in runes.clone() {
        let mut ptr = balance_pointer(atomic, to, &rune.id.clone().into());
        ptr.set_value::<u128>(
            rune.value
                .checked_add(ptr.get_value::<u128>())
                .ok_or("")
                .map_err(|_| anyhow!("balance overflow during credit_balances"))?,
        );
    }
    Ok(())
}

pub fn debit_balances(
    atomic: &mut AtomicPointer,
    to: &AlkaneId,
    runes: &AlkaneTransferParcel,
) -> Result<()> {
    for rune in runes.0.clone() {
        let mut pointer = balance_pointer(atomic, to, &rune.id.clone().into());
        let pointer_value = pointer.get_value::<u128>();
        let v = {
            // NOTE: we intentionally allow alkanes to mint an infinite amount of themselves
            // It is up to the contract creator to ensure that this functionality is not abused.
            // Alkanes should not be able to arbitrarily mint alkanes that is not itself
            if *to == rune.id {
                match pointer_value.checked_sub(rune.value) {
                    Some(value) => value,
                    None => pointer_value,
                }
            } else {
                overflow_error(pointer_value.checked_sub(rune.value))?
            }
        };
        pointer.set_value::<u128>(v);
    }
    Ok(())
}

pub fn transfer_from(
    parcel: &AlkaneTransferParcel,
    atomic: &mut AtomicPointer,
    from: &AlkaneId,
    to: &AlkaneId,
) -> Result<()> {
    for transfer in &parcel.0 {
        let mut from_pointer =
            balance_pointer(atomic, &from.clone().into(), &transfer.id.clone().into());
        let mut balance = from_pointer.get_value::<u128>();
        if balance < transfer.value {
            if &transfer.id == from {
                balance = transfer.value;
            } else {
                return Err(anyhow!("balance underflow during transfer_from"));
            }
        }
        from_pointer.set_value::<u128>(balance - transfer.value);
        let mut to_pointer =
            balance_pointer(atomic, &to.clone().into(), &transfer.id.clone().into());
        to_pointer.set_value::<u128>(to_pointer.get_value::<u128>() + transfer.value);
    }
    Ok(())
}
pub fn pipe_storagemap_to<T: KeyValuePointer>(map: &StorageMap, pointer: &mut T) {
    map.0.iter().for_each(|(k, v)| {
        pointer
            .keyword("/storage/")
            .select(k)
            .set(Arc::new(v.clone()));
    });
}
