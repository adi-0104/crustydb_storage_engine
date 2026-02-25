use crate::buffer_pool::buffer_frame::FrameReadGuard;
use crate::buffer_pool::buffer_frame::FrameWriteGuard;
use crate::buffer_pool::mem_pool_trait::MemPool;
use crate::buffer_pool::mem_pool_trait::PageFrameId;
use crate::heap_page::HeapPage;
#[allow(unused_imports)]
use common::ids::AtomicPageId;
use common::prelude::*;
#[allow(unused_imports)]
use std::sync::atomic::Ordering;
use std::sync::Arc;

/// The struct for a heap file.  
pub(crate) struct HeapFile<T: MemPool> {
    c_id: ContainerId,
    bp: Arc<T>,
    last_page: AtomicPageId,
}

/// HeapFile required functions
impl<T: MemPool> HeapFile<T> {
    /// Helper function to fetch a page for read from the buffer pool.
    fn get_page_for_read(&self, page_id: PageId) -> FrameReadGuard<'_> {
        self.bp
            .get_page_for_read(PageFrameId::new(self.c_id, page_id))
            .unwrap()
    }

    /// Helper function to fetch a page for write from the buffer pool.
    fn get_page_for_write(&self, page_id: PageId) -> FrameWriteGuard<'_> {
        self.bp
            .get_page_for_write(PageFrameId::new(self.c_id, page_id))
            .unwrap()
    }

    // helper
    fn check_page_id(&self, page_id: PageId) -> Result<(), CrustyError> {
        if page_id >= self.num_pages() {
            Err(CrustyError::CrustyError(format!(
                "page id {} out of bounds",
                page_id
            )))
        } else {
            Ok(())
        }
    }

    /// Create a brand-new heap file for container `c_id`.
    pub fn new(c_id: ContainerId, mem_pool: Arc<T>) -> Result<Self, CrustyError> {
        // Note that the header page is always page 0, and the data pages start from 1.
        // You may not end up using the header page, but some tests will assume this.

        // Add any extra initialization code in this function.
        // register a container
        // mem_pool.create_container(c_id, false).unwrap();

        // allocate page 0 to header
        mem_pool.create_new_page_for_write(c_id).unwrap();

        let heap_file = HeapFile {
            c_id,
            bp: mem_pool.clone(),
            last_page: AtomicPageId::new(0),
        };
        Ok(heap_file)
    }

    /// Load an existing heap file.
    pub fn load(c_id: ContainerId, mem_pool: Arc<T>) -> Result<Self, CrustyError> {
        // Add any extra initialization code in this function.

        let heap_file = HeapFile {
            c_id,
            bp: mem_pool.clone(),
            last_page: AtomicPageId::new(
                mem_pool
                    .get_max_page_id(c_id)
                    .unwrap_or(1)
                    .saturating_sub(1),
            ),
        };
        Ok(heap_file)
    }

    /// Return the number of pages for this HeapFile.
    /// Return type is PageId (alias for another type) as we cannot have more
    /// pages than PageId can hold.
    pub fn num_pages(&self) -> PageId {
        self.bp.get_max_page_id(self.c_id).unwrap_or(0)
    }

    /// Read a value at (page_id, slot_id) from the heap file.
    pub fn get_val(&self, page_id: PageId, slot_id: SlotId) -> Result<Vec<u8>, CrustyError> {
        // check for valid page
        self.check_page_id(page_id)?;
        let page = self.get_page_for_read(page_id);
        match page.get_value(slot_id) {
            Some(slice) => Ok(slice.to_vec()),
            None => Err(CrustyError::CrustyError(format!(
                "slot id {} not found",
                slot_id
            ))),
        }
    }

    // Delete a value at (page_id, slot_id) from the heap file.
    pub fn delete_val(&self, page_id: PageId, slot_id: SlotId) -> Result<(), CrustyError> {
        self.check_page_id(page_id)?;
        let mut page = self.get_page_for_write(page_id);
        match page.delete_value(slot_id) {
            Some(()) => Ok(()),
            None => Err(CrustyError::CrustyError(format!(
                "slot id {} not found",
                slot_id
            ))),
        }
    }

    pub fn update_val(
        &self,
        page_id: PageId,
        slot_id: SlotId,
        val: &[u8],
    ) -> Result<ValueId, CrustyError> {
        self.check_page_id(page_id)?;
        {
            let mut page = self.get_page_for_write(page_id);
            // Note: handle cases where the value doesnt fit in page
            // check slot
            if slot_id >= page.get_num_slots()
                || page.get_num_slots() == 0
                || page.get_slot_length(slot_id) == 0
            {
                return Err(CrustyError::CrustyError(format!(
                    "slot id {} not found",
                    slot_id
                )));
            }
            if page.update_value(slot_id, val).is_some() {
                return Ok(ValueId {
                    container_id: self.c_id,
                    segment_id: None,
                    page_id: Some(page_id),
                    slot_id: Some(slot_id),
                });
            }
            // drop gaurd at end of cope if None returned
            page.delete_value(slot_id);
        }
        // update failed
        self.add_val(val)
    }

    // This function is not implemented in a thread-safe way. Can cause deadlocks when used in a multi-threaded environment.
    // We do not care about this for now.
    pub fn add_val(&self, val: &[u8]) -> Result<ValueId, CrustyError> {
        let last = self.last_page.load(Ordering::Relaxed);
        if last > 0 {
            let mut page = self.get_page_for_write(last);
            if let Some(slot_id) = page.add_value(val) {
                return Ok(ValueId {
                    container_id: self.c_id,
                    segment_id: None,
                    page_id: Some(last),
                    slot_id: Some(slot_id),
                });
            }
            // continue to next block if add_value returns None
        }
        // if no available page, allocate a new page
        let mut page = self.bp.create_new_page_for_write(self.c_id).unwrap();
        let page_id = page.get_page_id();
        page.init_heap_page();
        let slot_id = page.add_value(val).unwrap();
        self.last_page.store(page_id, Ordering::Relaxed);
        Ok(ValueId {
            container_id: self.c_id,
            segment_id: None,
            page_id: Some(page_id),
            slot_id: Some(slot_id),
        })
    }

    pub fn add_vals(
        &self,
        iter: impl Iterator<Item = Vec<u8>>,
    ) -> Result<Vec<ValueId>, CrustyError> {
        // You can change this function if desired.
        let mut val_ids = Vec::new();
        for val in iter {
            let val_id = self.add_val(&val)?;
            val_ids.push(val_id);
        }
        Ok(val_ids)
    }

    pub fn iter(self: &Arc<Self>) -> HeapFileIter<T> {
        // Create the HeapFileIter
        HeapFileIter::new_from(self.clone(), 1, 0)
    }

    pub fn iter_from(self: &Arc<Self>, page_id: PageId, slot_id: SlotId) -> HeapFileIter<T> {
        // Create the HeapFileIter
        HeapFileIter::new_from(self.clone(), page_id, slot_id)
    }
}

pub struct HeapFileIter<T: MemPool> {
    /// We are providing the elements of the iterator that we used, you are allowed to
    /// use them in the iterator or make changes. If you change the elements, you
    /// will want to change the new_from constructor to use the new elements.
    heapfile: Arc<HeapFile<T>>,
    initialized: bool,
    finished: bool,
    #[allow(dead_code)]
    first_page: PageId,
    current_slot_id: SlotId,
    current_page: Option<FrameReadGuard<'static>>,
    current_page_id: PageId,
}

impl<T: MemPool> HeapFileIter<T> {
    fn new_from(heapfile: Arc<HeapFile<T>>, page_id: PageId, slot_id: SlotId) -> Self {
        HeapFileIter {
            heapfile,
            initialized: false,
            finished: false,
            first_page: page_id,
            current_slot_id: slot_id,
            current_page: None,
            current_page_id: page_id,
        }
    }

    // Helper function to get a page for read from the buffer pool.
    fn get_page(&self, page_id: PageId) -> FrameReadGuard<'static> {
        // Safety: self.heapfile object has a reference to the buffer pool
        // which makes sure that the frame is not deallocated while this
        // (self) object is alive.
        let page = self.heapfile.get_page_for_read(page_id);
        unsafe { std::mem::transmute::<FrameReadGuard, FrameReadGuard<'static>>(page) }
    }

    fn initialize(&mut self) {
        if self.initialized {
            return;
        }
        // If any work is needed to be done to initialize the iterator, do it here.
        self.initialized = true;
    }
}

impl<T: MemPool> Iterator for HeapFileIter<T> {
    type Item = (Vec<u8>, ValueId);

    /// This function is called to get the next element of the iterator.
    /// It should return None when the iterator is finished.
    /// Otherwise it should return Some((val, val_id)).
    /// The val is the value that was read from the heap file.
    /// The val_id is the ValueId that was read from the heap file.
    fn next(&mut self) -> Option<Self::Item> {
        // Initialize the iterator
        if !self.initialized {
            self.initialize();
        }
        if self.finished {
            return None;
        }

        // Implement the iterator logic
        // load first page
        if self.current_page.is_none() {
            // check if reached end of heapfile
            if self.current_page_id >= self.heapfile.num_pages() {
                self.finished = true;
                return None;
            }
            self.current_page = Some(self.get_page(self.current_page_id));
        }

        while self.current_page_id < self.heapfile.num_pages() {
            let page = self.current_page.as_ref().unwrap();
            let num_slots = page.get_num_slots();

            if self.current_slot_id < num_slots {
                let slot_id = self.current_slot_id;
                self.current_slot_id += 1;

                if let Some(val) = page.get_value(slot_id) {
                    return Some((
                        val.to_vec(),
                        ValueId {
                            container_id: self.heapfile.c_id,
                            segment_id: None,
                            page_id: Some(self.current_page_id),
                            slot_id: Some(slot_id),
                        },
                    ));
                }
            } else {
                // next page
                self.current_page = None;
                self.current_page_id += 1;
                // reset slot_id
                self.current_slot_id = 0;

                if self.current_page_id >= self.heapfile.num_pages() {
                    self.finished = true;
                    return None;
                }
                self.current_page = Some(self.get_page(self.current_page_id))
            }
        }
        None
    }
}
