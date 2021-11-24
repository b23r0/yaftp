use client::SpawnClient;
use futures::{AsyncReadExt, StreamExt};
use async_std::{io, net::{TcpListener, TcpStream}, task};
use log::LevelFilter;
use simple_logger::SimpleLogger;

mod server;
mod client;
mod common;
mod cmd;
mod utils;

fn usage() {
	println!("yaftp - Yet Another File Transfer Protocol");
	println!("https://github.com/b23r0/yaftp");
	println!("Usage: yaftp [-l yaftp port] [-c ip port] [-t reverse port] [-r [master ip] [master port]]");
}

#[async_std::main]
async fn main() -> io::Result<()>  {
    SimpleLogger::new().with_colors(true).init().unwrap();
	::log::set_max_level(LevelFilter::Info);

	let arg_count = std::env::args().count();

	if  arg_count == 1{
		usage();
		return Ok(());
	}

	let first  = std::env::args().nth(1).unwrap();

	match first.as_str() {
		"-l" => {
            let port = match std::env::args().nth(2){
				None => {
					log::error!("not found listen port . eg : yaftp -l 8000");
					return Ok(());
				},
				Some(p) => p
			};
			log::info!("listen to : {}" , "0.0.0.0:".to_string() + &port);
			
			let listener = match TcpListener::bind("0.0.0.0:".to_string() + &port).await{
				Err(e) => {
					log::error!("error : {}", e);
					return Ok(());
				},
				Ok(p) => p
			};

			let mut incoming = listener.incoming();

			while let Some(stream) = incoming.next().await {
				let stream = stream?;
				task::spawn(async {
					server::yaftp_server_handle(stream).await;
				});
			}
		},
		"-c" => {
			let ip = match std::env::args().nth(2){
				None => {
					log::error!("not found listen port . eg : yaftp -c 127.0.0.1 8000");
					return Ok(());
				},
				Some(p) => p
			};
			let port = match std::env::args().nth(3){
				None => {
					log::error!("not found listen port . eg : yaftp -c 127.0.0.1 8000");
					return Ok(());
				},
				Some(p) => p
			};

			let spawner = SpawnClient::new(&ip , &port).await;

			cmd::handle_cmd(spawner).await;
		},
		"-t" => {
			let master_port = match std::env::args().nth(2){
				None => {
					log::error!("not found listen port . eg : yaftp -t 9000");
					return Ok(());
				},
				Some(p) => p
			};

			log::info!("listen to : {} waiting for slave" , "0.0.0.0:".to_string() + &master_port);
			
			let slave_listener = match TcpListener::bind("0.0.0.0:".to_string() + &master_port).await{
				Err(e) => {
					log::error!("error : {}", e);
					return Ok(());
				},
				Ok(p) => p
			};

			let stream  = match slave_listener.accept().await{
				Ok(p) => {
					log::info!("accept from : {}:{}" , p.1.ip() , p.1.port() );
					p.0
				},
				Err(e) => {
					log::error!("accept error : {}" , e);
					return Ok(());
				},
			};

			let spawner = SpawnClient::new_t(stream, slave_listener).await;

			cmd::handle_cmd(spawner).await;
		},
		"-r" => {
			let addr = match std::env::args().nth(2){
				None => {
					log::error!("not found ip . eg : yaftp -r 192.168.0.1 9000");
					return Ok(());
				},
				Some(p) => p
			};
			let port = match std::env::args().nth(3){
				None => {
					log::error!("not found port . eg : yaftp -r 192.168.0.1 9000");
					return Ok(());
				},
				Some(p) => p
			};
			let fulladdr = format!("{}:{}" , addr , port);
			let mut master_stream = match TcpStream::connect(fulladdr.clone()).await{
				Err(e) => {
					log::error!("error : {}", e);
					return Ok(());
				},
				Ok(p) => p
			};
			log::info!("connect to {} success" ,fulladdr );
			loop {
				let mut buf = [0u8 ; 1];
				match master_stream.read_exact(&mut buf).await{
					Err(e) => {
						log::error!("error : {}", e);
						return Ok(());
					},
					Ok(p) => p
				};
	
				if buf[0] == 0x55 {
					let stream = match TcpStream::connect(fulladdr.clone()).await{
						Err(e) => {
							log::error!("error : {}", e);
							return Ok(());
						},
						Ok(p) => p
					};

					task::spawn(async {
						server::yaftp_server_handle(stream).await;
					});
				}
			}
		},
		_ => {
			usage();
		}
	}
    Ok(())
}
