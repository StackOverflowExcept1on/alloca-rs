#![no_std]

use core::{
    ffi::c_void,
    mem::{self, MaybeUninit},
};

/// Allocates `[u8; size]` memory on stack and invokes `closure` with this slice as argument.
///
/// # Safety
/// This function is safe because `c_with_alloca` (which is internally used) will always returns non-null
/// pointer.
///
/// # Potential segfaults or UB
///
/// When using this function in wrong way your program might get UB or segfault "for free":
/// - Using memory allocated by `with_alloca` outside of it e.g closure is already returned but you somehow
/// managed to store pointer to memory and use it.
/// - Allocating more memory than thread stack size.
///
///   This will trigger segfault on stack overflow.
#[allow(nonstandard_style)]
pub fn with_alloca<R>(size: usize, f: impl FnOnce(&mut [MaybeUninit<u8>]) -> R) -> R {
    unsafe {
        type Callback = unsafe extern "C-unwind" fn(ptr: *mut u8, data: *mut c_void);
        extern "C-unwind" {
            fn c_with_alloca(size: usize, callback: Callback, data: *mut c_void);
        }
        let mut f = Some(f);
        let mut ret = None;
        // &mut (impl FnMut(*mut u8))
        let ref mut f = |ptr: *mut u8| {
            let slice = ::core::slice::from_raw_parts_mut(ptr.cast::<MaybeUninit<u8>>(), size);

            ret = Some(f.take().unwrap()(slice));
        };
        #[inline(always)]
        fn with_F_of_val<F>(_: &mut F) -> Callback
        where
            F: FnMut(*mut u8),
        {
            unsafe extern "C-unwind" fn trampoline<F: FnMut(*mut u8)>(
                ptr: *mut u8,
                data: *mut c_void,
            ) {
                (&mut *data.cast::<F>())(ptr);
            }

            trampoline::<F>
        }

        c_with_alloca(size, with_F_of_val(f), <*mut _>::cast::<c_void>(f));

        ret.unwrap()
    }
}

/// Same as `with_alloca` except it zeroes memory slice.
pub fn with_alloca_zeroed<R>(size: usize, f: impl FnOnce(&mut [u8]) -> R) -> R {
    with_alloca(size, |memory| unsafe {
        core::ptr::write_bytes(memory.as_mut_ptr().cast::<u8>(), 0, size);
        f(core::mem::transmute(memory))
    })
}

/// Allocates `T` on stack space.
pub fn alloca<T, R>(f: impl FnOnce(&mut MaybeUninit<T>) -> R) -> R {
    use mem::{align_of, size_of};

    with_alloca(size_of::<T>() + (align_of::<T>() - 1), |memory| unsafe {
        let mut raw_memory = memory.as_mut_ptr();
        if raw_memory as usize % align_of::<T>() != 0 {
            raw_memory = raw_memory.add(align_of::<T>() - raw_memory as usize % align_of::<T>());
        }
        f(&mut *raw_memory.cast::<MaybeUninit<T>>())
    })
}

#[cfg(test)]
mod tests;
