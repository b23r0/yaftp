include!{"utils.rs"}

use std::{fs, net::Shutdown};

use futures::{AsyncReadExt, AsyncWriteExt, FutureExt, StreamExt};
use async_std::{io, net::{TcpListener, TcpStream}, task};

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

async fn c_ls(stream :&mut  TcpStream , narg : u32) {

    let mut ret = 0u8;

    if narg != 1 {
        log::error!("command [{}] arguments count unvalid : {}" , "ls", narg);
        ret = 8;
        return;
    }

    loop {
        /*
        +-----------------+---------------------+
        | NEXT_ARG_SIZE   |         ARG         |
        +-----------------+---------------------+
        |      4(u64)     |       Variable      |
        +-----------------+---------------------+
        */
        let mut size = [0u8 ; 8];
        match stream.read_exact(&mut size).await{
            Ok(_) => {},
            Err(e) => {
                log::error!("error : {}" , e);
                break;
            },
        };

        let size = u64::from_be_bytes(size);

        if size > 1024 {
            ret = 10;
            log::error!("command [{}] arguments size unvalid : {}" , "ls" , size);
            break;
        }

        let path = vec![0u8;size as usize].into_boxed_slice();

        let path = match String::from_utf8(path.to_vec()){
            Ok(p) => p,
            Err(e) => {
                ret = 9;
                log::error!("error : {}" , e);
                break;
            },
        };

        let paths = match fs::read_dir(path){
            Ok(p) => p,
            Err(_) => {
                ret = 11;
                break;
            },
        };

        let cols = String::from("Name");
        let mut rows1 : Vec<String> = Vec::new();

        for path in paths {
            rows1.push(String::from(path.unwrap().file_name().to_string_lossy()));
        }

        if ret == 0 {
            /*
            +-----------+-----------+
            |  RETCODE  |  NARG     |
            +-----------+-----------+
            |  1(u8)    |  4(u32)   |
            +-----------+-----------+
            */
            let narg : u32 = (rows1.len() + 1) as u32;
            let b_narg = narg.to_be_bytes();
            let mut args : Vec<u8> = Vec::new();

            args.push(ret);
            args.append(&mut b_narg.to_vec());
            match stream.write_all( &mut args).await{
                Ok(_) => {},
                Err(e) => {
                    log::error!("error : {}" , e);
                    return;
                },
            };

            let a = cols.as_bytes();

            /*
            +-----------------+---------------------+
            | NEXT_ARG_SIZE   |         ARG         |
            +-----------------+---------------------+
            |      4(u64)     |       Variable      |
            +-----------------+---------------------+
            */

            match stream.write_all( &(a.len() as u64).to_be_bytes()).await{
                Ok(_) => {},
                Err(e) => {
                    log::error!("error : {}" , e);
                    return;
                },
            };

            match stream.write_all( &a).await{
                Ok(_) => {},
                Err(e) => {
                    log::error!("error : {}" , e);
                    return;
                },
            };

            for i in rows1{
                let a = i.as_bytes();

                match stream.write_all( &(a.len() as u64).to_be_bytes()).await{
                    Ok(_) => {},
                    Err(e) => {
                        log::error!("error : {}" , e);
                        return;
                    },
                };
    
                match stream.write_all( &a).await{
                    Ok(_) => {},
                    Err(e) => {
                        log::error!("error : {}" , e);
                        return;
                    },
                };
            }
        }

        break;
    }

    if ret != 0{
        /*
        +-----------+-----------+
        |  RETCODE  |  NARG     |
        +-----------+-----------+
        |  1(u8)    |  4(u32)   |
        +-----------+-----------+
        */
        match stream.write_all( &[0,0,0,0,0]).await{
            Ok(_) => {},
            Err(e) => {
                log::error!("error : {}" , e);
                return;
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
        match stream.write_all(&[1u8, 8u8 , 1u8 , 2u8 , 3u8 , 4u8 , 5u8 , 6u8 , 7u8, 8u8]).await{
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

        
        let mut dst = [0u8; 4];
        dst.clone_from_slice(&command[1..4]);
        let narg = u32::from_be_bytes(dst);

        match command[0] {
            0x01 => c_ls(&mut stream , narg ).await,
            _ => {}
        }
        break;
    }

    match stream.shutdown(Shutdown::Both){
        Ok(_) => {},
        Err(_) => {},
    };


}