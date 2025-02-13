use std::{iter::FusedIterator, marker::PhantomData, ptr::NonNull};

#[derive(Debug)]
pub struct Drain<'a, T> {
    // The value lives as long as 'a, but we are able to safely mutate the values as it is now
    // added to the gap. As long as the gap buffer this originated from is not mutated this is safe
    // to use in any way.
    pub(crate) ptr: NonNull<[T]>,
    pub(crate) __p: PhantomData<&'a T>,
}

impl<T> Drain<'_, T> {

    /// Returns a slice of the remaining elements in the drain
    #[inline(always)]
    pub fn as_slice(&self) -> &[T] {
        unsafe { self.ptr.as_ref() }
    }

    /// Returns a mutable slice of the remaining elements in the drain
    #[inline(always)]
    pub fn as_slice_mut(&mut self) -> &mut [T] {
        unsafe { self.ptr.as_mut() }
    }
}

impl<T> Iterator for Drain<'_, T> {
    type Item = T;
    fn next(&mut self) -> Option<Self::Item> {
        let len = self.ptr.len();
        if len == 0 {
            return None;
        }
        let ptr = self.ptr.cast::<T>();
        // SAFETY: we have checked the remaining length above
        // it is now guaranteed that we at least have one value stored
        unsafe {
            let t = ptr.read();
            // SAFETY: we have read the last value in the slice, to avoid a double drop we shrink the
            // length of our slice
            self.ptr = NonNull::slice_from_raw_parts(ptr.add(1), len - 1);
            Some(t)
        }
    }

    fn nth(&mut self, mut n: usize) -> Option<Self::Item>
    where
        Self: Sized,
    {
        let len = self.ptr.len();
        if n >= len {
            // we must exhaust all of the items and to not return any T's in other calls we
            // call the drop code and set the slice length to 0
            // SAFETY: since T's will never be accessed after this point it is safe to call its drop code
            unsafe { self.ptr.drop_in_place() };
            self.ptr = NonNull::slice_from_raw_parts(self.ptr.cast::<T>(), 0);
            return None;
        }
        let ptr = self.ptr.cast::<T>();

        // go to the requested value and read it
        unsafe {
            let t = ptr.add(n).read();
            // drop all values until the one that was read
            NonNull::slice_from_raw_parts(ptr, n).drop_in_place();

            // we minimally always drop one value in this branch
            // to account for the item that was read, and the ones that were dropped readjust the slice
            // start and length
            n += 1;
            self.ptr = NonNull::slice_from_raw_parts(ptr.add(n), len - n);
            Some(t)
        }
    }

    fn count(mut self) -> usize
    where
        Self: Sized,
    {
        let len = self.ptr.len();
        // drop the items and set the length as the count method should exhaust all items
        // we also have to leave the fields in valid state in case [`Iterator::by_ref`] or
        // similar methods are used
        //
        // same as calling [`Iterator::next`] until None is returned
        // SAFETY: since T's will never be accessed after this point it is safe to call its drop code
        unsafe { self.ptr.drop_in_place() };
        self.ptr = NonNull::slice_from_raw_parts(self.ptr.cast::<T>(), 0);
        len
    }

    fn last(mut self) -> Option<Self::Item>
    where
        Self: Sized,
    {
        let len = self.ptr.len();
        if len == 0 {
            return None;
        }

        let ptr = self.ptr.cast::<T>();
        // SAFETY: we have checked if the length is 0 and we decrement the length without any
        // wrapping
        // we can't have a double drop as the value is returned to the user with its own drop code
        // at the end of the function
        let t = unsafe { ptr.add(len - 1).read() };
        self.ptr = NonNull::slice_from_raw_parts(ptr, 0);

        Some(t)
    }
}

impl<T> DoubleEndedIterator for Drain<'_, T> {
    fn next_back(&mut self) -> Option<Self::Item> {
        let len = self.ptr.len();
        if len == 0 {
            return None;
        }
        let ptr = self.ptr.cast::<T>();

        // we already checked if len is zero above, this cannot wrap
        let t = unsafe { ptr.add(len - 1).read() };
        self.ptr = NonNull::slice_from_raw_parts(ptr, len - 1);
        Some(t)
    }
}

impl<T> FusedIterator for Drain<'_, T> {}

impl<T> Drop for Drain<'_, T> {
    fn drop(&mut self) {
        unsafe {
            self.ptr.drop_in_place();
        }
    }
}

// see buf.rs module for drain tests
