
use std::{fmt::format, io::Write};

use futures::{AsyncReadExt, AsyncWriteExt, FutureExt, StreamExt};
use async_std::{io, net::{TcpListener, TcpStream}, task};
use futures::select;
use log::LevelFilter;
use simple_logger::SimpleLogger;

mod server;
mod client;
use console::Term;
use console::style;
use tabled::{Tabled, Table};

#[derive(Tabled)]
struct FileInfo {
    name: String,
	typ: String,
	size: String,
}

fn usage() {

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
			let mut term = Term::stdout();
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

			let mut client = client::Client::new(ip.clone() , port.clone()).await?;
			let cwd = match client.cwd().await{
				Ok(p) => p,
				Err(e) => {
					log::error!("error : {}" , e);
					return Ok(());
				},
			};

			loop {
				term.set_title("yaftp");
				let wt = format!("yaftp @ {} > ", style(cwd.clone()).red());
				term.write_all(wt.as_bytes()).unwrap();
				let cmd = term.read_line().unwrap();

				if cmd == "ls" {
					let mut client = client::Client::new(ip.clone() , port.clone()).await?;
					let result = client.ls(String::from(cwd.clone())).await?;

					let mut files : Vec<FileInfo> = vec![];

					for i in result {
						let col : Vec<&str> = i.split("|").collect();
						files.push(FileInfo{name : col[0].to_string() , typ : col[1].to_string() , size : col[2].to_string()});
					}

					let table = Table::new(files).to_string();
					print!("{}",table);
				}
			}

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
