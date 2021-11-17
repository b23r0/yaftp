include!{"utils.rs"}

use std::{fs, io::Error, net::Shutdown};

use futures::{AsyncReadExt, AsyncWriteExt, FutureExt, StreamExt};
use async_std::{net::{TcpStream}};

fn check_support_methods(methods : &[u8]) -> bool {
	
	let mut i = 0 ;
	while i < methods.len() {
		if methods[i] > 0x08 {
			return false;
		}
		i += 1;
	}
	
	true
}

async fn send_reply(stream :&mut  TcpStream , retcode : u8 , narg : u32) -> Result<Vec<u8>, Error> {
	/*
	+-----------+-----------+
	|  RETCODE  |  NARG	    |
	+-----------+-----------+
	|  1(u8)    |  4(u32)   |
	+-----------+-----------+
	*/
	let b_narg = narg.to_be_bytes();
	let mut args : Vec<u8> = Vec::new();

	args.push(retcode);
	args.append(&mut b_narg.to_vec());
	match stream.write_all( &mut args).await{
		Ok(_) => {},
		Err(e) => {
			log::error!("error : {}" , e);
			return Err(e);
		},
	};

	Ok(args)
}

async fn send_argument(stream :&mut  TcpStream , data :&mut Vec<u8>) -> Result<Vec<u8>, Error>{
	/*
	+-----------------+---------------------+
	| NEXT_ARG_SIZE   |	      ARG           |
	+-----------------+---------------------+
	|     8(u64)      |	   Variable	        |
	+-----------------+---------------------+
	*/
	let size = data.len() as u64;
	let b_size = size.to_be_bytes();
	let mut argument = vec![];
	argument.append(&mut b_size.to_vec());
	argument.append(data);

	match stream.write_all(&mut argument).await{
		Ok(_) => {},
		Err(e) => {
			log::error!("error : {}" , e);
			return Err(e);
		},
	};
	Ok(argument)
}

async fn read_argument(stream :&mut  TcpStream , max_size : u64) -> Result<Vec<u8>, Error> {
	/*
	+-----------------+---------------------+
	| NEXT_ARG_SIZE   |      ARG            |
	+-----------------+---------------------+
	|     8(u64)      |	     Variable       |
	+-----------------+---------------------+
	*/
	let mut argument_size = [0u8;8];
	match stream.read_exact(&mut argument_size).await{
		Ok(_) => {},
		Err(e) => {
			log::error!("error : {}" , e);
			return Err(e);
		},
	};

	let size = u64::from_be_bytes(argument_size);

	if size > max_size {
		log::error!("argument size error : {}" , size);
		return Err(Error::new(std::io::ErrorKind::InvalidData, "argument size error"));
	}

	let mut arg = vec![0u8;size as usize].into_boxed_slice();

	match stream.read_exact(&mut arg).await{
		Ok(_) => {},
		Err(e) => {
			log::error!("error : {}" , e);
			return Err(e);
		},
	};

	Ok(arg.to_vec())
}

async fn c_ls(stream :&mut  TcpStream, narg : u32) {

	let mut ret = 0u8;

	if narg != 1 {
		log::error!("command [{}] arguments count unvalid : {}" , "ls", narg);
		ret = 8;
		return;
	}

	loop {
		let path = match read_argument(stream, 1024).await{
			Ok(p) => p,
			Err(e) => {
				ret = 8;
				return;
			}
		};

		let path = match String::from_utf8(path.to_vec()){
			Ok(p) => p,
			Err(e) => {
				ret = 8;
				break;
			},
		};

		let paths = match fs::read_dir(path){
			Ok(p) => p,
			Err(_) => {
				ret = 9;
				break;
			},
		};

		let mut rows1 : Vec<String> = Vec::new();
		let mut rows2 : Vec<String> = Vec::new();
		let mut rows3 : Vec<String> = Vec::new();

		for path in paths {
			let path = path.unwrap();
			let filename = String::from(path.file_name().to_str().unwrap());
			rows1.push(filename);

			let t = path.file_type().unwrap();
			if t.is_dir() {
				rows2.push(String::from("folder"));
			} else if t.is_file() {
				rows2.push(String::from("file"));
			} else if t.is_symlink() {
				rows2.push(String::from("symlink"));
			} else {
				rows2.push(String::from("other"));
			}
			rows3.push(path.metadata().unwrap().len().to_string());
		}

		if ret == 0 {

			match send_reply(stream, 0 , rows1.len() as u32).await {
				Ok(_) => {},
				Err(_) => {
					break;
				},
			};

			let mut i = 0 ;

			while i < rows1.len(){
				let full = [rows1[i].clone() , rows2[i].clone(), rows3[i].clone()].join("|");
				match send_argument(stream, &mut full.as_bytes().to_vec()).await {
					Ok(_) => {},
					Err(e) => {
						log::error!("error : {}" , e);
						break;
					},
				};
				i += 1;
			}
		}

		break;
	}

	if ret != 0{
		/*
		+-----------+-----------+
		|  RETCODE  |  NARG	    |
		+-----------+-----------+
		|  1(u8)    |  4(u32)   |
		+-----------+-----------+
		*/

		match send_reply(stream, ret , 0).await {
			Ok(_) => {},
			Err(_) => {
			},
		};
	}

}

async fn c_cwd(stream :&mut  TcpStream, narg : u32) {

	let mut ret = 0u8;

	if narg != 0 {
		log::error!("command [{}] arguments count unvalid : {}" , "ls", narg);
		ret = 8;
		return;
	}

	loop {
		let paths = match  std::env::current_dir(){
			Ok(p) => p,
			Err(_) => {
				ret = 10;
				break;
			},
		};

		let path = paths.to_str().unwrap();

		if ret == 0 {

			match send_reply(stream, 0 , 1).await {
				Ok(_) => {},
				Err(_) => {
					break;
				},
			};

			match send_argument(stream, &mut path.as_bytes().to_vec()).await {
				Ok(_) => {},
				Err(_) => {
					break;
				},
			};
		}

		break;
	}

	if ret != 0{
		/*
		+-----------+-----------+
		|  RETCODE  |  NARG	    |
		+-----------+-----------+
		|  1(u8)    |  4(u32)   |
		+-----------+-----------+
		*/

		match send_reply(stream, ret , 0).await {
			Ok(_) => {},
			Err(_) => {
			},
		};
	}

}

pub async fn yaftp_server_handle(mut stream : TcpStream){

	loop {
		/*
		+-------+----------+---------------+
		|  VER  | NMETHODS | METHODS       |
		+-------+----------+---------------+
		| 1(u8) |   1(u8)  | 1 to 255 (u8) |
		+-------+----------+---------------+
		*/
		let mut header = [0u8;2];
		match stream.read_exact(&mut header).await{
			Ok(_) => {},
			Err(e) => {
				log::error!("error : {}" , e);
				break;
			},
		};

		if header[0] != 0x01{
			log::error!("not support version : {}" , header[0]);
			break;
		}

		let mut methods = vec![0u8;header[1].into()].into_boxed_slice();

		match stream.read_exact(&mut methods).await{
			Ok(_) => {},
			Err(e) => {
				log::error!("error : {}" , e);
				break;
			},
		};

		if !check_support_methods(&methods){
			log::warn!("the client has not support method");
		}

		/*
		+-------+----------+---------------+
		|  VER  | NMETHODS | METHODS       |
		+-------+----------+---------------+
		| 1(u8) |   1(u8)  | 1 to 255 (u8) |
		+-------+----------+---------------+
		*/
		match stream.write_all(&[1u8, 8u8 , 1u8 , 2u8 , 3u8 , 4u8 , 5u8 , 6u8 , 7u8, 8u8]).await{
			Ok(_) => {},
			Err(e) => {
				log::error!("error : {}" , e);
				break;
			},
		};

		/*
		+-------+--------+
		|  CMD  | NARG   |
		+-------+--------+
		| 1(u8) | 4(u32) |
		+-------+--------+
		*/
		let mut command = [0u8;5];
		match stream.read_exact(&mut command).await{
			Ok(_) => {},
			Err(e) => {
				log::error!("error : {}" , e);
				break;
			},
		};

		if !check_support_methods(&[command[0]]){
			log::error!("not support command : {}" , command[0]);
			break;
		}

		let narg = u32::from_be_bytes(command[1..5].try_into().unwrap());

		match command[0] {
			0x01 => c_ls(&mut stream , narg ).await,
			0x02 => c_cwd(&mut stream , narg ).await,
			_ => {
				log::error!("not support command {}" , command[0]);
				break;
			}
		}
		break;
	}

	match stream.shutdown(Shutdown::Both){
		Ok(_) => {},
		Err(_) => {},
	};


}