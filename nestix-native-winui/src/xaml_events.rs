use std::{
    cell::{Cell, RefCell},
    collections::HashMap,
};

use nestix_native_core::dpi::Size;

thread_local! {
    static NEXT_CALLBACK_ID: Cell<u64> = const { Cell::new(1) };
    static CLICK_CALLBACKS: RefCell<HashMap<u64, nestix::Shared<dyn Fn()>>> = RefCell::new(HashMap::new());
    static SCALE_FACTOR_CALLBACKS: RefCell<HashMap<u64, nestix::Shared<dyn Fn(f64)>>> = RefCell::new(HashMap::new());
    static RESIZE_CALLBACKS: RefCell<HashMap<u64, nestix::Shared<dyn Fn(Size)>>> = RefCell::new(HashMap::new());
    static TAB_SELECTION_CALLBACKS: RefCell<HashMap<u64, nestix::Shared<dyn Fn(String)>>> = RefCell::new(HashMap::new());
    static CONTENT_SIZE_CALLBACKS: RefCell<HashMap<u64, nestix::Shared<dyn Fn(f32, f32)>>> = RefCell::new(HashMap::new());
    static TEXT_CHANGED_CALLBACKS: RefCell<HashMap<u64, nestix::Shared<dyn Fn(String)>>> = RefCell::new(HashMap::new());
    static BOOL_CALLBACKS: RefCell<HashMap<u64, nestix::Shared<dyn Fn(bool)>>> = RefCell::new(HashMap::new());
    static F64_CALLBACKS: RefCell<HashMap<u64, nestix::Shared<dyn Fn(f64)>>> = RefCell::new(HashMap::new());
    static STRING_CALLBACKS: RefCell<HashMap<u64, nestix::Shared<dyn Fn(String)>>> = RefCell::new(HashMap::new());
}

macro_rules! registered_value_callback {
    ($name:ident, $map:ident, $value:ty) => {
        #[derive(Debug)]
        pub(crate) struct $name {
            id: u64,
        }
        impl $name {
            pub fn register(callback: nestix::Shared<dyn Fn($value)>) -> Self {
                let id = next_callback_id();
                $map.with_borrow_mut(|callbacks| {
                    callbacks.insert(id, callback);
                });
                Self { id }
            }
            pub fn id(&self) -> u64 {
                self.id
            }
            pub fn invoke(id: u64, value: $value) {
                let callback = $map.with_borrow(|callbacks| callbacks.get(&id).cloned());
                if let Some(callback) = callback {
                    callback(value);
                }
            }
        }
        impl Drop for $name {
            fn drop(&mut self) {
                $map.with_borrow_mut(|callbacks| {
                    callbacks.remove(&self.id);
                });
            }
        }
    };
}

registered_value_callback!(RegisteredBoolCallback, BOOL_CALLBACKS, bool);
registered_value_callback!(RegisteredF64Callback, F64_CALLBACKS, f64);
registered_value_callback!(RegisteredStringCallback, STRING_CALLBACKS, String);

#[derive(Debug)]
pub(crate) struct RegisteredContentSizeCallback {
    id: u64,
}

impl RegisteredContentSizeCallback {
    pub fn register(callback: nestix::Shared<dyn Fn(f32, f32)>) -> Self {
        let id = next_callback_id();
        CONTENT_SIZE_CALLBACKS.with_borrow_mut(|callbacks| {
            callbacks.insert(id, callback);
        });
        Self { id }
    }

    pub fn id(&self) -> u64 {
        self.id
    }

    pub fn invoke(id: u64, width: f32, height: f32) {
        let callback = CONTENT_SIZE_CALLBACKS.with_borrow(|callbacks| callbacks.get(&id).cloned());
        if let Some(callback) = callback {
            callback(width, height);
        }
    }
}

impl Drop for RegisteredContentSizeCallback {
    fn drop(&mut self) {
        CONTENT_SIZE_CALLBACKS.with_borrow_mut(|callbacks| {
            callbacks.remove(&self.id);
        });
    }
}

#[derive(Debug)]
pub(crate) struct RegisteredTextChangedCallback {
    id: u64,
}

impl RegisteredTextChangedCallback {
    pub fn register(callback: nestix::Shared<dyn Fn(String)>) -> Self {
        let id = next_callback_id();
        TEXT_CHANGED_CALLBACKS.with_borrow_mut(|callbacks| {
            callbacks.insert(id, callback);
        });
        Self { id }
    }

    pub fn id(&self) -> u64 {
        self.id
    }

    pub fn invoke(id: u64, text: String) {
        let callback = TEXT_CHANGED_CALLBACKS.with_borrow(|callbacks| callbacks.get(&id).cloned());
        if let Some(callback) = callback {
            callback(text);
        }
    }
}

impl Drop for RegisteredTextChangedCallback {
    fn drop(&mut self) {
        TEXT_CHANGED_CALLBACKS.with_borrow_mut(|callbacks| {
            callbacks.remove(&self.id);
        });
    }
}

#[derive(Debug)]
pub(crate) struct RegisteredTabSelectionCallback {
    id: u64,
}

impl RegisteredTabSelectionCallback {
    pub fn register(callback: nestix::Shared<dyn Fn(String)>) -> Self {
        let id = next_callback_id();
        TAB_SELECTION_CALLBACKS.with_borrow_mut(|callbacks| {
            callbacks.insert(id, callback);
        });
        Self { id }
    }

    pub fn id(&self) -> u64 {
        self.id
    }

    pub fn invoke(id: u64, selected_id: String) {
        let callback = TAB_SELECTION_CALLBACKS.with_borrow(|callbacks| callbacks.get(&id).cloned());
        if let Some(callback) = callback {
            callback(selected_id);
        }
    }
}

impl Drop for RegisteredTabSelectionCallback {
    fn drop(&mut self) {
        TAB_SELECTION_CALLBACKS.with_borrow_mut(|callbacks| {
            callbacks.remove(&self.id);
        });
    }
}

#[derive(Debug)]
pub(crate) struct RegisteredClickCallback {
    id: u64,
}

impl RegisteredClickCallback {
    pub fn register(callback: nestix::Shared<dyn Fn()>) -> Self {
        let id = next_callback_id();
        CLICK_CALLBACKS.with_borrow_mut(|callbacks| {
            callbacks.insert(id, callback);
        });
        Self { id }
    }

    pub fn id(&self) -> u64 {
        self.id
    }

    pub fn invoke(id: u64) {
        let callback = CLICK_CALLBACKS.with_borrow(|callbacks| callbacks.get(&id).cloned());
        if let Some(callback) = callback {
            callback();
        }
    }
}

impl Drop for RegisteredClickCallback {
    fn drop(&mut self) {
        CLICK_CALLBACKS.with_borrow_mut(|callbacks| {
            callbacks.remove(&self.id);
        });
    }
}

#[derive(Debug)]
pub(crate) struct RegisteredScaleFactorCallback {
    id: u64,
}

impl RegisteredScaleFactorCallback {
    pub fn register(callback: nestix::Shared<dyn Fn(f64)>) -> Self {
        let id = next_callback_id();
        SCALE_FACTOR_CALLBACKS.with_borrow_mut(|callbacks| {
            callbacks.insert(id, callback);
        });
        Self { id }
    }

    pub fn id(&self) -> u64 {
        self.id
    }

    pub fn invoke(id: u64, scale_factor: f64) {
        let callback = SCALE_FACTOR_CALLBACKS.with_borrow(|callbacks| callbacks.get(&id).cloned());
        if let Some(callback) = callback {
            callback(scale_factor);
        }
    }
}

impl Drop for RegisteredScaleFactorCallback {
    fn drop(&mut self) {
        SCALE_FACTOR_CALLBACKS.with_borrow_mut(|callbacks| {
            callbacks.remove(&self.id);
        });
    }
}

#[derive(Debug)]
pub(crate) struct RegisteredResizeCallback {
    id: u64,
}

impl RegisteredResizeCallback {
    pub fn register(callback: nestix::Shared<dyn Fn(Size)>) -> Self {
        let id = next_callback_id();
        RESIZE_CALLBACKS.with_borrow_mut(|callbacks| {
            callbacks.insert(id, callback);
        });
        Self { id }
    }

    pub fn id(&self) -> u64 {
        self.id
    }

    pub fn invoke(id: u64, size: Size) {
        let callback = RESIZE_CALLBACKS.with_borrow(|callbacks| callbacks.get(&id).cloned());
        if let Some(callback) = callback {
            callback(size);
        }
    }
}

impl Drop for RegisteredResizeCallback {
    fn drop(&mut self) {
        RESIZE_CALLBACKS.with_borrow_mut(|callbacks| {
            callbacks.remove(&self.id);
        });
    }
}

fn next_callback_id() -> u64 {
    NEXT_CALLBACK_ID.with(|next_id| {
        let id = next_id.get();
        next_id.set(id.wrapping_add(1).max(1));
        id
    })
}
