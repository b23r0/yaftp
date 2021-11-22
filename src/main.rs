use futures::{StreamExt};
use async_std::{io, net::{TcpListener}, task};
use log::LevelFilter;
use simple_logger::SimpleLogger;

mod server;
mod client;
mod common;
mod cmd;

fn usage() {
	println!("yaftp - Yet Another File Transfer Protocol");
	println!("https://github.com/b23r0/yaftp");
	println!("Usage: yaftp [-l yaftp port] [-c ip port]");
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

			cmd::handle_cmd(ip , port).await;
		},
		"-t" => {
		},
		"-r" => {
		},
		_ => {
			usage();
		}
	}
    Ok(())
}
