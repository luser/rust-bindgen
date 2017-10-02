/* automatically generated by rust-bindgen */


#![allow(dead_code, non_snake_case, non_camel_case_types, non_upper_case_globals)]


#[repr(C)]
#[derive(Debug, Copy, Hash, PartialEq, Eq)]
pub struct Bar {
    pub b: *mut a,
}
#[test]
fn bindgen_test_layout_Bar() {
    assert_eq!(
        ::std::mem::size_of::<Bar>(),
        8usize,
        concat!("Size of: ", stringify!(Bar))
    );
    assert_eq!(
        ::std::mem::align_of::<Bar>(),
        8usize,
        concat!("Alignment of ", stringify!(Bar))
    );
    assert_eq!(
        unsafe { &(*(0 as *const Bar)).b as *const _ as usize },
        0usize,
        concat!("Alignment of field: ", stringify!(Bar), "::", stringify!(b))
    );
}
impl Clone for Bar {
    fn clone(&self) -> Self {
        *self
    }
}
impl Default for Bar {
    fn default() -> Self {
        unsafe { ::std::mem::zeroed() }
    }
}
#[repr(C)]
#[derive(Copy)]
pub struct c {
    pub __bindgen_anon_1: c__bindgen_ty_1,
}
#[repr(C)]
#[derive(Copy)]
pub union c__bindgen_ty_1 {
    _bindgen_union_align: u8,
    pub _address: u8,
}
#[test]
fn bindgen_test_layout_c__bindgen_ty_1() {
    assert_eq!(
        ::std::mem::size_of::<c__bindgen_ty_1>(),
        1usize,
        concat!("Size of: ", stringify!(c__bindgen_ty_1))
    );
    assert_eq!(
        ::std::mem::align_of::<c__bindgen_ty_1>(),
        1usize,
        concat!("Alignment of ", stringify!(c__bindgen_ty_1))
    );
}
impl Clone for c__bindgen_ty_1 {
    fn clone(&self) -> Self {
        *self
    }
}
impl Default for c__bindgen_ty_1 {
    fn default() -> Self {
        unsafe { ::std::mem::zeroed() }
    }
}
#[test]
fn bindgen_test_layout_c() {
    assert_eq!(
        ::std::mem::size_of::<c>(),
        1usize,
        concat!("Size of: ", stringify!(c))
    );
    assert_eq!(
        ::std::mem::align_of::<c>(),
        1usize,
        concat!("Alignment of ", stringify!(c))
    );
}
impl Clone for c {
    fn clone(&self) -> Self {
        *self
    }
}
impl Default for c {
    fn default() -> Self {
        unsafe { ::std::mem::zeroed() }
    }
}
#[repr(C)]
#[derive(Copy)]
pub struct a {
    pub d: c,
}
#[test]
fn bindgen_test_layout_a() {
    assert_eq!(
        ::std::mem::size_of::<a>(),
        1usize,
        concat!("Size of: ", stringify!(a))
    );
    assert_eq!(
        ::std::mem::align_of::<a>(),
        1usize,
        concat!("Alignment of ", stringify!(a))
    );
    assert_eq!(
        unsafe { &(*(0 as *const a)).d as *const _ as usize },
        0usize,
        concat!("Alignment of field: ", stringify!(a), "::", stringify!(d))
    );
}
impl Clone for a {
    fn clone(&self) -> Self {
        *self
    }
}
impl Default for a {
    fn default() -> Self {
        unsafe { ::std::mem::zeroed() }
    }
}
