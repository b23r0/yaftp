include!{"utils.rs"}

use crate::common::{YaftpError, error_retcode};
use std::{fs, net::Shutdown};

use futures::{AsyncReadExt, AsyncWriteExt};
use async_std::{net::{TcpStream}};
use chrono::DateTime;
use chrono::offset::Utc;

async fn send_reply(stream :&mut  TcpStream , retcode : u8 , narg : u32) -> Result<Vec<u8>, YaftpError> {
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
			return Err(YaftpError::UnknownNetwordError);
		},
	};

	Ok(args)
}

async fn send_argument(stream :&mut  TcpStream , data :&mut Vec<u8>) -> Result<Vec<u8>, YaftpError>{
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
			return Err(YaftpError::UnknownNetwordError);
		},
	};
	Ok(argument)
}

async fn read_argument(stream :&mut  TcpStream , max_size : u64) -> Result<Vec<u8>, YaftpError> {
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
			return Err(YaftpError::UnknownNetwordError);
		},
	};

	let size = u64::from_be_bytes(argument_size);

	if size > max_size {
		log::error!("argument size error : {}" , size);
		return Err(YaftpError::ArgumentUnvalid);
	}

	let mut arg = vec![0u8;size as usize].into_boxed_slice();

	match stream.read_exact(&mut arg).await{
		Ok(_) => {},
		Err(e) => {
			log::error!("error : {}" , e);
			return Err(YaftpError::UnknownNetwordError);
		},
	};

	Ok(arg.to_vec())
}

async fn c_ls(stream :&mut  TcpStream, narg : u32) -> u8 {

	let mut ret = error_retcode(YaftpError::OK);

	if narg != 1 {
		log::error!("command [{}] arguments count unvalid : {}" , "ls", narg);
		ret = error_retcode(YaftpError::ArgumentUnvalid);

		match send_reply(stream, ret , 0).await {
			Ok(_) => {},
			Err(e) => {
				ret = error_retcode(e);
			},
		};

		return ret;
	}

	loop {
		let path = match read_argument(stream, 1024).await{
			Ok(p) => p,
			Err(_) => {
				ret = error_retcode(YaftpError::UnknownNetwordError);
				break;
			}
		};

		let path = match String::from_utf8(path.to_vec()){
			Ok(p) => p,
			Err(_) => {
				ret = error_retcode(YaftpError::ArgumentUnvalid);
				break;
			},
		};

		let paths = match fs::read_dir(path){
			Ok(p) => p,
			Err(_) => {
				ret = error_retcode(YaftpError::ReadFolderFaild);
				break;
			},
		};

		let mut rows1 : Vec<String> = Vec::new();
		let mut rows2 : Vec<String> = Vec::new();
		let mut rows3 : Vec<String> = Vec::new();
		let mut rows4 : Vec<String> = Vec::new();

		for path in paths {
			let path = path.unwrap();
			let filename = String::from(path.file_name().to_str().unwrap());
			let mt = path.metadata().unwrap().modified().unwrap();

			let mt: DateTime<Utc> = mt.into();

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

			rows4.push(mt.format("%Y-%m-%d %H:%M:%S").to_string());
		}

		if ret == error_retcode(YaftpError::OK) {

			match send_reply(stream, 0 , rows1.len() as u32).await {
				Ok(_) => {},
				Err(_) => {
					break;
				},
			};

			let mut i = 0 ;

			while i < rows1.len(){
				let full = [rows1[i].clone() , rows2[i].clone(), rows3[i].clone() , rows4[i].clone()].join("|");
				match send_argument(stream, &mut full.as_bytes().to_vec()).await {
					Ok(_) => {},
					Err(e) => {
						log::error!("yaftp send argument error");
						ret = error_retcode(e);
						break;
					},
				};
				i += 1;
			}
		}

		break;
	}

	if ret != error_retcode(YaftpError::OK){

		match send_reply(stream, ret , 0).await {
			Ok(_) => {},
			Err(_) => {
			},
		};
	}

	ret

}

async fn c_cwd(stream :&mut  TcpStream, narg : u32) -> u8 {

	let mut ret = error_retcode(YaftpError::OK);

	if narg != 0 {
		log::error!("command [{}] arguments count unvalid : {}" , "ls", narg);
		ret = error_retcode(YaftpError::ArgumentUnvalid);
		match send_reply(stream, ret , 0).await {
			Ok(_) => {},
			Err(e) => {
				ret = error_retcode(e);
			},
		};
		return ret;
	}

	loop {
		let paths = match  std::env::current_dir(){
			Ok(p) => p,
			Err(_) => {
				ret = error_retcode(YaftpError::ReadCwdFaild);
				break;
			},
		};

		let path = paths.to_str().unwrap();

		if ret == error_retcode(YaftpError::OK) {

			match send_reply(stream, 0 , 1).await {
				Ok(_) => {},
				Err(e) => {
					ret = error_retcode(e);
					break;
				},
			};

			match send_argument(stream, &mut path.as_bytes().to_vec()).await {
				Ok(_) => {},
				Err(e) => {
					ret = error_retcode(e);
					break;
				},
			};
		}

		break;
	}

	if ret != error_retcode(YaftpError::OK){
		/*
		+-----------+-----------+
		|  RETCODE  |  NARG	    |
		+-----------+-----------+
		|  1(u8)    |  4(u32)   |
		+-----------+-----------+
		*/

		match send_reply(stream, ret , 0).await {
			Ok(_) => {},
			Err(e) => {
				ret = error_retcode(e);
			},
		};
	}

	ret

}

async fn c_info(stream :&mut  TcpStream, narg : u32) {

	let mut ret = 0u8;

	if narg != 1 {
		log::error!("command [{}] arguments count unvalid : {}" , "ls", narg);
		ret = error_retcode(YaftpError::ArgumentUnvalid);
		match send_reply(stream, ret , 0).await {
			Ok(_) => {},
			Err(_) => {},
		};
		return;
	}

	loop {
		let path = match read_argument(stream, 1024).await{
			Ok(p) => p,
			Err(_) => {
				ret = error_retcode(YaftpError::ArgumentUnvalid);
				break;
			}
		};

		let path = match String::from_utf8(path.to_vec()){
			Ok(p) => p,
			Err(_) => {
				ret = error_retcode(YaftpError::ArgumentUnvalid);
				break;
			},
		};

		let info = match fs::metadata(path){
			Ok(p) => p,
			Err(e) => {
				if e.kind() == std::io::ErrorKind::PermissionDenied {
					ret = error_retcode(YaftpError::NoPermission);
				} else if e.kind() == std::io::ErrorKind::NotFound {
					ret = error_retcode(YaftpError::NotFound);
				}
				break;
			},
		};

		let mut t : u8 = 0xff;

		if info.is_dir() {
			t = 0;
		} else if info.is_file() {
			t = 1;
		} else {
			t = 0xff;
		}

		let size : u64 = info.len();

		let mt =  info.modified().unwrap();
		let mt: DateTime<Utc> = mt.into();
		let mt : u64 = mt.timestamp().try_into().unwrap();

		let at =  info.accessed().unwrap();
		let at: DateTime<Utc> = at.into();
		let at : u64 = at.timestamp().try_into().unwrap();

		if ret == error_retcode(YaftpError::OK) {

			match send_reply(stream, 0 , 4).await {
				Ok(_) => {},
				Err(e) => {
					ret = error_retcode(e);
					break;
				},
			};

			match send_argument(stream, &mut [t].to_vec()).await {
				Ok(_) => {},
				Err(e) => {
					log::error!("yaftp send argument error");
					ret = error_retcode(e);
					break;
				},
			};

			let mut size = size.to_be_bytes().to_vec();

			match send_argument(stream, &mut size).await {
				Ok(_) => {},
				Err(e) => {
					log::error!("yaftp send argument error");
					ret = error_retcode(e);
					break;
				},
			};

			let mut mt = mt.to_be_bytes().to_vec();

			match send_argument(stream, &mut mt).await {
				Ok(_) => {},
				Err(e) => {
					log::error!("yaftp send argument error");
					ret = error_retcode(e);
					break;
				},
			};

			let mut at = at.to_be_bytes().to_vec();

			match send_argument(stream, &mut at).await {
				Ok(_) => {},
				Err(e) => {
					log::error!("yaftp send argument error");
					ret = error_retcode(e);
					break;
				},
			};
		}

		break;
	}

	if ret != error_retcode(YaftpError::OK){

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
			0x01 => {
				let _ = c_ls(&mut stream , narg ).await;
			},
			0x02 => {
				let _ = c_cwd(&mut stream , narg ).await;
			},
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