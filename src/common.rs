use std::{error, fmt::{self, Debug}};

pub enum YaftpError {
	OK,
	NoSupportVersion,
	NoSupportCommand,
	NoPermission, 
	NotFound, 
	StartPosError,
	EndPosError,
	ArgumentSizeError,
	ArgumentError,
	ArgumentCountError,
	ReadFolderFaild,
	ReadCwdFaild,
	UTF8FormatError,
	ReadFileError,
	WriteFileError,
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
			Self::StartPosError => write!(f, "START_POS_ERROR"),
			Self::EndPosError => write!(f, "END_POS_ERROR"),
			Self::ArgumentSizeError => write!(f, "CHECK_HASH_FAILD"),
			Self::ArgumentError => write!(f, "ARGUMENT_ERROR"),
			Self::ArgumentCountError => write!(f, "ARGUMENT_COUNT_ERROR"),
			Self::ReadFolderFaild => write!(f, "READ_FOLDER_FAILD"),
			Self::ReadCwdFaild => write!(f, "READ_CWD_FAILD"),
			Self::UTF8FormatError => write!(f, "UTF8_FORMAT_ERROR"),
			Self::ReadFileError => write!(f, "READ_FILE_ERROR"),
			Self::WriteFileError => write!(f, "WRITE_FILE_ERROR"),
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
			Self::StartPosError => write!(f, "START_POS_ERROR"),
			Self::EndPosError => write!(f, "END_POS_ERROR"),
			Self::ArgumentSizeError => write!(f, "CHECK_HASH_FAILD"),
			Self::ArgumentError => write!(f, "ARGUMENT_ERROR"),
			Self::ArgumentCountError => write!(f, "ARGUMENT_COUNT_ERROR"),
			Self::ReadFolderFaild => write!(f, "READ_FOLDER_FAILD"),
			Self::ReadCwdFaild => write!(f, "READ_CWD_FAILD"),
			Self::UTF8FormatError => write!(f, "UTF8_FORMAT_ERROR"),
			Self::ReadFileError => write!(f, "READ_FILE_ERROR"),
			Self::WriteFileError => write!(f, "WRITE_FILE_ERROR"),
			Self::UnknownNetwordError => write!(f, "UNKNOWN_NETWORD_ERROR"),
			Self::UnknownError => write!(f, "UNKNOWN_ERROR"),
		}
	}
}

pub fn retcode_error(retcode : u8) -> YaftpError {
	match retcode {
		0x00 => YaftpError::OK,
		0x01 => YaftpError::NoSupportVersion,
		0x02 => YaftpError::NoSupportCommand,
		0x03 => YaftpError::NoPermission,
		0x04 => YaftpError::NotFound,
		0x05 => YaftpError::StartPosError,
		0x06 => YaftpError::EndPosError,
		0x07 => YaftpError::ArgumentSizeError,
		0x08 => YaftpError::ArgumentError,
		0x09 => YaftpError::ArgumentCountError,
		0x0a => YaftpError::ReadFolderFaild,
		0x0b => YaftpError::ReadCwdFaild,
		0x0c => YaftpError::UTF8FormatError,
		0x0d => YaftpError::ReadFileError,
		0x0e => YaftpError::WriteFileError,
		0x0f => YaftpError::UnknownNetwordError,
		_ =>	YaftpError::UnknownError
	}
}

pub fn error_retcode(code : YaftpError) -> u8 {
	match code {
		YaftpError::OK => 0x00,
		YaftpError::NoSupportVersion => 0x01,
		YaftpError::NoSupportCommand => 0x02,
		YaftpError::NoPermission => 0x03,
		YaftpError::NotFound => 0x04,
		YaftpError::StartPosError => 0x05,
		YaftpError::EndPosError => 0x06,
		YaftpError::ArgumentSizeError => 0x07,
		YaftpError::ArgumentError => 0x08,
		YaftpError::ArgumentCountError => 0x09,
		YaftpError::ReadFolderFaild => 0x0a,
		YaftpError::ReadCwdFaild => 0x0b,
		YaftpError::UTF8FormatError => 0x0c,
		YaftpError::ReadFileError => 0x0d,
		YaftpError::WriteFileError => 0x0e,
		YaftpError::UnknownNetwordError => 0x0f,
		YaftpError::UnknownError => 0xff,
	}
}