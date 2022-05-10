use std::marker::PhantomData;
use std::sync::atomic::AtomicUsize;
use std::sync::atomic::Ordering::{Relaxed, Release};

pub trait GenIndexConfig {
    const MAX_HEAP_SIZE: usize;
}

pub struct DefaultMaxHeapSize;

impl GenIndexConfig for DefaultMaxHeapSize {
    /// Assume that the heap can only reach a max size of 1TB. This should far exceed any current
    /// consumer hardware and is more than enough for all but the largest servers as well.
    #[cfg(target_pointer_width = "64")]
    const MAX_HEAP_SIZE: usize = 1usize << 40;

    /// There isn't really a good size which provides enough bytes for a generational index without
    /// greatly reducing the maximum size of the heap. I settled on a size of 1GB for the default,
    /// but at this size, it really needs to be a separate entry.
    #[cfg(target_pointer_width = "32")]
    const MAX_SIZE: usize = 1usize << 30;
}

#[repr(transparent)]
#[derive(Copy, Clone)]
pub struct CompressedGenIndex<C> {
    ptr: usize,
    _phantom: PhantomData<C>,
}

impl<C: GenIndexConfig> CompressedGenIndex<C> {
    const PTR_MASK: usize = C::MAX_HEAP_SIZE.next_power_of_two() - 1;
    const GEN_MASK: usize = !Self::PTR_MASK;

    const GEN_BITS: u32 = Self::GEN_MASK.count_ones();
    const PTR_BITS: u32 = Self::PTR_MASK.count_ones();

    pub fn next_for<T>(offset: usize, new_ptr: *mut T, target: &AtomicUsize) -> Self {
        let offset_ptr = new_ptr as usize - offset;
        assert_eq!(offset_ptr & Self::PTR_MASK, offset_ptr);

        let mut gen: usize = 0;
        let _ = target.fetch_update(Release, Relaxed, |value| {
            gen = (value & Self::GEN_MASK) >> Self::PTR_BITS;

            let max_generation = (1 << Self::GEN_BITS) - 1;
            debug_assert!(Self::GEN_BITS == 0 || gen != max_generation, "Generational index will overflow (max generation: {})", max_generation);

            Some(gen.wrapping_add(1).wrapping_shl(Self::PTR_BITS) | offset_ptr)
        });

        CompressedGenIndex {
            ptr: gen,
            _phantom: PhantomData,
        }
    }

    #[inline(always)]
    fn split_gen_address<T>(gen_addr: usize, offset: usize) -> (usize, *mut T) {
        let generation = gen_addr & Self::GEN_MASK;
        let ptr = (offset + (gen_addr & Self::PTR_MASK)) as *mut T;
        (generation, ptr)
    }

    #[inline]
    pub unsafe fn to_ptr<T>(&self, offset: usize) -> Option<*mut T> {
        let (gen1, ref_table) = Self::split_gen_address(self.ptr, offset);
        let (gen2, data) = Self::split_gen_address(*ref_table, offset);

        if gen1 == gen2 { Some(data) } else { None }
    }
}

