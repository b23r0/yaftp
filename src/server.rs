include!{"utils.rs"}

use crate::common::{YaftpError, error_retcode};
use std::{fs, io::{Read, SeekFrom}, net::Shutdown, path::{Path}};

use futures::{AsyncReadExt, AsyncWriteExt};
use async_std::{io::{self, prelude::SeekExt}, net::{TcpStream}};
use chrono::DateTime;
use chrono::offset::Utc;
use path_absolutize::*;
use md5::{Digest, Md5};

async fn send_reply(stream :&mut  TcpStream , retcode : u8 , narg : u32) -> Result<Vec<u8>, YaftpError> {
	/*
	+-----------+-----------+
	|  RETCODE  |  NARG     |
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
	| NEXT_ARG_SIZE   |       ARG           |
	+-----------------+---------------------+
	|     8(u64)      |    Variable         |
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
		let mut rows5 : Vec<String> = Vec::new();

		for path in paths {
			let path = path.unwrap();
			let filename = String::from(path.file_name().to_str().unwrap());

			let mt = path.metadata().unwrap().modified().unwrap();
			let mt: DateTime<Utc> = mt.into();

			let at = path.metadata().unwrap().accessed().unwrap();
			let at: DateTime<Utc> = at.into();

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
			rows5.push(at.format("%Y-%m-%d %H:%M:%S").to_string());
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
				let full = [rows1[i].clone() , rows2[i].clone(), rows3[i].clone() , rows4[i].clone() , rows5[i].clone()].join("|");
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
		log::error!("command [{}] arguments count unvalid : {}" , "cwd", narg);
		ret = error_retcode(YaftpError::ArgumentCountError);
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
		ret = error_retcode(YaftpError::ArgumentCountError);
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

		let path = Path::new(path.as_str());
		let path =  match path.absolutize(){
			Ok(p) => p,
			Err(e) => {
				if e.kind() == std::io::ErrorKind::PermissionDenied {
					ret = error_retcode(YaftpError::NoPermission);
				} else if e.kind() == std::io::ErrorKind::NotFound {
					ret = error_retcode(YaftpError::NotFound);
				} else {
					print!("error : {}" , e);
					ret = error_retcode(YaftpError::UnknownError);
				}
				break;
			},
		};

		let path = path.to_str().unwrap().to_string();

		let info = match fs::metadata(path.clone()){
			Ok(p) => p,
			Err(e) => {
				if e.kind() == std::io::ErrorKind::PermissionDenied {
					ret = error_retcode(YaftpError::NoPermission);
				} else if e.kind() == std::io::ErrorKind::NotFound {
					ret = error_retcode(YaftpError::NotFound);
				} else {
					print!("error : {}" , e);
					ret = error_retcode(YaftpError::UnknownError);
				}
				break;
			},
		};

		let t : u8;

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

			match send_reply(stream, 0 , 5).await {
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

			match send_argument(stream, &mut path.as_bytes().to_vec()).await {
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

async fn c_cp(stream :&mut  TcpStream, narg : u32){

	let mut ret = 0u8;

	if narg != 2 {
		log::error!("command [{}] arguments count unvalid : {}" , "cp", narg);
		ret = error_retcode(YaftpError::ArgumentCountError);
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

		let srcpath = match String::from_utf8(path.to_vec()){
			Ok(p) => p,
			Err(_) => {
				ret = error_retcode(YaftpError::ArgumentUnvalid);
				break;
			},
		};


		let srcpath = Path::new(srcpath.as_str());
		let srcpath =  match srcpath.absolutize(){
			Ok(p) => p,
			Err(e) => {
				if e.kind() == std::io::ErrorKind::PermissionDenied {
					ret = error_retcode(YaftpError::NoPermission);
				} else if e.kind() == std::io::ErrorKind::NotFound {
					ret = error_retcode(YaftpError::NotFound);
				} else {
					print!("error : {}" , e);
					ret = error_retcode(YaftpError::UnknownError);
				}
				break;
			},
		};

		let path = match read_argument(stream, 1024).await{
			Ok(p) => p,
			Err(_) => {
				ret = error_retcode(YaftpError::ArgumentUnvalid);
				break;
			}
		};

		let targetpath = match String::from_utf8(path.to_vec()){
			Ok(p) => p,
			Err(_) => {
				ret = error_retcode(YaftpError::ArgumentUnvalid);
				break;
			},
		};

		let targetpath = Path::new(targetpath.as_str());
		let targetpath =  match targetpath.absolutize(){
			Ok(p) => p,
			Err(e) => {
				if e.kind() == std::io::ErrorKind::PermissionDenied {
					ret = error_retcode(YaftpError::NoPermission);
				} else if e.kind() == std::io::ErrorKind::NotFound {
					ret = error_retcode(YaftpError::NotFound);
				} else {
					print!("error : {}" , e);
					ret = error_retcode(YaftpError::UnknownError);
				}
				break;
			},
		};

		match fs::copy(srcpath, targetpath){
			Ok(p) => p,
			Err(e) => {
				if e.kind() == std::io::ErrorKind::PermissionDenied {
					ret = error_retcode(YaftpError::NoPermission);
				} else if e.kind() == std::io::ErrorKind::NotFound {
					ret = error_retcode(YaftpError::NotFound);
				} else {
					print!("error : {}" , e);
					ret = error_retcode(YaftpError::UnknownError);
				}
				break;
			},
		};

		if ret == error_retcode(YaftpError::OK){
			match send_reply(stream, 0 , 0).await {
				Ok(_) => {},
				Err(_) => {
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

async fn c_mkd(stream :&mut  TcpStream, narg : u32){

	let mut ret = 0u8;

	if narg != 1 {
		log::error!("command [{}] arguments count unvalid : {}" , "mkd", narg);
		ret = error_retcode(YaftpError::ArgumentCountError);
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


		let path = Path::new(path.as_str());
		let path =  match path.absolutize(){
			Ok(p) => p,
			Err(e) => {
				if e.kind() == std::io::ErrorKind::PermissionDenied {
					ret = error_retcode(YaftpError::NoPermission);
				} else if e.kind() == std::io::ErrorKind::NotFound {
					ret = error_retcode(YaftpError::NotFound);
				} else {
					print!("error : {}" , e);
					ret = error_retcode(YaftpError::UnknownError);
				}
				break;
			},
		};


		match fs::create_dir_all(path){
			Ok(p) => p,
			Err(e) => {
				if e.kind() == std::io::ErrorKind::PermissionDenied {
					ret = error_retcode(YaftpError::NoPermission);
				} else if e.kind() == std::io::ErrorKind::AlreadyExists {
					ret = error_retcode(YaftpError::NotFound);
				} else {
					print!("error : {}" , e);
					ret = error_retcode(YaftpError::UnknownError);
				}
				break;
			},
		};

		if ret == error_retcode(YaftpError::OK){
			match send_reply(stream, 0 , 0).await {
				Ok(_) => {},
				Err(_) => {
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

async fn c_mv(stream :&mut  TcpStream, narg : u32){

	let mut ret = 0u8;

	if narg != 2 {
		log::error!("command [{}] arguments count unvalid : {}" , "mv", narg);
		ret = error_retcode(YaftpError::ArgumentCountError);
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

		let srcpath = match String::from_utf8(path.to_vec()){
			Ok(p) => p,
			Err(_) => {
				ret = error_retcode(YaftpError::ArgumentUnvalid);
				break;
			},
		};


		let srcpath = Path::new(srcpath.as_str());
		let srcpath =  match srcpath.absolutize(){
			Ok(p) => p,
			Err(e) => {
				if e.kind() == std::io::ErrorKind::PermissionDenied {
					ret = error_retcode(YaftpError::NoPermission);
				} else if e.kind() == std::io::ErrorKind::NotFound {
					ret = error_retcode(YaftpError::NotFound);
				} else {
					print!("error : {}" , e);
					ret = error_retcode(YaftpError::UnknownError);
				}
				break;
			},
		};

		let path = match read_argument(stream, 1024).await{
			Ok(p) => p,
			Err(_) => {
				ret = error_retcode(YaftpError::ArgumentUnvalid);
				break;
			}
		};

		let targetpath = match String::from_utf8(path.to_vec()){
			Ok(p) => p,
			Err(_) => {
				ret = error_retcode(YaftpError::ArgumentUnvalid);
				break;
			},
		};

		let targetpath = Path::new(targetpath.as_str());
		let targetpath =  match targetpath.absolutize(){
			Ok(p) => p,
			Err(e) => {
				if e.kind() == std::io::ErrorKind::PermissionDenied {
					ret = error_retcode(YaftpError::NoPermission);
				} else if e.kind() == std::io::ErrorKind::NotFound {
					ret = error_retcode(YaftpError::NotFound);
				} else {
					print!("error : {}" , e);
					ret = error_retcode(YaftpError::UnknownError);
				}
				break;
			},
		};

		match fs::copy(srcpath.clone(), targetpath){
			Ok(p) => p,
			Err(e) => {
				if e.kind() == std::io::ErrorKind::PermissionDenied {
					ret = error_retcode(YaftpError::NoPermission);
				} else if e.kind() == std::io::ErrorKind::NotFound {
					ret = error_retcode(YaftpError::NotFound);
				} else {
					print!("error : {}" , e);
					ret = error_retcode(YaftpError::UnknownError);
				}
				break;
			},
		};

		match fs::remove_file(srcpath){
			Ok(p) => p,
			Err(e) => {
				if e.kind() == std::io::ErrorKind::PermissionDenied {
					ret = error_retcode(YaftpError::NoPermission);
				} else if e.kind() == std::io::ErrorKind::NotFound {
					ret = error_retcode(YaftpError::NotFound);
				} else {
					print!("error : {}" , e);
					ret = error_retcode(YaftpError::UnknownError);
				}
				break;
			},
		};

		if ret == error_retcode(YaftpError::OK){
			match send_reply(stream, 0 , 0).await {
				Ok(_) => {},
				Err(_) => {
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

async fn c_rm(stream :&mut  TcpStream, narg : u32){

	let mut ret = 0u8;

	if narg != 1 {
		log::error!("command [{}] arguments count unvalid : {}" , "rm", narg);
		ret = error_retcode(YaftpError::ArgumentCountError);
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


		let path = Path::new(path.as_str());
		let path =  match path.absolutize(){
			Ok(p) => p,
			Err(e) => {
				if e.kind() == std::io::ErrorKind::PermissionDenied {
					ret = error_retcode(YaftpError::NoPermission);
				} else if e.kind() == std::io::ErrorKind::NotFound {
					ret = error_retcode(YaftpError::NotFound);
				} else {
					print!("error : {}" , e);
					ret = error_retcode(YaftpError::UnknownError);
				}
				break;
			},
		};

		let info = match fs::metadata(path.clone()){
			Ok(p) => p,
			Err(e) => {
				if e.kind() == std::io::ErrorKind::PermissionDenied {
					ret = error_retcode(YaftpError::NoPermission);
				} else if e.kind() == std::io::ErrorKind::NotFound {
					ret = error_retcode(YaftpError::NotFound);
				} else {
					print!("error : {}" , e);
					ret = error_retcode(YaftpError::UnknownError);
				}
				break;
			},
		};

		if info.is_dir(){
			match fs::remove_dir_all(path){
				Ok(p) => p,
				Err(e) => {
					if e.kind() == std::io::ErrorKind::PermissionDenied {
						ret = error_retcode(YaftpError::NoPermission);
					} else if e.kind() == std::io::ErrorKind::NotFound {
						ret = error_retcode(YaftpError::NotFound);
					} else {
						print!("error : {}" , e);
						ret = error_retcode(YaftpError::UnknownError);
					}
					break;
				},
			};
		} else if info.is_file(){
			match fs::remove_file(path){
				Ok(p) => p,
				Err(e) => {
					if e.kind() == std::io::ErrorKind::PermissionDenied {
						ret = error_retcode(YaftpError::NoPermission);
					} else if e.kind() == std::io::ErrorKind::NotFound {
						ret = error_retcode(YaftpError::NotFound);
					} else {
						print!("error : {}" , e);
						ret = error_retcode(YaftpError::UnknownError);
					}
					break;
				},
			};
		} else{
			ret = error_retcode(YaftpError::UnknownError);
			break;
		}



		if ret == error_retcode(YaftpError::OK){
			match send_reply(stream, 0 , 0).await {
				Ok(_) => {},
				Err(_) => {
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


async fn c_get(stream :&mut  TcpStream, narg : u32){

	let mut ret = 0u8;

	if narg != 2 {
		log::error!("command [{}] arguments count unvalid : {}" , "get", narg);
		ret = error_retcode(YaftpError::ArgumentCountError);
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

		let start_pos = match read_argument(stream, 8).await{
			Ok(p) => p,
			Err(_) => {
				ret = error_retcode(YaftpError::ArgumentUnvalid);
				break;
			}
		};

		let start_pos = u64::from_be_bytes(start_pos.try_into().unwrap());

		let path = match String::from_utf8(path.to_vec()){
			Ok(p) => p,
			Err(_) => {
				ret = error_retcode(YaftpError::ArgumentUnvalid);
				break;
			},
		};


		let path = Path::new(path.as_str());
		let path =  match path.absolutize(){
			Ok(p) => p,
			Err(e) => {
				if e.kind() == std::io::ErrorKind::PermissionDenied {
					ret = error_retcode(YaftpError::NoPermission);
				} else if e.kind() == std::io::ErrorKind::NotFound {
					ret = error_retcode(YaftpError::NotFound);
				} else {
					print!("error : {}" , e);
					ret = error_retcode(YaftpError::UnknownError);
				}
				break;
			}
		};

		let path = path.to_str().unwrap().to_string();

		let mut f = match async_std::fs::File::open(path).await{
			Ok(p) => p,
			Err(e) => {
				if e.kind() == std::io::ErrorKind::PermissionDenied {
					ret = error_retcode(YaftpError::NoPermission);
				} else if e.kind() == std::io::ErrorKind::NotFound {
					ret = error_retcode(YaftpError::NotFound);
				} else {
					print!("error : {}" , e);
					ret = error_retcode(YaftpError::UnknownError);
				}
				break;
			}
		};

		f.seek(SeekFrom::Start(start_pos));

		match send_reply(stream, 0 , 1).await {
			Ok(_) => {},
			Err(_) => {
			},
		};

		/*
		+-----------------+---------------------+
		| NEXT_ARG_SIZE   |       ARG           |
		+-----------------+---------------------+
		|     8(u64)      |    Variable         |
		+-----------------+---------------------+
		*/

		let size = f.metadata().await.unwrap().len();

		match stream.write_all(&size.to_be_bytes().to_vec()).await{
			Ok(_) => {},
			Err(_) => {
				ret = error_retcode(YaftpError::UnknownNetwordError);
				break;
			}
		};

		match io::copy(&mut f , stream).await{
			Ok(_) => {},
			Err(e) => {
				if e.kind() == std::io::ErrorKind::PermissionDenied {
					ret = error_retcode(YaftpError::NoPermission);
				} else if e.kind() == std::io::ErrorKind::NotFound {
					ret = error_retcode(YaftpError::NotFound);
				} else {
					print!("error : {}" , e);
					ret = error_retcode(YaftpError::UnknownError);
				}
				break;
			}
		};

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

async fn c_hash(stream :&mut  TcpStream, narg : u32){

	let mut ret = 0u8;

	if narg != 2 {
		log::error!("command [{}] arguments count unvalid : {}" , "hash", narg);
		ret = error_retcode(YaftpError::ArgumentCountError);
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

		let end_pos = match read_argument(stream, 8).await{
			Ok(p) => p,
			Err(_) => {
				ret = error_retcode(YaftpError::ArgumentUnvalid);
				break;
			}
		};

		let end_pos = u64::from_be_bytes(end_pos.try_into().unwrap());

		let path = match String::from_utf8(path.to_vec()){
			Ok(p) => p,
			Err(_) => {
				ret = error_retcode(YaftpError::ArgumentUnvalid);
				break;
			},
		};

		let path = Path::new(path.as_str());
		let path =  match path.absolutize(){
			Ok(p) => p,
			Err(e) => {
				if e.kind() == std::io::ErrorKind::PermissionDenied {
					ret = error_retcode(YaftpError::NoPermission);
				} else if e.kind() == std::io::ErrorKind::NotFound {
					ret = error_retcode(YaftpError::NotFound);
				} else {
					print!("error : {}" , e);
					ret = error_retcode(YaftpError::UnknownError);
				}
				break;
			},
		};

		let mut f = match fs::File::open(&path){
			Ok(p) => p,
			Err(e) => {
				if e.kind() == std::io::ErrorKind::PermissionDenied {
					ret = error_retcode(YaftpError::NoPermission);
				} else if e.kind() == std::io::ErrorKind::NotFound {
					ret = error_retcode(YaftpError::NotFound);
				} else {
					print!("error : {}" , e);
					ret = error_retcode(YaftpError::UnknownError);
				}
				break;
			},
		};

		let mut md5 = Md5::default();

		let mut buffer = vec![0u8 ; 1024 * 1024 * 20].into_boxed_slice();
		
		let mut sum : u64 = 0;
		loop{

			let n = match f.read(&mut buffer) {
				Ok(n) => n,
				Err(_) => {
					ret = error_retcode(YaftpError::UnknownError);
					break;
				},
			};
			sum += n as u64;
			md5.update(&buffer[..n]);
			if n == 0 {
				break;
			}
			if sum == end_pos {
				break;
			}
		}

		let mut md5_str = String::new();

		for b in md5.finalize(){
			let a = format!("{:02x}", b);
			md5_str += &a;
		}

		if ret == error_retcode(YaftpError::OK){
			match send_reply(stream, 0 , 1).await {
				Ok(_) => {},
				Err(_) => {
				},
			};
		}

		match send_argument(stream, &mut md5_str.as_bytes().to_vec()).await {
			Ok(_) => {},
			Err(e) => {
				log::error!("yaftp send argument error");
				ret = error_retcode(e);
				break;
			},
		};

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
		match stream.write_all(&[1u8, 10u8 , 1u8 , 2u8 , 3u8 , 4u8 , 5u8 , 6u8 , 7u8, 8u8, 9u8 , 10u8]).await{
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
			0x03 => {
				let _ = c_cp(&mut stream , narg ).await;
			},
			0x04 => {
				let _ = c_mkd(&mut stream , narg ).await;
			},
			0x05 => {
				let _ = c_mv(&mut stream , narg ).await;
			},
			0x06 => {
				let _ = c_rm(&mut stream , narg ).await;
			},
			0x08 => {
				let _ = c_get(&mut stream , narg ).await;
			}
			0x09 => {
				let _ = c_info(&mut stream , narg ).await;
			},
			0x0a => {
				let _ = c_hash(&mut stream , narg ).await;
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