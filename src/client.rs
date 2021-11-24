use futures::{AsyncReadExt, AsyncSeekExt, AsyncWriteExt};
use async_std::{fs::{self, File}, net::{TcpListener, TcpStream}};
use indicatif::{ProgressBar, ProgressStyle};

use std::{io::{Error, SeekFrom}, net::Shutdown};
use crate::{common::{YaftpError, retcode_error}, println_err, println_info, utils::*};

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

pub struct SpawnClient {
	master : Option<TcpStream>,
	slave : Option<TcpListener>,
	ip : String ,
	port : String 

}

impl SpawnClient {

	pub async fn new(ip : &String , port : &String) -> SpawnClient {
		SpawnClient{ip : ip.clone() , port : port.clone() , master : None , slave : None}
	}

	pub async fn new_t(master : TcpStream , slave : TcpListener) -> SpawnClient {
		SpawnClient{ip : String::new() , port : String::new() , master : Some(master) , slave : Some(slave)}
	}

	pub async fn spawn(self : &SpawnClient) -> Result<Client , Error> {

		if self.ip.len() == 0 {
			self.master.as_ref().unwrap().write_all(&mut [0x55, 0x55].to_vec()).await?;
			let (stream , _) = self.slave.as_ref().unwrap().accept().await?;

			return Client::from(stream).await;
		}

		return Client::new(&self.ip , &self.port).await;
	}
}

impl Client {
	
	pub async fn new(ip : &String ,port : &String) -> Result<Client , Error>{
		let fulladdr = format!("{}:{}", ip ,port);
		let conn = match TcpStream::connect(fulladdr).await{
			Ok(p) => p,
			Err(e) => return Err(e),
		};
		
		return Ok(Client{conn : conn});
	}

	pub async fn from(conn : TcpStream) -> Result<Client , Error> {
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

	pub async fn ls(self : &mut Client , path : &String) -> Result<Vec<String> ,YaftpError> {

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

	pub async fn info(self : &mut Client , path : &String) -> Result<(Vec<u64> , String),YaftpError> {

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

	pub async fn cp(self : &mut Client , srcpath : &String , targetpath : &String) -> Result<u32 , YaftpError> {

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

	pub async fn mv(self : &mut Client , srcpath : &String , targetpath : &String) -> Result<u32 , YaftpError> {

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

	pub async fn mkd(self : &mut Client , path : &String) -> Result<u32 , YaftpError> {

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

	pub async fn rm(self : &mut Client , path : &String) -> Result<u32 , YaftpError> {

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

	pub async fn put(self : &mut Client , localpath : &String ,remotepath : &String , start_pos : u64) -> Result<String,YaftpError> {

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

		let size = f.metadata().await.unwrap().len() - start_pos;

		match self.conn.write_all(&size.to_be_bytes().to_vec()).await{
			Ok(_) => {},
			Err(e) => {
				println_err!("file transfer faild : {}" ,e);
				return Err(YaftpError::UnknownNetwordError);
			}
		};

		let mut buf = [0;2048];
		let mut sum = 0u64;

		match f.seek(SeekFrom::Start(start_pos)).await{
			Ok(_) => {},
			Err(e) => {
				println_err!("local file seek pos faild : {}" ,e);
				return Err(YaftpError::StartPosError);
			}
		};

		let pb = ProgressBar::new(size);
		pb.set_style(ProgressStyle::default_bar()
			.template("[{elapsed_precise}] [{wide_bar:.cyan/blue}] {bytes}/{total_bytes} ({eta})")
			.progress_chars("#>-"));
	

		loop{
			let a = match f.read(&mut buf).await{
				Ok(p) => p,
				Err(e) => {
					pb.finish_with_message("[-] file transfer faild");
					println_err!("file transfer faild : {}" , e);
					return Err(YaftpError::UnknownError);
				},
			};

			if a == 0 {
				break;
			}

			match self.conn.write_all(&buf[..a]).await{
				Ok(p) => p,
				Err(e) => {
					pb.finish_with_message("[-] file transfer faild");
					println_err!("file transfer faild : {}" , e);
					return Err(YaftpError::UnknownError);
				},
			};
			pb.set_position(sum);
			sum += a as u64;

			if sum >= size {
				break
			}
		}

		pb.finish_with_message("[+] finished");

		f.close().await.unwrap();

		let _ = match self.read_reply().await{
			Ok(p) => p,
			Err(e) => {
				println_err!("server error code : {}" , e);
				return Err(e);
			},
		};
	
		Ok(remotepath.clone())
	}

	pub async fn get(self : &mut Client ,localpath : &String ,remotepath : &String , start_pos : u64) -> Result<String,YaftpError> {

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

		let mut f : File;

		if start_pos == 0{
			f = match fs::File::create(localpath.clone()).await{
				Ok(f) => f,
				Err(_) => {
					println_err!("create local file faild : {}" , localpath);
					return Err(YaftpError::UnknownError);
				},
			};
		} else {
			f = match async_std::fs::OpenOptions::new().write(true).read(true).open(localpath.clone()).await{
				Ok(p) => p,
				Err(_) => {
					println_err!("open local file faild : {}" , localpath);
					return Err(YaftpError::UnknownError);
				}
			};
		}

		if start_pos != 0 {

			let pos : SeekFrom;

			if start_pos == f.metadata().await.unwrap().len(){
				pos = SeekFrom::End(0);
			} else {
				pos = SeekFrom::Start(start_pos);
			}

			match f.seek(pos).await{
				Ok(_) => {},
				Err(e) => {
					println_info!("seek local file faild : {}" , e);
					return Err(YaftpError::StartPosError);
				},
			};
		}

		let pb = ProgressBar::new(size);
		pb.set_style(ProgressStyle::default_bar()
			.template("[{elapsed_precise}] [{wide_bar:.cyan/blue}] {bytes}/{total_bytes} ({eta})")
			.progress_chars("#>-"));

		let mut buf = [0;2048];
		let mut sum = 0u64;
		loop{
			let a = match self.conn.read(&mut buf).await{
				Ok(p) => p,
				Err(e) => {
					pb.finish_with_message("[-] finished");
					println_err!("file transfer faild : {}" , e);
					return Err(YaftpError::UnknownError);
				},
			};

			match f.write_all(&buf[..a]).await{
				Ok(p) => p,
				Err(e) => {
					pb.finish_with_message("[-] finished");
					println_err!("file transfer faild : {}" , e);
					return Err(YaftpError::UnknownError);
				},
			};

			pb.set_position(sum);
			sum += a as u64;

			if sum >= size {
				break
			}
		}

		pb.finish_with_message("[+] finished");

		f.close().await.unwrap();

		Ok(localpath.clone())
	}

	pub async fn hash(self : &mut Client , path : &String , end_pos : u64) -> Result<String,YaftpError> {

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