
use futures::{AsyncReadExt, AsyncWriteExt, FutureExt, StreamExt};
use async_std::{io, net::{TcpListener, TcpStream}, task};
use futures::select;
use log::LevelFilter;
use simple_logger::SimpleLogger;

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
					log::error!("not found listen port . eg : rsocx -l 8000");
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
				});
			}
		},
		"-c" => {
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