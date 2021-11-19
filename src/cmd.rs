use std::fmt::Debug;
use std::fmt;
use std::error;

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

pub fn cmd(input : String) -> Result<Vec<String> , CmdError>{

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

#[test]
fn test_cmd() {
    let ret = cmd("ls \"test\" 123".to_string()).unwrap();
    assert_eq!(ret[0] , String::from("ls"));
    assert_eq!(ret[1] , String::from("test"));
    assert_eq!(ret[2] , String::from("123"));

    let ret = cmd("ls \"test\"".to_string()).unwrap();

    assert_eq!(ret[0] , String::from("ls"));
    assert_eq!(ret[1] , String::from("test"));
}