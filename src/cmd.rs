use std::fmt::Debug;
use std::fmt;
use std::error;
use std::io::Write;

use crate::client::SpawnClient;
use crate::common::error_retcode;
use crate::println_err;
use crate::println_info;
use crate::utils::calc_md5;

use async_std::fs;
use console::Term;
use console::style;
use futures::AsyncWriteExt;
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
		Self { t: self.t.clone(), v: self.v }
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

	( String::from_utf8(ret).unwrap() , 0 )
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

fn pre_handle_path (path : &String , cwd : &String) -> String{

	let mut cdpath : String;

	let is_windows = cwd.as_bytes()[0] != b'/';

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

			if cdpath.is_empty() {
				cdpath = "/".to_string();
			}
		}
		
	} else {

		loop {

			if is_windows {
				if path.len() > 1 && path.as_bytes()[1] == b':' {
    						cdpath = path.clone();
    						break;
    					}

				if cwd.len() == 3 {
					cdpath = cwd.clone() + &path.clone();
				} else {
					cdpath = [cwd.clone() , path.clone()].join("\\");
				}
			} else {
				if path.as_bytes()[0] == b'/'{
					cdpath = path.clone();
					break;
				}
				if cwd == "/" {
					cdpath = cwd.clone() + &path.clone();
				} else {
					cdpath = [cwd.clone() , path.clone()].join("/");
				}
			}
			break;
		}
	}

	cdpath
}

pub async fn handle_cmd(spawn : SpawnClient){

	let mut term = Term::stdout();
	let mut client = match spawn.spawn().await{
		Ok(p) => p,
		Err(_) => {
            println_err!("connect faild!");
            return;
        } ,
	};
	let mut cwd = match client.cwd().await{
		Ok(p) => p,
		Err(e) => {
			println_err!("error code : {}" , error_retcode(e));
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
				println_err!("command parser faild");
				continue;
			},
		};

		if cmd.is_empty(){
			continue;
		}

		if cmd[0] == "help" {
			println!("-------------------------- yaftp client command list ------------------------------");
			println!("help                          - command usage information");
			println!("ls                            - list of current work directory");
			println!("cd    [path]                  - change current work directory");
			println!("cp    [srcpath] [targetpath]  - copy file to target path");
			println!("mv    [srcpath] [targetpath]  - move file to target path");
			println!("rm    [path]                  - remove a file or folder");
			println!("hash  [path]                  - calcatute a file hash");
			println!("mkdir [path]                  - make directory");
			println!("get   [remote path]           - download a file to local");
			println!("put   [local path]            - upload a file to remote work directory");
			println!("cat   [remote path]           - read a file and print content");
			println!("-----------------------------------------------------------------------------------");
			continue;
		}

		if cmd[0] == "ls" {

			if cmd.len() != 1{
				println_err!("command 'ls' not any argument . eg : ls");
				continue;
			}

			let mut client = match spawn.spawn().await{
				Ok(p) => p,
				Err(e) => {
					println_err!("connect error : {}", e);
					continue;
				},
			};
			let result = match client.ls(&cwd).await{
				Ok(p) => p,
				Err(_) => {
					println_err!("command execute faild");
					continue;
				},
			};

			let mut files : Vec<FileInfo> = vec![];

			for i in result {
				let col : Vec<&str> = i.split('|').collect();
				files.push(FileInfo{name : col[0].to_string() , typ : col[1].to_string() , size : col[2].to_string() , modified : col[3].to_string() , accesstime : col[4].to_string()});
			}

			let table = Table::new(files).to_string();
			print!("{}",table);
		}
		
		if cmd[0] == "cd" {
			if cmd.len() != 2{
				println_err!("command 'cd' need 1 argument . eg : cd /var");
				continue;
			}

			if cmd[1] == "." {
				continue;
			}

			let cdpath = pre_handle_path(&cmd[1], &cwd);

			if cdpath.is_empty(){
				continue;
			}

			let mut client = match spawn.spawn().await{
				Ok(p) => p,
				Err(e) => {
					println_err!("connect error : {}", e);
					continue;
				},
			};

			let (ret, path) = match client.info(&cdpath).await{
				Ok(p) => p,
				Err(e) => {
                    println_err!("get cd target path information faild : {}" , e);
					continue;
				},
			};

			if ret[0] == 0 {
				cwd = path;
			} else {
				println_err!("'{}' not path" , path);
			}

		}

		if cmd[0] == "cp" {
			if cmd.len() != 3{
				println!("command 'cp' need 2 argument . eg : cp /var/file1 /var/file2");
				continue;
			}

			let srcpath = pre_handle_path(&cmd[1], &cwd);

			if srcpath.is_empty(){
				continue;
			}

			let targetpath = pre_handle_path(&cmd[2], &cwd);

			if targetpath.is_empty(){
				continue;
			}

			let mut client = match spawn.spawn().await{
				Ok(p) => p,
				Err(e) => {
					println_err!("connect error : {}", e);
					continue;
				},
			};

			let (ret, _) = match client.info(&srcpath).await{
				Ok(p) => p,
				Err(e) => {
                    println_err!("read source path infomation faild : {}" , e);
					continue;
				},
			};
			
			if ret[0] != 1 {
				println_err!("'{}' not file" , srcpath);
				continue;
			}

			let mut client = match spawn.spawn().await{
				Ok(p) => p,
				Err(e) => {
					println_err!("connect error : {}", e);
					continue;
				},
			};

			let _ = match client.cp(&srcpath , &targetpath).await{
				Ok(_) => {
					println_err!("copy file '{}' to '{}' success" , srcpath , targetpath);
				},
				Err(_) => {
					continue;
				},
			};

		}

		if cmd[0] == "mkdir" {
			if cmd.len() != 2{
				println_err!("command 'mkdir' need 1 argument . eg : mkdir /var/folder1/folder2");
				continue;
			}

			let path = pre_handle_path(&cmd[1], &cwd);

			if path.is_empty(){
				continue;
			}

			let mut client = match spawn.spawn().await{
				Ok(p) => p,
				Err(e) => {
					println_err!("connect error : {}", e);
					continue;
				},
			};

			let _ = match client.mkd(&path).await{
				Ok(_) => {
					println_info!("mkdir '{}' success" , path);
				},
				Err(_) => {
					continue;
				},
			};

		}

		if cmd[0] == "mv" {
			if cmd.len() != 3{
				println_err!("command 'mv' need 2 argument . eg : mv /var/file1 /var/file2");
				continue;
			}

			let srcpath = pre_handle_path(&cmd[1], &cwd);

			if srcpath.is_empty(){
                println_err!("command error , please check argument format");
				continue;
			}

			let targetpath = pre_handle_path(&cmd[2], &cwd);

			if targetpath.is_empty(){
                println_err!("command error , please check argument format");
				continue;
			}

			let mut client = match spawn.spawn().await{
				Ok(p) => p,
				Err(e) => {
					println_err!("connect error : {}", e);
					continue;
				},
			};

			let (ret, _) = match client.info(&srcpath).await{
				Ok(p) => p,
				Err(e) => {
                    println_err!("read source path infomation faild : {}" , e);
					continue;
				},
			};
			
			if ret[0] != 1 {
				println_err!("'{}' not a file" , srcpath);
				continue;
			}

			let mut client = match spawn.spawn().await{
				Ok(p) => p,
				Err(e) => {
					println_err!("connect error : {}", e);
					continue;
				},
			};

			let _ = match client.mv(&srcpath , &targetpath).await{
				Ok(_) => {
					println_info!("move file '{}' to '{}' success" , srcpath , targetpath);
				},
				Err(_) => {
					continue;
				},
			};
		}
		
		if cmd[0] == "rm" {
			if cmd.len() != 2{
				println_err!("command 'rm' need 1 argument . eg : rm /var/folder1/file2");
				continue;
			}

			let path = pre_handle_path(&cmd[1], &cwd);

			if path.is_empty(){
				continue;
			}

			let mut client = match spawn.spawn().await{
				Ok(p) => p,
				Err(e) => {
					println_err!("connect error : {}", e);
					continue;
				},
			};

			let _ = match client.rm(&path).await{
				Ok(_) => {
					println_info!("remove '{}' success" , path);
				},
				Err(_) => {
					continue;
				},
			};
		}

		if cmd[0] == "hash" {
			if cmd.len() != 2{
				println_err!("command 'hash' need 1 argument . eg : hash /var/folder1/file2");
				continue;
			}

			let path = pre_handle_path(&cmd[1], &cwd);

			if path.is_empty(){
				continue;
			}

			let mut client = match spawn.spawn().await{
				Ok(p) => p,
				Err(e) => {
					println_err!("connect error : {}", e);
					continue;
				},
			};

			let (info, abspath) = match client.info(&path).await{
				Ok(p) => p,
				Err(e) => {
                    println_err!("read file information faild : {}" , e);
					continue;
				},
			};

			if info[0] != 1 {
				println_err!("'{}' not a file" , abspath);
				continue;
			}

			let mut client = match spawn.spawn().await{
				Ok(p) => p,
				Err(e) => {
					println_err!("connect error : {}", e);
					continue;
				},
			};

			match client.hash(&path , info[1]).await{
				Ok(p) => {
					println_info!("{}", p);
				},
				Err(_) => {
					continue;
				},
			};

		}

		if cmd[0] == "cat" {
			if cmd.len() != 2{
				println_err!("command 'cat' need 1 argument . eg : cat /var/folder1/file2");
				continue;
			}

			let path = pre_handle_path(&cmd[1], &cwd);

			if path.is_empty(){
				continue;
			}

			let mut client = match spawn.spawn().await{
				Ok(p) => p,
				Err(e) => {
					println_err!("connect error : {}", e);
					continue;
				},
			};

			let (info, abspath) = match client.info(&path).await{
				Ok(p) => p,
				Err(e) => {
                    println_err!("read file infomation faild : {}" , e);
					continue;
				},
			};

			if info[0] != 1 {
				println_err!("'{}' not a file" , abspath);
				continue;
			}

			if info[1] > 1024 * 100 {
				println_err!("'{}' file size > 100kb , command cat cant read" , abspath);
				continue;
			}

			let mut client = match spawn.spawn().await{
				Ok(p) => p,
				Err(e) => {
					println_err!("connect error : {}", e);
					continue;
				},
			};

			match client.cat(&path).await{
				Ok(p) => {
					println!("{}" , p);
				},
				Err(_) => {
					continue;
				},
			};
		}

		if cmd[0] == "get" {
			if cmd.len() != 2{
				println_err!("command 'get' need 1 argument . eg : get /var/folder1/file2");
				continue;
			}

			let path = pre_handle_path(&cmd[1], &cwd);

			if path.is_empty(){
				continue;
			}

			let mut client = match spawn.spawn().await{
				Ok(p) => p,
				Err(e) => {
					println_err!("connect error : {}", e);
					continue;
				},
			};

			let (info, abspath) = match client.info(&path).await{
				Ok(p) => p,
				Err(e) => {
                    println_err!("read file infomation faild : {}" , e);
					continue;
				},
			};

			if info[0] != 1 {
				println_err!("'{}' not a file" , abspath);
				continue;
			}

			let filename : String;

			if path.as_bytes()[0] == b'/' {
				filename = path.split_at(path.rfind('/').unwrap() + 1).1.to_string();
			} else {
				filename = path.split_at(path.rfind('\\').unwrap() + 1).1.to_string();
			}

			match fs::metadata(filename.clone()).await{
				Ok(p) => {
					println_info!("local file '{}' already exists" , filename);
					let remote_file_size = info[1];
					let local_file_size = p.len();

					if local_file_size >= remote_file_size{
						println_err!("local file and remote file size equal or bigger. please remove local file before execute command put.");
						continue;
					}

					let mut client = match spawn.spawn().await{
						Ok(p) => p,
						Err(e) => {
							println_err!("connect error : {}", e);
							continue;
						},
					};

					match client.hash(&path, p.len()).await{
						Ok(p) => {
							
							println_info!("remote file hash : {}" , p);
							
							let mut f = match fs::File::open(&filename).await{
								Ok(p) => p,
								Err(e) => {
									println_err!("open local file faild : {}" , e);
									continue;
								},
							};
					
							let md5_str = calc_md5(&mut f , local_file_size).await;

							match f.close().await{
								Ok(_) => {},
								Err(e) => {
									println_err!("close local file faild : {}" , e);
									continue;
								},
							};

							if md5_str.is_empty() {
								println_err!("calc local file hash faild !");
								continue;
							}

							println_info!("local file hash : {}" , md5_str);

							if md5_str != p{
								println_err!("remote file and local file hash not equal. please remove remote file before execute command put.");
								continue;
							}

							println_info!("start resume broken transfer");

							let mut client = match spawn.spawn().await{
								Ok(p) => p,
								Err(e) => {
									println_err!("connect error : {}", e);
									continue;
								},
							};
				
							match client.get(&filename , &path , local_file_size).await{
								Ok(_) => {
									println_info!("file transfer success!");
								},
								Err(_) => {
									continue;
								},
							};
							continue;
						},
						Err(e) => {
							println_info!("calc remote file hash faild : {} ", e);
							continue;
						},
					}
				},
				Err(_) => {
					println_info!("start file transfer");
				},
			};

			let mut client = match spawn.spawn().await{
				Ok(p) => p,
				Err(e) => {
					println_err!("connect error : {}", e);
					continue;
				},
			};

			match client.get(&filename , &path , 0).await{
				Ok(_) => {
                    println_info!("file transfer success!");
                },
				Err(_) => {
					continue;
				},
			};

		}

		if cmd[0] == "put" {
			if cmd.len() != 2{
				println_err!("command 'put' need 1 argument . eg : put /localfile/file1");
				continue;
			}

			let localpath = cmd[1].clone();

			let filename : String;

			if cmd[1].as_bytes()[0] == b'/' {
				let pos = match localpath.rfind('/'){
					Some(p) => {
						p
					},
					None => {
						println_err!("command 'put' need 1 argument . eg : put /localfile/file1");
						continue;
					}
				};
				filename = localpath.split_at(pos + 1).1.to_string();
			} else {
				let pos = match localpath.rfind('\\'){
					Some(p) => {
						p
					},
					None => {
						println_err!("command 'put' need 1 argument . eg : put /localfile/file1");
						continue;
					}
				};
				filename = localpath.split_at(pos + 1).1.to_string();
			}

			let remotepath = pre_handle_path(&filename, &cwd);

			if localpath.is_empty(){
				continue;
			}

			let mut client = match spawn.spawn().await{
				Ok(p) => p,
				Err(e) => {
					println_err!("connect error : {}", e);
					continue;
				},
			};

			match client.info(&remotepath).await{
				Ok(p) => {
					println_info!("remote file '{}' already exists" , p.1);
					let size = p.0[1];

					let mut client = match spawn.spawn().await{
						Ok(p) => p,
						Err(e) => {
							println_err!("connect error : {}", e);
							continue;
						},
					};

					match client.hash(&p.1, size).await{
						Ok(p) => {
							println_info!("remote file hash : {}" , p);

							let mut f = match fs::File::open(&localpath).await{
								Ok(p) => p,
								Err(e) => {
									println_err!("open local file faild : {}" , e);
									continue;
								},
							};
					
							if size >= f.metadata().await.unwrap().len(){
								println_err!("remote file and local file size equal or bigger. please remove remote file before execute command put.");
								continue;
							}
					
							let md5_str = calc_md5(&mut f , size).await;

							match f.close().await{
								Ok(_) => {},
								Err(e) => {
									println_err!("close local file faild : {}" , e);
									continue;
								},
							};

							if md5_str.is_empty() {
								println_err!("calc local file hash faild !");
								continue;
							}

							println_info!("local file hash : {}" , md5_str);

							if md5_str != p{
								println_err!("remote file and local file hash not equal. please remove remote file before execute command put.");
								continue;
							}
						},
						Err(e) => {
							println_err!("calc remote file hash faild : {}" , e );
							continue;
						},
					};

					println_info!("start resume broken transfer!");

					let mut client = match spawn.spawn().await{
						Ok(p) => p,
						Err(e) => {
							println_err!("connect error : {}", e);
							continue;
						},
					};

					match client.put(&localpath , &remotepath , size).await{
						Ok(_) => {
							println_info!("file transfer success!");
						},
						Err(_) => {
							continue;
						},
					};
					continue;
				},
				Err(_) => {},
			};

			let mut client = match spawn.spawn().await{
				Ok(p) => p,
				Err(e) => {
					println_err!("connect error : {}", e);
					continue;
				},
			};

			match client.put(&localpath , &remotepath , 0).await{
				Ok(_) => {
                    println_info!("file transfer success!");
                },
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