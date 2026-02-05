use common::prelude::*;
#[allow(unused_imports)]
use common::PAGE_SIZE;

use crate::page::PAGE_FIXED_HEADER_LEN;
#[allow(unused_imports)]
use crate::page::{Offset, Page, OFFSET_NUM_BYTES};

use std::mem;

#[allow(dead_code)]
/// The size of a slotID
pub(crate) const SLOT_ID_SIZE: usize = mem::size_of::<SlotId>();
#[allow(dead_code)]
/// The allowed metadata size per slot
pub(crate) const SLOT_METADATA_SIZE: usize = 4;
#[allow(dead_code)]
/// The size of the metadata allowed for the heap page, this is in addition to the page header
pub(crate) const HEAP_PAGE_FIXED_METADATA_SIZE: usize = 8;
pub const NUM_SLOTS_OFFSET: usize = PAGE_FIXED_HEADER_LEN;
pub const FREE_PTR_OFFSET: usize = NUM_SLOTS_OFFSET + OFFSET_NUM_BYTES;


/// This is trait of a HeapPage for the Page struct.
///
/// The page header size is fixed to `PAGE_FIXED_HEADER_LEN` bytes and you will use
/// additional bytes for the HeapPage metadata
/// Your HeapPage implementation can use a fixed metadata of 8 bytes plus 4 bytes per value/entry/slot stored.
/// For example a page that has stored 3 values, we would assume that the fist
/// `PAGE_FIXED_HEADER_LEN` bytes are used for the page metadata, 8 bytes for the HeapPage metadata
/// and 12 bytes for slot meta data (4 bytes for each of the 3 values).
/// This leave the rest free for storing data (PAGE_SIZE-PAGE_FIXED_HEADER_LEN-8-12).
///
/// If you delete a value, you do not need reclaim header space the way you must reclaim page
/// body space. E.g., if you insert 3 values then delete 2 of them, your header can remain 26
/// bytes & subsequent inserts can simply add 6 more bytes to the header as normal.
/// The rest must filled as much as possible to hold values.
pub trait HeapPage {
    // get/set function for heap file and slot metadata
    fn get_num_slots(&self) -> u16;
    fn set_num_slots(&mut self, n: u16);
    fn get_free_ptr(&self) -> u16;
    fn set_free_ptr(&mut self, ptr: u16);
    fn set_slot_metadata(&mut self, slot_id: SlotId, offset: u16, length: u16);
    fn get_slot_offset(&self, slot_id: SlotId) -> u16;
    fn get_slot_length(&self, slot_id: SlotId) -> u16;
    fn compact(&mut self);

    // Do not change these functions signatures (only the function bodies)

    /// Initialize the page struct as a heap page.
    #[allow(dead_code)]
    fn init_heap_page(&mut self);

    /// Attempts to add a new value to this page if there is space available.
    /// Returns Some(SlotId) if it was inserted or None if there was not enough space.
    /// Note that where the bytes are stored in the page does not matter (heap), but it
    /// should not change the slotId for any existing value. This means that
    /// bytes in the page may not follow the slot order.
    /// If a slot is deleted you should reuse the slotId in the future.
    /// The page should always assign the lowest available slot_id to an insertion.
    ///
    /// HINT: You can copy/clone bytes into a slice using the following function.
    /// They must have the same size.
    /// self.data[X..y].clone_from_slice(&bytes);
    #[allow(dead_code)]
    fn add_value(&mut self, bytes: &[u8]) -> Option<SlotId>;

    /// Return the bytes for the slotId. If the slotId is not valid then return None
    #[allow(dead_code)]
    fn get_value(&self, slot_id: SlotId) -> Option<&[u8]>;

    /// Delete the bytes/slot for the slotId. If the slotId is not valid then return None
    /// The slotId for a deleted slot should be assigned to the next added value
    /// The space for the value should be free to use for a later added value.
    /// HINT: Return Some(()) for a valid delete
    #[allow(dead_code)]
    fn delete_value(&mut self, slot_id: SlotId) -> Option<()>;

    /// Update the value for the slotId. If the slotId is not valid or there is not
    /// space on the page return None and leave the old value/slot. If there is space, update the value and return Some(())
    #[allow(dead_code)]
    fn update_value(&mut self, slot_id: SlotId, bytes: &[u8]) -> Option<()>;

    /// A utility function to determine the current size of the header for this page
    /// Will be used by tests. Optional for you to use in your code
    #[allow(dead_code)]
    fn get_header_size(&self) -> usize;

    /// A utility function to determine the total current free space in the page.
    /// This should account for the header space used and space that could be reclaimed if needed.
    /// Will be used by tests. Optional for you to use in your code, but strongly suggested
    #[allow(dead_code)]
    fn get_free_space(&self) -> usize;

    #[allow(dead_code)]
    /// Create an iterator for the page. This should return an iterator that will
    /// return the bytes and the slotId for each value in the page.
    fn iter(&self) -> HeapPageIter<'_>;
}

impl HeapPage for Page {
    fn init_heap_page(&mut self) {
        // heapmetadata: slot number and pointer to data
        let num_slots: u16 = 0;
        self.data[NUM_SLOTS_OFFSET..NUM_SLOTS_OFFSET + OFFSET_NUM_BYTES].copy_from_slice(&num_slots.to_le_bytes());
        let free_pointer: u16 = PAGE_SIZE.try_into().unwrap();
        self.data[FREE_PTR_OFFSET..FREE_PTR_OFFSET + OFFSET_NUM_BYTES].copy_from_slice(&free_pointer.to_le_bytes());

    }

    fn get_num_slots(&self) -> u16 {
        u16::from_le_bytes(self.data[NUM_SLOTS_OFFSET..NUM_SLOTS_OFFSET + OFFSET_NUM_BYTES].try_into().unwrap())
    }
    
    fn set_num_slots(&mut self, n: u16) {
        self.data[NUM_SLOTS_OFFSET..NUM_SLOTS_OFFSET + OFFSET_NUM_BYTES].copy_from_slice(&n.to_le_bytes());
    }
    
    fn get_free_ptr(&self) -> u16 {
        u16::from_le_bytes(self.data[FREE_PTR_OFFSET..FREE_PTR_OFFSET + OFFSET_NUM_BYTES].try_into().unwrap())
    }
    fn set_free_ptr(&mut self, ptr: u16) {
        self.data[FREE_PTR_OFFSET..FREE_PTR_OFFSET + OFFSET_NUM_BYTES].copy_from_slice(&ptr.to_le_bytes());
    }
    
    fn get_slot_offset(&self, slot_id: SlotId) -> u16 {
        let slot_meta_start = PAGE_FIXED_HEADER_LEN + HEAP_PAGE_FIXED_METADATA_SIZE 
                            + (slot_id as usize * SLOT_METADATA_SIZE);
        u16::from_le_bytes(self.data[slot_meta_start..slot_meta_start + OFFSET_NUM_BYTES].try_into().unwrap())
    }

    fn get_slot_length(&self, slot_id: SlotId) -> u16 {
        let slot_meta_start = PAGE_FIXED_HEADER_LEN + HEAP_PAGE_FIXED_METADATA_SIZE 
                            + (slot_id as usize * SLOT_METADATA_SIZE);
        u16::from_le_bytes(self.data[slot_meta_start + OFFSET_NUM_BYTES..slot_meta_start + SLOT_METADATA_SIZE].try_into().unwrap())
    }

    fn set_slot_metadata(&mut self, slot_id: SlotId, offset: u16, length: u16) {
        let slot_meta_start = PAGE_FIXED_HEADER_LEN + HEAP_PAGE_FIXED_METADATA_SIZE 
                            + (slot_id as usize * SLOT_METADATA_SIZE);
        self.data[slot_meta_start..slot_meta_start + OFFSET_NUM_BYTES].copy_from_slice(&offset.to_le_bytes());
        self.data[slot_meta_start + OFFSET_NUM_BYTES..slot_meta_start + SLOT_METADATA_SIZE].copy_from_slice(&length.to_le_bytes());
    }


    fn add_value(&mut self, bytes: &[u8]) -> Option<SlotId> {
        let data_len = bytes.len();

        let num_slots = self.get_num_slots();
        let mut slot_id: Option<SlotId> = None;
        let mut new_slot_flag = true;

        for i in 0..num_slots{
            if self.get_slot_length(i) == 0 {
                slot_id = Some(i);
                new_slot_flag = false;
                break;
            }
        }
        // move to last slot if no free slot
        let slot_id = slot_id.unwrap_or(num_slots);
        
        // check space availability
        let bytes_needed = data_len + if new_slot_flag {SLOT_METADATA_SIZE} else {0};
        if bytes_needed > self.get_free_space() {
            return None;
        }
        let free_ptr = self.get_free_ptr() as usize;
        let new_free_ptr = free_ptr - data_len;
        self.data[new_free_ptr..free_ptr].clone_from_slice(bytes);

        // slot metadata
        self.set_free_ptr(new_free_ptr as u16);
        self.set_slot_metadata(slot_id, new_free_ptr as u16, data_len as u16);

        if new_slot_flag {
            self.set_num_slots(num_slots + 1);
        }

        Some(slot_id)

    }

    fn get_value(&self, slot_id: SlotId) -> Option<&[u8]> {
        let num_slots = self.get_num_slots();
        // check validity 
        if slot_id >= num_slots {
            return None;
        }

        // check if deleted slot
        let slot_offset = self.get_slot_offset(slot_id) as usize;
        let slot_length = self.get_slot_length(slot_id) as usize;

        if slot_length == 0{
            return None;
        }

        Some(&self.data[slot_offset..slot_offset + slot_length])

    }

    fn delete_value(&mut self, slot_id: SlotId) -> Option<()> {
        // use compaction logic
        let num_slots = self.get_num_slots();
        // check validity 
        if slot_id >= num_slots || self.get_slot_length(slot_id) == 0 {
            return None;
        }

        // set slot metadata to zero
        self.set_slot_metadata(slot_id, 0, 0);

        Some(())
    }

    fn update_value(&mut self, slot_id: SlotId, bytes: &[u8]) -> Option<()> {
        panic!("TODO milestone pg");
    }

    #[allow(dead_code)]
    fn get_header_size(&self) -> usize {
        PAGE_FIXED_HEADER_LEN + HEAP_PAGE_FIXED_METADATA_SIZE + (SLOT_METADATA_SIZE * self.get_num_slots() as usize)
    }

    #[allow(dead_code)]
    fn get_free_space(&self) -> usize {
        let mut data_bytes = 0;

        for slot_id in 0..self.get_num_slots() {
            data_bytes += self.get_slot_length(slot_id)
        }

        PAGE_SIZE - self.get_header_size() - data_bytes as usize
    }

    fn iter(&self) -> HeapPageIter<'_> {
        HeapPageIter {
            page: self,
            //TODO milestone pg
            //Initialize with added variables here
        }
    }

    fn compact(&mut self) {
        let mut entries: Vec<(SlotId, Vec<u8>)> = Vec::new();
        // collect all data present in page
        for slot_id in 0..self.get_num_slots() {
            if let Some(data) = self.get_value(slot_id){
                entries.push((slot_id, data.to_vec()));
            }
        }

        // Rewrite bottom up
        let mut free_ptr = PAGE_SIZE;
        for (slot_id, current_value) in entries{
            let value_len = current_value.len();
            free_ptr -= value_len;
            self.data[free_ptr..free_ptr + value_len].copy_from_slice(&current_value);
            self.set_slot_metadata(slot_id, free_ptr as u16, value_len as u16);
        }

        self.set_free_ptr(free_ptr as u16);

    }
}

pub struct HeapPageIter<'a> {
    page: &'a Page,
    //TODO milestone pg
    // Add any variables here
}

impl<'a> Iterator for HeapPageIter<'a> {
    type Item = (&'a [u8], SlotId);

    /// This function will return the next value in the page. It should return
    /// None if there are no more values in the page.
    /// The iterator should return the bytes reference and the slotId for each value in the page as a tuple.
    fn next(&mut self) -> Option<Self::Item> {
        panic!("TODO milestone pg");
    }
}

/// The implementation of IntoIterator which allows an iterator to be created
/// for a page. This should create the PageIter struct with the appropriate state/metadata
/// on initialization.
impl<'a> IntoIterator for &'a Page {
    type Item = (&'a [u8], SlotId);
    type IntoIter = HeapPageIter<'a>;

    fn into_iter(self) -> Self::IntoIter {
        HeapPageIter {
            page: self,
            //TODO milestone pg
            //Initialize with added variables here
        }
    }
}
