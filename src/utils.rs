#[allow(dead_code)]
pub fn check_support_methods(methods : &[u8]) -> bool {
	
	let mut i = 0 ;
	while i < methods.len() {
		if methods[i] > 0x0a {
			return false;
		}
		i += 1;
	}
	
	true
}

#[allow(unused_macros)]
macro_rules! println_err {
    () => ($crate::print!("\n"));
    ($($arg:tt)*) => ({
		print!("[-] ");
        println!($($arg)*);
    })
}

#[allow(unused_macros)]
macro_rules! println_info {
    () => ($crate::print!("\n"));
    ($($arg:tt)*) => ({
		print!("[+] ");
        println!($($arg)*);
    })
}