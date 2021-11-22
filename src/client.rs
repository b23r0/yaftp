include!("utils.rs");

use std::{io::Error, net::Shutdown};
use crate::common::{YaftpError, retcode_error};
use futures::{AsyncReadExt, AsyncWriteExt};
use async_std::{fs, net::{TcpStream}};

pub struct Client {
	conn : TcpStream
}

impl Drop for Client{
    fn drop(&mut self) {
		match self.conn.shutdown(Shutdown::Both){
			Ok(_) => {},
			Err(_) => {},
		};
    }
}

impl Client {
	
	pub async fn new(ip : String ,port : String) -> Result<Client , Error>{
		let fulladdr = format!("{}:{}", ip ,port);
		let conn = match TcpStream::connect(fulladdr).await{
			Ok(p) => p,
			Err(e) => return Err(e),
		};
		
		return Ok(Client{conn : conn});
	}

	async fn handshake(self : &mut Client) -> Result<Vec<u8> , YaftpError>{
		/*
		+-------+----------+---------------+
		|  VER  | NMETHODS | METHODS       |
		+-------+----------+---------------+
		| 1(u8) |   1(u8)  | 1 to 255 (u8) |
		+-------+----------+---------------+
		*/
		match self.conn.write_all(&[1u8, 8u8 , 1u8 , 2u8 , 3u8 , 4u8 , 5u8 , 6u8 , 7u8, 8u8]).await{
			Ok(_) => {},
			Err(_) => {
				return Err(YaftpError::UnknownNetwordError);
			},
		};

		/*
		+-------+----------+---------------+
		|  VER  | NMETHODS | METHODS	   |
		+-------+----------+---------------+
		| 1(u8) |   1(u8)  | 1 to 255 (u8) |
		+-------+----------+---------------+
		*/
		let mut header = [0u8;2];
		match self.conn.read_exact(&mut header).await{
			Ok(_) => {},
			Err(_) => {
				return Err(YaftpError::UnknownNetwordError);
			},
		};

		if header[0] != 0x01{
			return Err(YaftpError::NoSupportVersion);
		}

		let mut methods = vec![0u8;header[1].into()].into_boxed_slice();

		match self.conn.read_exact(&mut methods).await{
			Ok(_) => {},
			Err(_) => {
				return Err(YaftpError::UnknownNetwordError);
			},
		};

		if !check_support_methods(&methods){
			println_err!("the client has not support method");
		}

		Ok(methods.to_vec())
	}

	async fn send_command(self : &mut Client , command : u8 , narg : u32) -> Result<Vec<u8>, YaftpError>{
		/*
		+-------+--------+
		|  CMD  | NARG   |
		+-------+--------+
		| 1(u8) | 4(u32) |
		+-------+--------+
		*/
		let b_narg = narg.to_be_bytes();
		let mut command = vec![command];
		command.append(&mut b_narg.to_vec());

		match self.conn.write_all(&mut command).await{
			Ok(_) => {},
			Err(_) => {
				return Err(YaftpError::UnknownNetwordError);
			},
		};
		Ok(command)
	}

	async fn read_reply(self : &mut Client) -> Result<u32, YaftpError> {
		/*
		+-----------+-----------+
		|  RETCODE  |  NARG     |
		+-----------+-----------+
		|  1(u8)    |  4(u32)   |
		+-----------+-----------+
		*/
		let mut reply = [0u8;5];
		match self.conn.read_exact(&mut reply).await{
			Ok(_) => {},
			Err(_) => {
				return Err(YaftpError::UnknownNetwordError);
			},
		};

		if reply[0] != 0 {
			return Err(retcode_error(reply[0]));
		}

		Ok(u32::from_be_bytes(reply[1..5].try_into().unwrap()))
	}

	async fn send_argument(self : &mut Client , data :&mut Vec<u8>) -> Result<Vec<u8>, YaftpError>{
		/*
		+-----------------+---------------------+
		| NEXT_ARG_SIZE   |      ARG            |
		+-----------------+---------------------+
		|     8(u64)      |    Variable         |
		+-----------------+---------------------+
		*/
		let size = data.len() as u64;
		let b_size = size.to_be_bytes();
		let mut argument = vec![];
		argument.append(&mut b_size.to_vec());
		argument.append(data);

		match self.conn.write_all(&mut argument).await{
			Ok(_) => {},
			Err(_) => {
				return Err(YaftpError::UnknownNetwordError);
			},
		};
		Ok(argument)
	}

	async fn read_argument(self : &mut Client , max_size : u64) -> Result<Vec<u8>, YaftpError> {
		/*
		+-----------------+---------------------+
		| NEXT_ARG_SIZE   |	     ARG            |
		+-----------------+---------------------+
		|     8(u64)      |	   Variable         |
		+-----------------+---------------------+
		*/
		let mut argument_size = [0u8;8];
		match self.conn.read_exact(&mut argument_size).await{
			Ok(_) => {},
			Err(_) => {
				return Err(YaftpError::UnknownNetwordError);
			},
		};

		let size = u64::from_be_bytes(argument_size);

		if size > max_size {
			return Err(YaftpError::ArgumentError);
		}

		let mut arg = vec![0u8;size as usize].into_boxed_slice();

		match self.conn.read_exact(&mut arg).await{
			Ok(_) => {},
			Err(_) => {
				return Err(YaftpError::UnknownNetwordError);
			},
		};

		Ok(arg.to_vec())
	}

	pub async fn ls(self : &mut Client , path : String) -> Result<Vec<String> ,YaftpError> {

		match self.handshake().await{
			Ok(_) => {},
			Err(e) => {
				println_err!("yaftp handshake error");
				return Err(e);
			},
		};

		match self.send_command(1u8, 1).await{
			Ok(_) => {},
			Err(e) => {
				println_err!("yaftp send command error");
				return Err(e);
			},
		};

		match self.send_argument(&mut path.as_bytes().to_vec()).await{
			Ok(_) => {},
			Err(e) => {
				println_err!("yaftp send argument error");
				return Err(e);
			},
		};

		let narg = match self.read_reply().await{
			Ok(p) => p,
			Err(e) => {
				println_err!("server error code : {}" , e);
				return Err(e);
			},
		};
		
		let mut ret : Vec<String> = vec![];
		let mut i = 0 ; 
		while i < narg {
			let arg = match self.read_argument(2048).await{
				Ok(p) => p,
				Err(e) => {
					println_err!("yaftp read argument error");
					return Err(e);
				},
			};

			let row = match String::from_utf8(arg){
				Ok(p) => p,
				Err(_) => {
					println_err!("format argument to utf8 string faild");
					return Err(YaftpError::ArgumentError);
				},
			}; 
			ret.push(row);
			i +=1;
		}

		Ok(ret)
	}

	pub async fn info(self : &mut Client , path : String) -> Result<(Vec<u64> , String),YaftpError> {

		match self.handshake().await{
			Ok(_) => {},
			Err(e) => {
				println_err!("yaftp handshake error");
				return Err(e);
			},
		};

		match self.send_command(9u8, 1).await{
			Ok(_) => {},
			Err(e) => {
				println_err!("yaftp send command error");
				return Err(e);
			},
		};

		match self.send_argument(&mut path.as_bytes().to_vec()).await{
			Ok(_) => {},
			Err(e) => {
				println_err!("yaftp send argument error");
				return Err(e);
			},
		};

		let _ = match self.read_reply().await{
			Ok(p) => p,
			Err(e) => {
				//println!("server error code : {}" , e);
				return Err(e);
			},
		};
		
		let mut ret : Vec<u64> = vec![];

		let arg = match self.read_argument(1).await{
			Ok(p) => p,
			Err(e) => {
				println_err!("yaftp read argument error");
				return Err(e);
			},
		};

		ret.push(arg[0] as u64);

		let arg = match self.read_argument(8).await{
			Ok(p) => p,
			Err(e) => {
				println_err!("yaftp read argument error");
				return Err(e);
			},
		};

		let size = u64::from_be_bytes(arg[0..8].try_into().unwrap());
		ret.push(size);

		let arg = match self.read_argument(8).await{
			Ok(p) => p,
			Err(e) => {
				println_err!("yaftp read argument error");
				return Err(e);
			},
		};

		let mt = u64::from_be_bytes(arg[0..8].try_into().unwrap());
		ret.push(mt);

		let arg = match self.read_argument(8).await{
			Ok(p) => p,
			Err(e) => {
				println_err!("yaftp read argument error");
				return Err(e);
			},
		};

		let at = u64::from_be_bytes(arg[0..8].try_into().unwrap());
		ret.push(at);

		let arg = match self.read_argument(2048).await{
			Ok(p) => p,
			Err(e) => {
				println_err!("yaftp read argument error");
				return Err(e);
			},
		};

		let path = match String::from_utf8(arg) {
			Ok(p) => p,
			Err(_) => {
				println_err!("format argument to utf8 string faild");
				return Err(YaftpError::ArgumentError);
			}
		};
		
		Ok((ret , path))
	}

	pub async fn cwd(self : &mut Client) -> Result<String,YaftpError> {

		match self.handshake().await{
			Ok(_) => {},
			Err(e) => {
				println_err!("yaftp handshake error");
				return Err(e);
			},
		};

		match self.send_command(2u8, 0).await{
			Ok(_) => {},
			Err(e) => {
				println_err!("yaftp send command error");
				return Err(e);
			},
		};

		match self.read_reply().await{
			Ok(p) => p,
			Err(e) => {
				println_err!("server error code : {}" , e);
				return Err(e);
			},
		};

		let arg = match self.read_argument(2048).await{
			Ok(p) => p,
			Err(e) => {
				println_err!("yaftp read argument error");
				return Err(e);
			},
		};
		
		let ret = match String::from_utf8(arg) {
			Ok(p) => p,
			Err(_) => {
				println_err!("format argument to utf8 string faild");
				return Err(YaftpError::ArgumentError);
			}
		};
	
		Ok(ret)
	}

	pub async fn cp(self : &mut Client , srcpath : String , targetpath : String) -> Result<u32 , YaftpError> {

		match self.handshake().await{
			Ok(_) => {},
			Err(e) => {
				println_err!("yaftp handshake error");
				return Err(e);
			},
		};

		match self.send_command(3u8, 2).await{
			Ok(_) => {},
			Err(e) => {
				println_err!("yaftp send command error");
				return Err(e);
			},
		};

		match self.send_argument(&mut srcpath.as_bytes().to_vec()).await{
			Ok(_) => {},
			Err(e) => {
				println_err!("yaftp send argument error");
				return Err(e);
			},
		};

		match self.send_argument(&mut targetpath.as_bytes().to_vec()).await{
			Ok(_) => {},
			Err(e) => {
				println_err!("yaftp send argument error");
				return Err(e);
			},
		};

		match self.read_reply().await{
			Ok(p) => {
				return Ok(p);
			},
			Err(e) => {
				println_err!("server error code : {}" , e);
				return Err(e);
			},
		}
	}

	pub async fn mv(self : &mut Client , srcpath : String , targetpath : String) -> Result<u32 , YaftpError> {

		match self.handshake().await{
			Ok(_) => {},
			Err(e) => {
				println_err!("yaftp handshake error");
				return Err(e);
			},
		};

		match self.send_command(5u8, 2).await{
			Ok(_) => {},
			Err(e) => {
				println_err!("yaftp send command error");
				return Err(e);
			},
		};

		match self.send_argument(&mut srcpath.as_bytes().to_vec()).await{
			Ok(_) => {},
			Err(e) => {
				println_err!("yaftp send argument error");
				return Err(e);
			},
		};

		match self.send_argument(&mut targetpath.as_bytes().to_vec()).await{
			Ok(_) => {},
			Err(e) => {
				println_err!("yaftp send argument error");
				return Err(e);
			},
		};

		match self.read_reply().await{
			Ok(p) => {
				return Ok(p);
			},
			Err(e) => {
				println_err!("server error code : {}" , e);
				return Err(e);
			},
		}
	}

	pub async fn mkd(self : &mut Client , path : String) -> Result<u32 , YaftpError> {

		match self.handshake().await{
			Ok(_) => {},
			Err(e) => {
				println_err!("yaftp handshake error");
				return Err(e);
			},
		};

		match self.send_command(4u8, 1).await{
			Ok(_) => {},
			Err(e) => {
				println_err!("yaftp send command error");
				return Err(e);
			},
		};

		match self.send_argument(&mut path.as_bytes().to_vec()).await{
			Ok(_) => {},
			Err(e) => {
				println_err!("yaftp send argument error");
				return Err(e);
			},
		};

		match self.read_reply().await{
			Ok(p) => {
				return Ok(p);
			},
			Err(e) => {
				println_err!("server error code : {}" , e);
				return Err(e);
			},
		}
	}

	pub async fn rm(self : &mut Client , path : String) -> Result<u32 , YaftpError> {

		match self.handshake().await{
			Ok(_) => {},
			Err(e) => {
				println_err!("yaftp handshake error");
				return Err(e);
			},
		};

		match self.send_command(6u8, 1).await{
			Ok(_) => {},
			Err(e) => {
				println_err!("yaftp send command error");
				return Err(e);
			},
		};

		match self.send_argument(&mut path.as_bytes().to_vec()).await{
			Ok(_) => {},
			Err(e) => {
				println_err!("yaftp send argument error");
				return Err(e);
			},
		};

		match self.read_reply().await{
			Ok(p) => {
				return Ok(p);
			},
			Err(e) => {
				println_err!("server error code : {}" , e);
				return Err(e);
			},
		}
	}

	pub async fn put(self : &mut Client , localpath : String ,remotepath : String , start_pos : u64) -> Result<String,YaftpError> {

		match self.handshake().await{
			Ok(_) => {},
			Err(e) => {
				println_err!("yaftp handshake error");
				return Err(e);
			},
		};

		match self.send_command(7u8, 3).await{
			Ok(_) => {},
			Err(e) => {
				println_err!("yaftp send command error");
				return Err(e);
			},
		};

		match self.send_argument(&mut remotepath.as_bytes().to_vec()).await{
			Ok(_) => {},
			Err(e) => {
				println_err!("yaftp send argument error");
				return Err(e);
			},
		};

		match self.send_argument(&mut start_pos.to_be_bytes().to_vec()).await{
			Ok(_) => {},
			Err(e) => {
				println_err!("yaftp send argument error");
				return Err(e);
			},
		};

		let mut f = match fs::File::open(localpath.clone()).await{
			Ok(f) => f,
			Err(_) => {
				println_err!("open local file faild : {}" , localpath);
				return Err(YaftpError::UnknownError);
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

		match self.conn.write_all(&size.to_be_bytes().to_vec()).await{
			Ok(_) => {},
			Err(e) => {
				println_err!("file transfer faild : {}" ,e);
				return Err(YaftpError::UnknownNetwordError);
			}
		};

		let mut buf = [0;2048];
		let mut sum = 0u64;
		loop{
			let a = match f.read(&mut buf).await{
				Ok(p) => p,
				Err(e) => {
					println_err!("file transfer faild : {}" , e);
					return Err(YaftpError::UnknownError);
				},
			};

			match self.conn.write_all(&buf[..a]).await{
				Ok(p) => p,
				Err(e) => {
					println_err!("file transfer faild : {}" , e);
					return Err(YaftpError::UnknownError);
				},
			};

			sum += a as u64;

			if sum >= size {
				break
			}
		}

		f.close().await.unwrap();

		let _ = match self.read_reply().await{
			Ok(p) => p,
			Err(e) => {
				println_err!("server error code : {}" , e);
				return Err(e);
			},
		};
	
		Ok(remotepath)
	}

	pub async fn get(self : &mut Client , path : String , start_pos : u64) -> Result<String,YaftpError> {

		match self.handshake().await{
			Ok(_) => {},
			Err(e) => {
				println_err!("yaftp handshake error");
				return Err(e);
			},
		};

		match self.send_command(8u8, 2).await{
			Ok(_) => {},
			Err(e) => {
				println_err!("yaftp send command error");
				return Err(e);
			},
		};

		match self.send_argument(&mut path.as_bytes().to_vec()).await{
			Ok(_) => {},
			Err(e) => {
				println_err!("yaftp send argument error");
				return Err(e);
			},
		};

		match self.send_argument(&mut start_pos.to_be_bytes().to_vec()).await{
			Ok(_) => {},
			Err(e) => {
				println_err!("yaftp send argument error");
				return Err(e);
			},
		};

		let _ = match self.read_reply().await{
			Ok(p) => p,
			Err(e) => {
				println_err!("server error code : {}" , e);
				return Err(e);
			},
		};
	
		/*
		+-----------------+---------------------+
		| NEXT_ARG_SIZE   |       ARG           |
		+-----------------+---------------------+
		|     8(u64)      |    Variable         |
		+-----------------+---------------------+
		*/

		let mut argument_size = [0u8;8];
		match self.conn.read_exact(&mut argument_size).await{
			Ok(_) => {},
			Err(_) => {
				println_err!("read file size faild!");
				return Err(YaftpError::UnknownNetwordError);
			},
		};

		let size = u64::from_be_bytes(argument_size);

		let filename : String;

		if path.as_bytes()[0] == '/' as u8 {
			filename = path.split_at(path.rfind('/').unwrap() + 1).1.to_string();
		} else {
			filename = path.split_at(path.rfind('\\').unwrap() + 1).1.to_string();
		}

		let mut f = match fs::File::create(filename.clone()).await{
			Ok(f) => f,
			Err(_) => {
				println_err!("create local file faild : {}" , filename);
				return Err(YaftpError::UnknownError);
			},
		};

		let mut buf = [0;2048];
		let mut sum = 0u64;
		loop{
			let a = match self.conn.read(&mut buf).await{
				Ok(p) => p,
				Err(e) => {
					println_err!("file transfer faild : {}" , e);
					return Err(YaftpError::UnknownError);
				},
			};

			match f.write_all(&buf[..a]).await{
				Ok(p) => p,
				Err(e) => {
					println_err!("file transfer faild : {}" , e);
					return Err(YaftpError::UnknownError);
				},
			};

			sum += a as u64;

			if sum >= size {
				break
			}
		}

		f.close().await.unwrap();

		Ok(filename)
	}

	pub async fn hash(self : &mut Client , path : String , end_pos : u64) -> Result<String,YaftpError> {

		match self.handshake().await{
			Ok(_) => {},
			Err(e) => {
				println_err!("yaftp handshake error");
				return Err(e);
			},
		};

		match self.send_command(10u8, 2).await{
			Ok(_) => {},
			Err(e) => {
				println_err!("yaftp send command error");
				return Err(e);
			},
		};

		match self.send_argument(&mut path.as_bytes().to_vec()).await{
			Ok(_) => {},
			Err(e) => {
				println_err!("yaftp send argument error");
				return Err(e);
			},
		};

		match self.send_argument(&mut end_pos.to_be_bytes().to_vec()).await{
			Ok(_) => {},
			Err(e) => {
				println_err!("yaftp send argument error");
				return Err(e);
			},
		};

		match self.read_reply().await{
			Ok(p) => p,
			Err(e) => {
				println_err!("server error code : {}" , e);
				return Err(e);
			},
		};

		let arg = match self.read_argument(32).await{
			Ok(p) => p,
			Err(e) => {
				println_err!("yaftp read argument error");
				return Err(e);
			},
		};
		
		let ret = match String::from_utf8(arg) {
			Ok(p) => p,
			Err(_) => {
				println_err!("format argument to utf8 string faild");
				return Err(YaftpError::ArgumentError);
			}
		};
	
		Ok(ret)
	}
}