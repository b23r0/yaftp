use std::fmt::Debug;
use std::fmt;
use std::error;
use std::io::Write;

use crate::client;
use crate::common::error_retcode;

use console::Term;
use console::style;
use tabled::{Tabled, Table};

#[derive(Tabled)]
struct FileInfo {
    name: String,
	typ: String,
	size: String,
	modified : String,
	accesstime : String
}

pub enum CmdError {
    NotFindQuotaUntilEnd
}

impl Debug for CmdError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CmdError::NotFindQuotaUntilEnd => write!(f, "NotFindQuotaUntilEnd"),
        }
    }
}

impl PartialEq for CmdError {
    fn eq(&self, other: &Self) -> bool {
        core::mem::discriminant(self) == core::mem::discriminant(other)
    }
}

impl fmt::Display for CmdError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CmdError::NotFindQuotaUntilEnd => write!(f, "NotFindQuotaUntilEnd"),
        }
    }
}

impl error::Error for CmdError {
    fn source(&self) -> Option<&(dyn error::Error + 'static)> {
        None
    }
}

#[derive(PartialEq)]
enum Token{
    Char = 1,
    Quota = 2 ,
    Space = 3
}

impl Clone for Token {
    fn clone(&self) -> Self {
        match self {
            Self::Char => Self::Char,
            Self::Quota => Self::Quota,
            Self::Space => Self::Space,
        }
    }
}

struct TokenSt{
    t : Token,
    v : u8
}

impl Clone for TokenSt {
    fn clone(&self) -> Self {
        Self { t: self.t.clone(), v: self.v.clone() }
    }
}

fn tokenizer( s : Vec<u8> ) -> Result<Vec<TokenSt> ,CmdError> {
    let mut ret : Vec<TokenSt> = vec![];

    let mut i = 0;
    while i < s.len() {
        match s[i] as char {
            '\"' => {
                ret.push(TokenSt{t : Token::Quota ,v: s[i]});
            },
            ' ' => {
                ret.push(TokenSt{t : Token::Space ,v: s[i]});
            },
            '\t' => {
                ret.push(TokenSt{t : Token::Space ,v: s[i]});
            },
            _ => {
                ret.push(TokenSt{t : Token::Char ,v: s[i]});
            }
        }
        i += 1;
    }

    Ok(ret)
}

fn eat(s : Vec<TokenSt> , end : Vec<Token>) -> (String , u8) {
    let mut ret: Vec<u8> = vec![];
    let mut i = 0;

    while i < s.len() {
        if !end.contains(&s[i].t) {
            ret.push(s[i].v);
        } else {
            break;
        }
        i += 1;
    }

    if i == s.len() {
        return ( String::from_utf8(ret).unwrap() , 1 ) ;
    }

    return ( String::from_utf8(ret).unwrap() , 0 ) ;
}

fn parser(s : Vec<TokenSt>) -> Result<Vec<String> , CmdError> {
    let mut ret : Vec<String> = vec![];
    
    let mut i = 0;
    while i < s.len() {
        match s[i].t {
            Token::Char => {
                let (a, _) = eat(s[i..].to_vec(), [Token::Space].to_vec());
                i += a.len();
                ret.push(a);
            }
            Token::Quota => {
                if i != s.len() - 1 {
                    let (a, e) = eat(s[i + 1..].to_vec(), [Token::Quota].to_vec());

                    if e != 0 {
                        return Err(CmdError::NotFindQuotaUntilEnd);
                    }
    
                    i += a.len() + 2;
                    ret.push(a);
                } else {
                    return Err(CmdError::NotFindQuotaUntilEnd);
                }

            },
            Token::Space => {
                let (a, _) = eat(s[i..].to_vec(), [Token::Char , Token::Quota].to_vec());
                i += a.len();
            },
        }
    } 

    Ok(ret)
}

pub fn cmd_format(input : String) -> Result<Vec<String> , CmdError>{

    let s = input.as_bytes();

    let tokens = match tokenizer(s.to_vec()) {
        Ok(p) => p,
        Err(e) => {
            return Err(e);
        }
    };

    let ret = match parser(tokens) {
        Ok(p) => p,
        Err(e) => {
            return Err(e)
        },
    };

    Ok(ret)
}

fn pre_handle_path (path : String , cwd : String) -> String{

	let mut cdpath : String;

	let is_windows = cwd.as_bytes()[0] != '/' as u8;

	if path == ".."{

		if is_windows {

			let pos = cwd.rfind('\\').unwrap();

			if pos == 2 && cwd.len() == 3{
				return "".to_string();	
			}

			cdpath = cwd.split_at(pos).0.to_string();

			if cdpath.len() == 2 {
				cdpath += &"\\".to_string();
			}

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
				if path.len() > 1 {
					if path.as_bytes()[1] == ':' as u8 {
						cdpath = path;
						break;
					} 
				}

				if cwd.len() == 3 {
					cdpath = cwd + &path;
				} else {
					cdpath = [cwd , path].join("\\");
				}
			} else {
				if path.as_bytes()[0] == '/' as u8{
					cdpath = path;
					break;
				}
				if cwd == "/" {
					cdpath = cwd + &path;
				} else {
					cdpath = [cwd , path].join("/");
				}
			}
			break;
		}
	}

	cdpath
}

pub async fn handle_cmd(ip : String , port : String){

    let mut term = Term::stdout();
    let mut client = match client::Client::new(ip.clone() , port.clone()).await{
        Ok(p) => p,
        Err(_) => return ,
    };
    let mut cwd = match client.cwd().await{
        Ok(p) => p,
        Err(e) => {
            log::error!("error code : {}" , error_retcode(e));
            return;
        },
    };
    loop {
        term.set_title("yaftp");
        let wt = format!("yaftp @ {} > ", style(cwd.clone()).red());
        term.write_all(wt.as_bytes()).unwrap();
        let mut cmd = String::new();
        match std::io::stdin().read_line(&mut cmd){
            Ok(_) => {},
            Err(_) => return ,
        };
        let cmd = cmd.trim().to_string();

        let cmd = match cmd_format(cmd){
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

            if cmd.len() != 1{
                println!("command 'ls' not any argument . eg : ls");
                continue;
            }

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
                println!("command 'cd' need 1 argument . eg : cd /var");
                continue;
            }

            if cmd[1] == "." {
                continue;
            }

            let cdpath = pre_handle_path(cmd[1].clone(), cwd.clone());

            if cdpath.len() == 0{
                continue;
            }

            let mut client = match client::Client::new(ip.clone() , port.clone()).await{
                Ok(p) => p,
                Err(_) => {
                    println!("connect to {}:{} faild", ip ,port);
                    continue;
                },
            };

            let (ret, path) = match client.info(cdpath.clone()).await{
                Ok(p) => p,
                Err(_) => {
                    continue;
                },
            };

            if ret[0] == 0 {
                cwd = path;
            } else {
                println!("'{}' not a path" , path);
            }

        }

        if cmd[0] == "cp" {
            if cmd.len() != 3{
                println!("command 'cp' need 2 argument . eg : cp /var/file1 /var/file2");
                continue;
            }

            let srcpath = pre_handle_path(cmd[1].clone(), cwd.clone());

            if srcpath.len() == 0{
                continue;
            }

            let targetpath = pre_handle_path(cmd[2].clone(), cwd.clone());

            if targetpath.len() == 0{
                continue;
            }

            let mut client = match client::Client::new(ip.clone() , port.clone()).await{
                Ok(p) => p,
                Err(_) => {
                    println!("connect to {}:{} faild", ip ,port);
                    continue;
                },
            };

            let (ret, _) = match client.info(srcpath.clone()).await{
                Ok(p) => p,
                Err(_) => {
                    continue;
                },
            };
            
            if ret[0] != 1 {
                println!("'{}' not a file" , srcpath);
                continue;
            }

            let mut client = match client::Client::new(ip.clone() , port.clone()).await{
                Ok(p) => p,
                Err(_) => {
                    println!("connect to {}:{} faild", ip ,port);
                    continue;
                },
            };

            let _ = match client.cp(srcpath.clone() , targetpath.clone()).await{
                Ok(_) => {
                    println!("copy file '{}' to '{}' success" , srcpath , targetpath);
                },
                Err(_) => {
                    continue;
                },
            };

        }

        if cmd[0] == "mkdir" {
            if cmd.len() != 2{
                println!("command 'mkdir' need 1 argument . eg : mkdir /var/folder1/folder2");
                continue;
            }

            let path = pre_handle_path(cmd[1].clone(), cwd.clone());

            if path.len() == 0{
                continue;
            }

            let mut client = match client::Client::new(ip.clone() , port.clone()).await{
                Ok(p) => p,
                Err(_) => {
                    println!("connect to {}:{} faild", ip ,port);
                    continue;
                },
            };

            let _ = match client.mkd(path.clone()).await{
                Ok(_) => {
                    println!("mkdir '{}' success" , path);
                },
                Err(_) => {
                    continue;
                },
            };

        }

        if cmd[0] == "mv" {
            if cmd.len() != 3{
                println!("command 'mv' need 2 argument . eg : mv /var/file1 /var/file2");
                continue;
            }

            let srcpath = pre_handle_path(cmd[1].clone(), cwd.clone());

            if srcpath.len() == 0{
                continue;
            }

            let targetpath = pre_handle_path(cmd[2].clone(), cwd.clone());

            if targetpath.len() == 0{
                continue;
            }

            let mut client = match client::Client::new(ip.clone() , port.clone()).await{
                Ok(p) => p,
                Err(_) => {
                    println!("connect to {}:{} faild", ip ,port);
                    continue;
                },
            };

            let (ret, _) = match client.info(srcpath.clone()).await{
                Ok(p) => p,
                Err(_) => {
                    continue;
                },
            };
            
            if ret[0] != 1 {
                println!("'{}' not a file" , srcpath);
                continue;
            }

            let mut client = match client::Client::new(ip.clone() , port.clone()).await{
                Ok(p) => p,
                Err(_) => {
                    println!("connect to {}:{} faild", ip ,port);
                    continue;
                },
            };

            let _ = match client.mv(srcpath.clone() , targetpath.clone()).await{
                Ok(_) => {
                    println!("move file '{}' to '{}' success" , srcpath , targetpath);
                },
                Err(_) => {
                    continue;
                },
            };
        }
        
        if cmd[0] == "rm" {
            if cmd.len() != 2{
                println!("command 'rm' need 1 argument . eg : rm /var/folder1/file2");
                continue;
            }

            let path = pre_handle_path(cmd[1].clone(), cwd.clone());

            if path.len() == 0{
                continue;
            }

            let mut client = match client::Client::new(ip.clone() , port.clone()).await{
                Ok(p) => p,
                Err(_) => {
                    println!("connect to {}:{} faild", ip ,port);
                    continue;
                },
            };

            let _ = match client.rm(path.clone()).await{
                Ok(_) => {
                    println!("remove '{}' success" , path);
                },
                Err(_) => {
                    continue;
                },
            };
        }

        if cmd[0] == "hash" {
            if cmd.len() != 2{
                println!("command 'hash' need 1 argument . eg : hash /var/folder1/file2");
                continue;
            }

            let path = pre_handle_path(cmd[1].clone(), cwd.clone());

            if path.len() == 0{
                continue;
            }

            let mut client = match client::Client::new(ip.clone() , port.clone()).await{
                Ok(p) => p,
                Err(_) => {
                    println!("connect to {}:{} faild", ip ,port);
                    continue;
                },
            };

            let (info, abspath) = match client.info(path.clone()).await{
                Ok(p) => p,
                Err(_) => {
                    continue;
                },
            };

            if info[0] != 1 {
                println!("'{}' not a file" , abspath);
                continue;
            }

            let mut client = match client::Client::new(ip.clone() , port.clone()).await{
                Ok(p) => p,
                Err(_) => {
                    println!("connect to {}:{} faild", ip ,port);
                    continue;
                },
            };

            match client.hash(path.clone() , info[1]).await{
                Ok(p) => {
                    println!("{}", p);
                },
                Err(_) => {
                    continue;
                },
            };

        }

        if cmd[0] == "get" {
            if cmd.len() != 2{
                println!("command 'get' need 1 argument . eg : get /var/folder1/file2");
                continue;
            }

            let path = pre_handle_path(cmd[1].clone(), cwd.clone());

            if path.len() == 0{
                continue;
            }

            let mut client = match client::Client::new(ip.clone() , port.clone()).await{
                Ok(p) => p,
                Err(_) => {
                    println!("connect to {}:{} faild", ip ,port);
                    continue;
                },
            };

            let (info, abspath) = match client.info(path.clone()).await{
                Ok(p) => p,
                Err(_) => {
                    continue;
                },
            };

            if info[0] != 1 {
                println!("'{}' not a file" , abspath);
                continue;
            }

            let mut client = match client::Client::new(ip.clone() , port.clone()).await{
                Ok(p) => p,
                Err(_) => {
                    println!("connect to {}:{} faild", ip ,port);
                    continue;
                },
            };

            match client.get(path.clone() , 0).await{
                Ok(_) => {},
                Err(_) => {
                    continue;
                },
            };

        }

        if cmd[0] == "put" {
            if cmd.len() != 2{
                println!("command 'put' need 1 argument . eg : put /localfile/file1");
                continue;
            }

            let localpath = cmd[1].clone();

            let filename : String;

            if cmd[1].as_bytes()[0] == '/' as u8 {
                filename = localpath.split_at(localpath.rfind('/').unwrap() + 1).1.to_string();
            } else {
                filename = localpath.split_at(localpath.rfind('\\').unwrap() + 1).1.to_string();
            }

            let remotepath = pre_handle_path(filename, cwd.clone());

            if localpath.len() == 0{
                continue;
            }

            let mut client = match client::Client::new(ip.clone() , port.clone()).await{
                Ok(p) => p,
                Err(_) => {
                    println!("connect to {}:{} faild", ip ,port);
                    continue;
                },
            };

            match client.info(remotepath.clone()).await{
                Ok(_) => {
                    println!("'{}' remote file already exists" ,localpath);
                    continue;
                },
                Err(_) => {},
            };

            let mut client = match client::Client::new(ip.clone() , port.clone()).await{
                Ok(p) => p,
                Err(_) => {
                    println!("connect to {}:{} faild", ip ,port);
                    continue;
                },
            };

            match client.put(localpath , remotepath , 0).await{
                Ok(_) => {},
                Err(_) => {
                    continue;
                },
            };

        }
    }
}

#[test]
fn test_cmd() {
    let ret = cmd_format("ls \"test\" 123".to_string()).unwrap();
    assert_eq!(ret[0] , String::from("ls"));
    assert_eq!(ret[1] , String::from("test"));
    assert_eq!(ret[2] , String::from("123"));

    let ret = cmd_format("ls \"test\"".to_string()).unwrap();

    assert_eq!(ret[0] , String::from("ls"));
    assert_eq!(ret[1] , String::from("test"));
}