// #[macro_use(singleton)]
// extern crate cortex_m;

#[cfg(test)]
mod tests {

    #[test]
    fn test_mss_init() {
        crate::init("0").unwrap();
        crate::shutdown();
    }

//    #[test]
    fn test_alloc() {
        crate::init("0").unwrap();
        {
            let mut mem = match crate::rte_malloc("myDPDKmem", 4096, 4096) {
                Ok(p) => p,
                _ => panic!("failed"),
            };
            println!("memory allocation:{:?} {}", mem.ptr, mem.len);
            let slice = mem.as_slice();
            slice[1] = 99;
            println!("as slice {:?}", &slice[0..10]);
        }
        crate::shutdown();
    }
}
