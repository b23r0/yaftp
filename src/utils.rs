use async_std::fs::File;
use futures::AsyncReadExt;
use md5::{Digest, Md5};

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

pub async fn calc_md5(f :&mut File , end_pos : u64) -> String {

	let mut md5_str = String::new();

	let mut md5 = Md5::default();

	let mut buffer = vec![0u8 ; 1024 * 1024 * 20].into_boxed_slice();
	
	let mut sum : u64 = 0;
	loop{

		if (end_pos - sum) <= 1024 * 1024 * 20 {

			let mut last_buf = vec![0u8; (end_pos - sum) as usize].into_boxed_slice();
			match f.read_exact(&mut last_buf).await {
				Ok(n) => n,
				Err(_) => {
					return md5_str;
				},
			};

			md5.update(&last_buf);

			break;
		}

		let n = match f.read(&mut buffer).await {
			Ok(n) => n,
			Err(_) => {
				return md5_str;
			},
		};
		sum += n as u64;
		md5.update(&buffer[..n]);

		if n == 0 {
			break;
		}
	}

	for b in md5.finalize(){
		let a = format!("{:02x}", b);
		md5_str += &a;
	}

	md5_str
}

#[macro_export]
macro_rules! println_err {
    () => ($crate::print!("\n"));
    ($($arg:tt)*) => ({
		print!("[-] ");
        println!($($arg)*);
    })
}

#[macro_export]
macro_rules! println_info {
    () => ($crate::print!("\n"));
    ($($arg:tt)*) => ({
		print!("[+] ");
        println!($($arg)*);
    })
}