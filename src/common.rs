use std::{error, fmt::{self, Debug}};

pub enum YaftpError {
    OK,
    NoSupportVersion,
    NoSupportCommand,
    NoPermission, 
    NotFound, 
    StartPosUnvalid,
    EndPosUnvalid,
    CheckHashFaild,
    ArgumentUnvalid,
    ReadFolderFaild,
    ReadCwdFaild,
    UnknownNetwordError,
    UnknownError
}

impl fmt::Display for YaftpError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::OK => write!(f, "OK"),
            Self::NoSupportVersion => write!(f, "NO_SUPPORT_VERSION"),
            Self::NoSupportCommand => write!(f, "NO_SUPPORT_COMMAND"),
            Self::NoPermission => write!(f, "NO_PERMISSION"),
            Self::NotFound => write!(f, "NOT_FOUND"),
            Self::StartPosUnvalid => write!(f, "START_POS_UNVALID"),
            Self::EndPosUnvalid => write!(f, "END_POS_UNVALID"),
            Self::CheckHashFaild => write!(f, "CHECK_HASH_FAILD"),
            Self::ArgumentUnvalid => write!(f, "ARGUMENT_UNVALID"),
            Self::ReadFolderFaild => write!(f, "READ_FOLDER_FAILD"),
            Self::ReadCwdFaild => write!(f, "READ_CWD_FAILD"),
            Self::UnknownNetwordError => write!(f, "UNKNOWN_NETWORD_ERROR"),
            Self::UnknownError => write!(f, "UNKNOWN_ERROR"),
        }
    }
}

impl error::Error for YaftpError {
    fn source(&self) -> Option<&(dyn error::Error + 'static)> {
        None
    }
}

impl Debug for YaftpError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::OK => write!(f, "OK"),
            Self::NoSupportVersion => write!(f, "NO_SUPPORT_VERSION"),
            Self::NoSupportCommand => write!(f, "NO_SUPPORT_COMMAND"),
            Self::NoPermission => write!(f, "NO_PERMISSION"),
            Self::NotFound => write!(f, "NOT_FOUND"),
            Self::StartPosUnvalid => write!(f, "START_POS_UNVALID"),
            Self::EndPosUnvalid => write!(f, "END_POS_UNVALID"),
            Self::CheckHashFaild => write!(f, "CHECK_HASH_FAILD"),
            Self::ArgumentUnvalid => write!(f, "ARGUMENT_UNVALID"),
            Self::ReadFolderFaild => write!(f, "READ_FOLDER_FAILD"),
            Self::ReadCwdFaild => write!(f, "READ_CWD_FAILD"),
            Self::UnknownNetwordError => write!(f, "UNKNOWN_NETWORD_ERROR"),
            Self::UnknownError => write!(f, "UNKNOWN_ERROR"),
        }
    }
}

pub fn retcode_error(retcode : u8) -> YaftpError {
    match retcode {
        0x01 => YaftpError::NoSupportVersion,
        0x02 => YaftpError::NoSupportCommand,
        0x03 => YaftpError::NoPermission,
        0x04 => YaftpError::NotFound,
        0x05 => YaftpError::StartPosUnvalid,
        0x06 => YaftpError::EndPosUnvalid,
        0x07 => YaftpError::CheckHashFaild,
        0x08 => YaftpError::ArgumentUnvalid,
        0x09 => YaftpError::ReadFolderFaild,
        0x0a => YaftpError::ReadCwdFaild,
        0x0b => YaftpError::UnknownNetwordError,
        _ => YaftpError::UnknownError
    }
}

pub fn error_retcode(code : YaftpError) -> u8 {
    match code {
        YaftpError::OK => 0x00,
        YaftpError::NoSupportVersion => 0x01,
        YaftpError::NoSupportCommand => 0x02,
        YaftpError::NoPermission => 0x03,
        YaftpError::NotFound => 0x04,
        YaftpError::StartPosUnvalid => 0x05,
        YaftpError::EndPosUnvalid => 0x06,
        YaftpError::CheckHashFaild => 0x07,
        YaftpError::ArgumentUnvalid => 0x08,
        YaftpError::ReadFolderFaild => 0x09,
        YaftpError::ReadCwdFaild => 0x0a,
        YaftpError::UnknownNetwordError => 0x0b,
        _ => 0xff
    }
}