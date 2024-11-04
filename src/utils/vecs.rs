use std::cell::{Ref, RefCell};
use std::cmp::Ordering;
use std::mem;
use std::ops::{Deref, DerefMut, Index};

use elsa::FrozenVec;

macro_rules! deref_transparent {
    ( <$($generic: ident)*> $from: ty, $to: ty) => {
        impl<$($generic),* > Deref for $from {type Target = $to;#[inline] fn deref(&self) -> &Self::Target {&self.0}}
        impl<$($generic),* > DerefMut for $from {#[inline] fn deref_mut(&mut self) -> &mut Self::Target {&mut self.0}}
    };
}

#[allow(unused)]
macro_rules! from_into_transparent {
    ( <$($generic: ident)*> $from: ty, $to: ty) => {
        impl<$($generic),* > From<$from> for $to {#[inline] fn from(value: $from) -> Self {value.0}}
        impl<$($generic),* > From<$to> for $from {#[inline] fn from(value: $to) -> Self {Self(value)}}
    };
}

// deref_transparent! { <T> IdVec<T>, Vec<T> }
deref_transparent! { <T> SparseVec<T>, Vec<Option<T>> }
// from_into_transparent! { <T> IdVec<T>, Vec<T> }

pub struct IdVec<T>(FrozenVec<Box<T>>);

impl<T: PartialEq> Default for IdVec<T> {
    #[inline]
    fn default() -> Self {
        Self(FrozenVec::new())
    }
}

impl<T> Index<usize> for IdVec<T> {
    type Output = T;

    fn index(&self, index: usize) -> &Self::Output {
        &self.0[index]
    }
}

impl<T: PartialEq> IdVec<T> {
    #[inline]
    pub fn new() -> Self {
        Self::default()
    }

    pub fn id_or_insert(&self, item: T) -> usize {
        self.0.iter().position(|x| x.eq(&item)).unwrap_or_else(|| {
            let new_id = self.0.len();
            self.0.push(Box::new(item));
            new_id
        })
    }
}

impl<T> IdVec<T> {
    pub fn id_or_insert_with<U>(&self, item: U, func: impl FnOnce() -> T) -> usize
    where
        for<'a> &'a T: PartialEq<U>,
    {
        self.0.iter().position(|x| x.eq(&item)).unwrap_or_else(|| {
            let new_id = self.0.len();
            self.0.push(Box::new(func()));
            new_id
        })
    }
}

pub struct SparseVec<T>(Vec<Option<T>>);

impl<T> Default for SparseVec<T> {
    #[inline]
    fn default() -> Self {
        Self(Vec::new())
    }
}

impl<T> SparseVec<T> {
    #[inline]
    pub fn new() -> Self {
        Self::default()
    }

    #[inline]
    pub fn count_elements(&self) -> usize {
        self.iter().map(|x| usize::from(x.is_some())).sum()
    }

    pub fn get(&self, index: usize) -> Option<&T> {
        self.as_vec().get(index).and_then(|x| x.as_ref())
    }

    #[inline]
    pub fn into_inner(self) -> Vec<Option<T>> {
        self.0
    }
    #[inline]
    pub fn as_vec(&self) -> &Vec<Option<T>> {
        self
    }
    #[inline]
    pub fn as_mut_vec(&mut self) -> &mut Vec<Option<T>> {
        self
    }

    pub fn insert(&mut self, at: usize, value: T) -> Option<T> {
        match at.cmp(&self.len()) {
            Ordering::Greater => {
                self.resize_with(at, || None);
                self.push(Some(value));
            }
            Ordering::Equal => self.push(Some(value)),
            Ordering::Less => {
                return mem::replace(&mut self[at], Some(value));
            }
        }
        None
    }
}
