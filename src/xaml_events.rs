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
}

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
        CONTENT_SIZE_CALLBACKS.with_borrow(|callbacks| {
            if let Some(callback) = callbacks.get(&id) {
                callback(width, height);
            }
        });
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
        TAB_SELECTION_CALLBACKS.with_borrow(|callbacks| {
            if let Some(callback) = callbacks.get(&id) {
                callback(selected_id);
            }
        });
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
        CLICK_CALLBACKS.with_borrow(|callbacks| {
            if let Some(callback) = callbacks.get(&id) {
                callback();
            }
        });
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
        SCALE_FACTOR_CALLBACKS.with_borrow(|callbacks| {
            if let Some(callback) = callbacks.get(&id) {
                callback(scale_factor);
            }
        });
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
        RESIZE_CALLBACKS.with_borrow(|callbacks| {
            if let Some(callback) = callbacks.get(&id) {
                callback(size);
            }
        });
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
