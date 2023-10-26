use std::mem::MaybeUninit;

pub struct StackVec<const SIZE: usize, T: Sized> {
    len: u8,
    buf: [MaybeUninit<T>; SIZE],
}

impl<const SIZE: usize, const SMALLER_OR_EQ_SIZE: usize, T: Sized> From<[T; SMALLER_OR_EQ_SIZE]>
    for StackVec<SIZE, T>
{
    fn from(value: [T; SMALLER_OR_EQ_SIZE]) -> Self {
        assert!(SMALLER_OR_EQ_SIZE <= SIZE);

        let mut vec = Self::new();
        for v in value {
            vec.push(v);
        }
        vec
    }
}

impl<const SIZE: usize, T: Sized> StackVec<SIZE, T> {
    pub fn new() -> Self {
        let buf = unsafe { MaybeUninit::uninit().assume_init() };
        Self { len: 0, buf }
    }

    pub fn len(&self) -> u8 {
        self.len
    }

    pub fn is_empty(&self) -> bool {
        self.len == 0
    }

    pub fn push(&mut self, elem: T) {
        assert!(self.len < SIZE as u8);

        self.buf[self.len as usize] = MaybeUninit::new(elem);
        self.len += 1;
    }

    pub fn pop(&mut self) -> Option<T> {
        (self.len > 0).then(|| {
            let val = std::mem::replace(&mut self.buf[self.len as usize], MaybeUninit::uninit());
            self.len -= 1;
            unsafe { val.assume_init() }
        })
    }

    pub fn iter(&self) -> impl Iterator<Item = &T> {
        let slice = &self.buf[..self.len as usize];
        let slice: &[T] = unsafe { std::mem::transmute(slice) };
        slice.iter()
    }
}

impl<const SIZE: usize, T: Sized + Clone> Clone for StackVec<SIZE, T> {
    fn clone(&self) -> Self {
        let mut buf: [MaybeUninit<T>; SIZE] = unsafe { MaybeUninit::uninit().assume_init() };
        for i in 0..self.len {
            let val = unsafe { self.buf[i as usize].assume_init_ref() };
            buf[i as usize] = MaybeUninit::new(val.clone());
        }
        Self { len: self.len, buf }
    }
}

impl<const SIZE: usize, T: Sized + PartialEq + Eq> std::cmp::Eq for StackVec<SIZE, T> {}
impl<const SIZE: usize, T: Sized + PartialEq> std::cmp::PartialEq for StackVec<SIZE, T> {
    fn eq(&self, other: &Self) -> bool {
        if self.len != other.len {
            return false;
        }

        for i in 0..self.len {
            let a = unsafe { self.buf[i as usize].assume_init_ref() };
            let b = unsafe { other.buf[i as usize].assume_init_ref() };
            if a != b {
                return false;
            }
        }

        true
    }
}

impl<const SIZE: usize, T: Sized + std::fmt::Debug> std::fmt::Debug for StackVec<SIZE, T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_list().entries(self.iter()).finish()
    }
}

impl<const SIZE: usize, T: Sized> Drop for StackVec<SIZE, T> {
    fn drop(&mut self) {
        for i in 0..self.len {
            unsafe {
                self.buf[i as usize].assume_init_drop();
            }
        }
    }
}
