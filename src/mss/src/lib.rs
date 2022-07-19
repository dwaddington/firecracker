use std::ffi::CString;
use std::fmt;
use std::slice::from_raw_parts_mut;


#[allow(soft_unstable)]
#[allow(non_upper_case_globals)]
#[allow(non_camel_case_types)]
#[allow(non_snake_case)]
#[allow(deref_nullptr)]
#[allow(dead_code)]
mod mss_api;

mod unit_test;

#[derive(Debug)]
pub struct Error {
    code: i32,
    msg: String,
}

impl Error {
    fn new(code: i32, msg: &str) -> Error {
        Error {
            code,
            msg: msg.to_string(),
        }
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "Error: {} {}", self.code, self.msg)
    }
}

/// corresponding to mss_init - initializes DPDK runtime
pub fn init(lcores: &str) -> Result<(), Error> {
    println!("mss-client-rust::init ({})", &lcores);

    let c_lcores = CString::new(lcores).unwrap();

    match unsafe { mss_api::mss_init(c_lcores.as_ptr() as *const ::std::os::raw::c_char) } {
        0 => Ok(()),
        e => Err(Error::new(e, "mss_init failed unexpectedly")),
    }
}

pub fn shutdown() {
    unsafe { mss_api::mss_shutdown() }
}

/// holder for DPDK allocated memory which is freed when this object is dropped
#[derive(Debug)]
pub struct DpdkMemory<'a> {
    ptr: *mut ::std::os::raw::c_void,
    len: usize,
    slice: &'a mut [u8],
}

impl<'a> DpdkMemory<'a> {
    pub fn new(ptr: *mut ::std::os::raw::c_void, len: usize) -> DpdkMemory<'a> {
        DpdkMemory {
            len,
            ptr,
            slice: unsafe { from_raw_parts_mut(ptr as *mut u8, len) },
        }
    }

    pub fn as_slice(&mut self) -> &mut [u8] {
        self.slice
    }
}

impl<'a> Drop for DpdkMemory<'a> {
    fn drop(&mut self) {
        println!("freeing DPDK memory: {:?}", self.ptr);
        unsafe { mss_api::mss_rte_free(self.ptr) }
    }
}


impl fmt::Display for DpdkMemory<'_> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "DPDK-MEMORY: {:?} {}", self.ptr, self.len)
    }
}

/// corresponding to mss_rte_malloc - allocate DPDK memory
pub fn rte_malloc(tag: &str, len: usize, align: usize) -> Result<DpdkMemory, Error> {
    let c_tag = CString::new(tag).unwrap();

    let rptr = unsafe { mss_api::mss_rte_malloc(c_tag.as_ptr(), len as u64, align as u32) };
    if rptr.is_null() {
        return Err(Error::new(-1, "mss_rte_malloc failed unexpectedly"));
    }
    Ok(DpdkMemory::new(rptr, len))
}
