
use std::{ io::Write};

use futures::{StreamExt};
use async_std::{io, net::{TcpListener}, task};
use log::LevelFilter;
use simple_logger::SimpleLogger;

mod server;
mod client;
mod common;
mod cmd;
use console::Term;
use console::style;
use tabled::{Tabled, Table};

use crate::{common::error_retcode};

#[derive(Tabled)]
struct FileInfo {
    name: String,
	typ: String,
	size: String,
	modified : String,
	accesstime : String
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
			let mut cwd = match client.cwd().await{
				Ok(p) => p,
				Err(e) => {
					log::error!("error code : {}" , error_retcode(e));
					return Ok(());
				},
			};

			loop {
				term.set_title("yaftp");
				let wt = format!("yaftp @ {} > ", style(cwd.clone()).red());
				term.write_all(wt.as_bytes()).unwrap();
				let cmd = term.read_line().unwrap();

				let cmd = match cmd::cmd(cmd){
					Ok(p) => p,
					Err(_) => {
						println!("command parser faild");
						continue;
					},
				};

				if cmd.len() == 0{
					continue;
				}

				if cmd[0] == "ls" {
					let mut client = match client::Client::new(ip.clone() , port.clone()).await{
						Ok(p) => p,
						Err(_) => {
							println!("connect to {}:{} faild", ip ,port);
							continue;
						},
					};
					let result = match client.ls(String::from(cwd.clone())).await{
						Ok(p) => p,
						Err(_) => {
							println!("command execute faild");
							continue;
						},
					};

					let mut files : Vec<FileInfo> = vec![];

					for i in result {
						let col : Vec<&str> = i.split("|").collect();
						files.push(FileInfo{name : col[0].to_string() , typ : col[1].to_string() , size : col[2].to_string() , modified : col[3].to_string() , accesstime : col[4].to_string()});
					}

					let table = Table::new(files).to_string();
					print!("{}",table);
				}
				
				if cmd[0] == "cd" {
					if cmd.len() != 2{
						println!("command 'cd' need two argument . eg : cd /var");
						continue;
					}

					if cmd[1] == "." {
						continue;
					}

					let mut cdpath : String;

					let mut client = match client::Client::new(ip.clone() , port.clone()).await{
						Ok(p) => p,
						Err(_) => {
							println!("connect to {}:{} faild", ip ,port);
							continue;
						},
					};

					let is_windows = cwd.as_bytes()[0] != '/' as u8;

					if cmd[1] == ".."{

						if is_windows {

							let pos = cwd.rfind('\\').unwrap();

							if pos == 2 && cwd.len() == 3{
								continue;	
							}

							cdpath = cwd.split_at(pos).0.to_string();

						} else {
							let pos = cwd.rfind('/').unwrap();

							cdpath = cwd.split_at(pos).0.to_string();

							if cdpath.len() == 0 {
								cdpath = "/".to_string();
							}
						}
						
					} else {

						loop {

							if is_windows {
								if cmd[1].len() > 1 {
									if cmd[1].as_bytes()[1] == ':' as u8 {
										cdpath = cmd[1].clone();
										break;
									} 
								}

								if cwd.len() == 3 {
									cdpath = cwd.clone() + &cmd[1].clone();
								} else {
									cdpath = [cwd.clone() , cmd[1].clone()].join("\\");
								}
							} else {
								if cmd[1].as_bytes()[0] == '/' as u8{
									cdpath = cmd[1].clone();
									break;
								}
								if cwd == "/" {
									cdpath = cwd.clone() + &cmd[1].clone();
								} else {
									cdpath = [cwd.clone() , cmd[1].clone()].join("/");
								}
							}
							break;
						}
					}
					
					let ret = match client.info(cdpath.clone()).await{
						Ok(p) => p,
						Err(_) => {
							println!("check folder status faild");
							continue;
						},
					};

					if ret[0] == 0 {
						cwd = cdpath;
					} else {
						println!("'{}' not a path" , cdpath);
					}

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
