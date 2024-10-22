use std::{cell::RefCell, ptr::slice_from_raw_parts_mut, rc::Rc};

use libafl_bolts::shmem::ShMem;

#[derive(Clone)]
pub struct ShmemNetDeviceBuffers<S>
where
    S: ShMem,
{
    shmem: Rc<RefCell<S>>,
    offset: usize,
}

impl<S> ShmemNetDeviceBuffers<S>
where
    S: ShMem,
{
    pub fn new(shmem: Rc<RefCell<S>>) -> Self {
        Self { shmem, offset: 0 }
    }

    pub fn into_rx(mut self) -> Self {
        self = self.clone();
        self.offset = self.shmem.borrow().len() / 2;
        self
    }

    fn get_ptr(&mut self) -> *mut u8 {
        let res = self
            .shmem
            .borrow_mut()
            .as_mut_ptr()
            .wrapping_byte_add(self.offset);
        log::debug!(
            "value at ptr for offset {}: {}",
            self.offset,
            *cast_to_i32(res)
        );
        res
    }

    pub fn is_empty(&mut self) -> bool {
        *cast_to_i32(self.get_ptr()) < 0
    }

    pub fn set_empty(&mut self) {
        *cast_to_i32(self.get_ptr()) = -1
    }

    pub fn get_size(&mut self) -> &mut i32 {
        cast_to_i32(self.get_ptr())
    }

    pub fn send(&mut self, len: usize) {
        *cast_to_i32(self.get_ptr()) = len.try_into().unwrap();
    }

    pub fn prep_data(&mut self, len: usize) -> &mut [u8] {
        if len > self.shmem.borrow().len() / 2 - 4 {
            panic!("Attempting to prepare a slice larger than the shmem");
        }
        let ptr = self.get_ptr().wrapping_byte_add(4);
        unsafe { slice_from_raw_parts_mut(ptr, len).as_mut() }.unwrap()
    }

    pub fn get_data_and_set_empty(&mut self) -> Option<Vec<u8>> {
        (*self.get_size() >= 0).then(|| {
            let ptr = self.get_ptr().wrapping_byte_add(4);
            let size = (*self.get_size()).try_into().unwrap();
            let slice = unsafe { slice_from_raw_parts_mut(ptr, size).as_mut() }.unwrap();
            let vec = slice.to_vec();
            self.set_empty();
            vec
        })
    }
}

fn cast_to_i32<'a>(ptr: *mut u8) -> &'a mut i32 {
    unsafe { ptr.cast::<i32>().as_mut() }.unwrap()
}
