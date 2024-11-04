use crate::utils::OptExt;
use std::mem::MaybeUninit;

pub fn into_exactly<T, const N: usize>(mut iter: impl Iterator<Item = T>) -> [T; N] {
    let result = take_n(&mut iter);
    assert!(iter.next().is_none());
    result
}

pub fn take_n<T, const N: usize>(iter: &mut impl Iterator<Item = T>) -> [T; N] {
    [(); N].map(|_| iter.next().unwrap_unreach())
}

pub fn try_take_n<T, const N: usize>(iter: &mut impl Iterator<Item = T>) -> Option<[T; N]> {
    let result: MaybeUninit<[MaybeUninit<T>; N]> = MaybeUninit::uninit();
    let mut result = unsafe { result.assume_init() };

    for i in 0..N {
        if let Some(value) = iter.next() {
            result[i].write(value);
        } else {
            for item in result.iter_mut() {
                unsafe {
                    item.assume_init_drop();
                }
            }
            return None;
        }
    }

    Some(unsafe { result.map(|x| x.assume_init()) })
}
