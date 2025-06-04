use std::fmt::Debug;

use super::is_main_thread;

/// Based upon the fragile crate but made more simple for our purposes
pub struct Fragile<T>(T);

impl<T: Debug> Debug for Fragile<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if is_main_thread() {
            f.debug_tuple("Fragile").field(&self.0).finish()
        } else {
            f.debug_tuple("Fragile").field(&"< unavailable >").finish()
        }
    }
}

impl<T> Fragile<T> {
    pub fn new(value: T) -> Self {
        assert!(
            is_main_thread(),
            "Cannot create this type outside the main thread"
        );

        Fragile(value)
    }

    #[inline]
    pub fn get(&self) -> Option<&T> {
        if is_main_thread() {
            Some(&self.0)
        } else {
            None
        }
    }
}

impl<T> Drop for Fragile<T> {
    fn drop(&mut self) {
        if std::mem::needs_drop::<T>() {
            assert!(
                is_main_thread(),
                "Cannot drop this type outside the main thread"
            );
        }
    }
}

impl<T: Default> Default for Fragile<T> {
    fn default() -> Fragile<T> {
        Fragile::new(T::default())
    }
}

/// SAFETY: This struct makes sure access and drop is done on the main thread
unsafe impl<T> Sync for Fragile<T> {}
unsafe impl<T> Send for Fragile<T> {}
