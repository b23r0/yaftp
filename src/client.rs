use std::io::Error;

use futures::{AsyncReadExt, AsyncWriteExt, FutureExt, StreamExt};
use async_std::{io, net::{TcpStream}};

pub struct Client {
	conn : TcpStream
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

	async fn check_support_methods(self : &mut Client , methods : &[u8]) -> bool {
	
		let mut i = 0 ;
		while i < methods.len() {
			if methods[i] > 0x08 {
				return false;
			}
			i += 1;
		}
		
		true
	}

	async fn handshake(self : &mut Client) -> Result<Vec<u8> , Error>{
		/*
		+-------+----------+---------------+
		|  VER  | NMETHODS | METHODS       |
		+-------+----------+---------------+
		| 1(u8) |   1(u8)  | 1 to 255 (u8) |
		+-------+----------+---------------+
		*/
		match self.conn.write_all(&[1u8, 8u8 , 1u8 , 2u8 , 3u8 , 4u8 , 5u8 , 6u8 , 7u8, 8u8]).await{
			Ok(_) => {},
			Err(e) => {
				return Err(e);
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
			Err(e) => {
				return Err(e);
			},
		};

		if header[0] != 0x01{
			return Err(Error::new(io::ErrorKind::InvalidData, "not support the version"));
		}

		let mut methods = vec![0u8;header[1].into()].into_boxed_slice();

		match self.conn.read_exact(&mut methods).await{
			Ok(_) => {},
			Err(e) => {
				return Err(e);
			},
		};

		if !self.check_support_methods(&methods).await{
			println!("the client has not support method");
		}

		Ok(methods.to_vec())
	}

	async fn send_command(self : &mut Client , command : u8 , narg : u32) -> Result<Vec<u8>, Error>{
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
			Err(e) => {
				return Err(e);
			},
		};
		Ok(command)
	}

	async fn read_reply(self : &mut Client) -> Result<u32, Error> {
		/*
		+-----------+-----------+
		|  RETCODE  |  NARG	    |
		+-----------+-----------+
		|  1(u8)	|  4(u32)   |
		+-----------+-----------+
		*/
		let mut reply = [0u8;5];
		match self.conn.read_exact(&mut reply).await{
			Ok(_) => {},
			Err(e) => {
				return Err(e);
			},
		};

		if reply[0] != 0 {
			let err = format!("error code : {}" , reply[0]);
			return Err(Error::new(io::ErrorKind::InvalidData, err));
		}

		Ok(u32::from_be_bytes(reply[1..5].try_into().unwrap()))
	}

	async fn send_argument(self : &mut Client , data :&mut Vec<u8>) -> Result<Vec<u8>, Error>{
		/*
		+-----------------+---------------------+
		| NEXT_ARG_SIZE   |	     ARG            |
		+-----------------+---------------------+
		|     8(u64)      |	   Variable	        |
		+-----------------+---------------------+
		*/
		let size = data.len() as u64;
		let b_size = size.to_be_bytes();
		let mut argument = vec![];
		argument.append(&mut b_size.to_vec());
		argument.append(data);

		match self.conn.write_all(&mut argument).await{
			Ok(_) => {},
			Err(e) => {
				return Err(e);
			},
		};
		Ok(argument)
	}

	async fn read_argument(self : &mut Client , max_size : u64) -> Result<Vec<u8>, Error> {
		/*
		+-----------------+---------------------+
		| NEXT_ARG_SIZE   |	     ARG            |
		+-----------------+---------------------+
		|     8(u64)      |	   Variable	        |
		+-----------------+---------------------+
		*/
		let mut argument_size = [0u8;8];
		match self.conn.read_exact(&mut argument_size).await{
			Ok(_) => {},
			Err(e) => {
				return Err(e);
			},
		};

		let size = u64::from_be_bytes(argument_size);

		if size > max_size {
			return Err(Error::new(std::io::ErrorKind::InvalidData, "argument size error"));
		}

		let mut arg = vec![0u8;size as usize].into_boxed_slice();

		match self.conn.read_exact(&mut arg).await{
			Ok(_) => {},
			Err(e) => {
				return Err(e);
			},
		};

		Ok(arg.to_vec())
	}

	pub async fn ls(self : &mut Client , path : String) -> Result<Vec<String> ,Error> {

		match self.handshake().await{
			Ok(_) => {},
			Err(e) => {
				println!("{}" , e);
				return Err(e);
			},
		};

		match self.send_command(1u8, 1).await{
			Ok(_) => {},
			Err(e) => {
				println!("{}" , e);
				return Err(e);
			},
		};

		match self.send_argument(&mut path.as_bytes().to_vec()).await{
			Ok(_) => {},
			Err(e) => {
				println!("{}" , e);
				return Err(e);
			},
		};

		let narg = match self.read_reply().await{
			Ok(p) => p,
			Err(e) => {
				println!("{}" , e);
				return Err(e);
			},
		};
		
		let mut ret : Vec<String> = vec![];
		let mut i = 0 ; 
		while i < narg {
			let arg = match self.read_argument(2048).await{
				Ok(p) => p,
				Err(e) => {
					println!("{}" , e);
					return Err(e);
				},
			};

			let row = match String::from_utf8(arg){
				Ok(p) => p,
				Err(e) => {
					println!("{}" , e);
					return Err(Error::new(io::ErrorKind::InvalidData, "format utf8 faild"));
				},
			}; 
			ret.push(row);
			i +=1;
		}

		Ok(ret)
	}

	pub async fn cwd(self : &mut Client) -> Result<String,Error> {

		match self.handshake().await{
			Ok(_) => {},
			Err(e) => {
				println!("{}" , e);
				return Err(e);
			},
		};

		match self.send_command(2u8, 0).await{
			Ok(_) => {},
			Err(e) => {
				println!("{}" , e);
				return Err(e);
			},
		};

		match self.read_reply().await{
			Ok(p) => p,
			Err(e) => {
				println!("{}" , e);
				return Err(e);
			},
		};

		let arg = match self.read_argument(2048).await{
			Ok(p) => p,
			Err(e) => {
				println!("{}" , e);
				return Err(e);
			},
		};
		
		let ret = match String::from_utf8(arg) {
			Ok(p) => p,
			Err(e) => {
				println!("{}" , e);
				return Err(Error::new(io::ErrorKind::InvalidData, "format utf8 faild"));
			}
		};
	
		Ok(ret)
	}
}