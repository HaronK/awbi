// use core::ops::{Deref, DerefMut};
use std::cell::RefCell;
use std::rc::Rc;

#[derive(PartialEq, Debug, Default)]
pub struct Ref<T>(Rc<RefCell<T>>);

impl<T> Ref<T> {
    pub fn new(r: T) -> Self {
        Self(Rc::new(RefCell::new(r)))
    }

    pub fn get(&self) -> std::cell::Ref<T> {
        self.0.borrow()
    }

    pub fn get_mut(&self) -> std::cell::RefMut<T> {
        self.0.borrow_mut()
    }
}

impl<T> Clone for Ref<T> {
    fn clone(&self) -> Self {
        Self(self.0.clone())
    }
}

// impl<T> Deref for Ref<T> {
//     type Target = T;

//     #[inline(always)]
//     fn deref(&self) -> &Self::Target {
//         &*self.0.borrow()
//     }
// }

// impl<T> DerefMut for Ref<T> {
//     #[inline(always)]
//     fn deref_mut(&mut self) -> &mut Self::Target {
//         &mut *self.0.borrow_mut()
//     }
// }
