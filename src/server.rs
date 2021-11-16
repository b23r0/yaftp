include!{"utils.rs"}

use std::net::Shutdown;

use async_std::{io::{self, WriteExt}, net::{TcpListener, TcpStream}, task};
use futures::AsyncReadExt;

fn check_support_methods(methods : &[u8]) -> bool {
    
    let mut i = 0 ;
    while i < methods.len() {
        if methods[i] > 0x06 {
            return false;
        }
        i += 1;
    }
    
    true
}

async fn c_ls(stream :&mut  TcpStream , narg : u8) {

    if narg != 1 {
        log::error!("command [{}] arguments count unvalid : {}" , "ls", narg);
        return;
    }

    let mut size = [0u8 ; 8];

    loop {
        match stream.read_exact(&mut size).await{
            Ok(_) => {},
            Err(e) => {
                log::error!("error : {}" , e);
                break;
            },
        };

        let size = u64::from_be_bytes(size);

        if size > 1024 {
            log::error!("command [{}] arguments size unvalid : {}" , "ls" , size);
            break;
        }

        let mut path = vec![0u8;size as usize].into_boxed_slice();

        let path = String::from_utf8(path.to_vec());


    }


}

pub async fn yaftp_server_handle(mut stream : TcpStream){

    loop {
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

        stream.write_all(&[1u8, 8u8 , 1u8 , 2u8 , 3u8 , 4u8 , 5u8 , 6u8 , 7u8, 8u8]);

        let mut command = [0u8;2];
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

        match command[0] {
            0x01 => c_ls(&mut stream , command[1] ).await,
            _ => {}
        }
        break;
    }

    match stream.shutdown(Shutdown::Both){
        Ok(_) => {},
        Err(_) => {},
    };


}