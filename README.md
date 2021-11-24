# yaftp [![Build Status](https://img.shields.io/github/workflow/status/b23r0/yaftp/Rust)](https://github.com/b23r0/yaftp/actions/workflows/rust.yml) [![ChatOnDiscord](https://img.shields.io/badge/chat-on%20discord-blue)](https://discord.gg/ZKtYMvDFN4) [![Crate](https://img.shields.io/crates/v/yaftp)](https://crates.io/crates/yaftp)
Yet another File Transfer Protocol implementation by Rust.

Support with resume broken transfer & reverse mode & largefile.

# Features

* C2C
* Lightweight
* Per something per session
* Support large file
* Support Resume broken transfer
* Support reverse mode(cross firewall)

# Build & Run

`$> cargo build --release`

# Installation

`$> cargo install yaftp`

# Usage

## Bind Mode

You can run a yaftp server and listen port at 8000

`$> ./yaftp -l 8000`

then connect to server and get a shell

`$> ./yaftp -c 127.0.0.1 8000`

## Reverse Mode

First listen a port waiting for slave connected and get shell

`$> ./rsocx -t 8000`

then reverse connect to master in slave

`$> ./rsocx -r 127.0.0.1 8000`

# Protocol(v1)

## Data Type

```
+--------+-------------------+------------------+
| TYPE   |      EXTYPE       | LENGTH(bytes)    |
+--------+-------------------+------------------+
| string |      utf-8        |    Variable      |
+--------+-------------------+------------------+
| path   |      utf-8        |    < 1024        |
+--------+-------------------+------------------+
| u8     |  be_uchar_8bit    |       1          |
+--------+-------------------+------------------+
| u16    |  be_uword_16bit   |       2          |
+--------+-------------------+------------------+
| u32    |  be_ulong_32bit   |       4          |
+--------+-------------------+------------------+
| u64    |   be_ull_64bit    |       8          |
+--------+-------------------+------------------+
```

## Handshake Request

```
+-------+----------+---------------+
|  VER  | NMETHODS | METHODS       |
+-------+----------+---------------+
| 1(u8) |   1(u8)  | 1 to 255 (u8) |
+-------+----------+---------------+
```

fisrt , client will send client version and support methods . 

In version 1.0 , only support 10 methods.

```
+------+-----------+
| CMD  |   VALUE   |
+------+-----------+
| ls   |   0x01    |
+------+-----------+
| cwd  |   0x02    |
+------+-----------+
| cp   |   0x03    |
+------+-----------+
| mkd  |   0x04    |
+------+-----------+
| mv   |   0x05    |
+------+-----------+
| rm   |   0x06    |
+------+-----------+
| put  |   0x07    |
+------+-----------+
| get  |   0x08    |
+------+-----------+
| info |   0x09    |
+------+-----------+
| hash |   0x0a    |
+------+-----------+
```

## Handshake Reply

```
+-------+----------+---------------+
|  VER  | NMETHODS | METHODS       |
+-------+----------+---------------+
| 1(u8) |   1(u8)  | 1 to 255 (u8) |
+-------+----------+---------------+
```

client will reply to server version and support methods.

## Command Request

```
+-------+--------+
|  CMD  | NARG   |
+-------+--------+
| 1(u8) | 4(u32) |
+-------+--------+
```

client send command message to server , tell server command type , arguments count , and next argument size.

if command has two arguments and above,  client will keep send argument message until last one.

```
+-----------------+---------------------+
| NEXT_ARG_SIZE   |         ARG         |
+-----------------+---------------------+
|      8(u64)     |       Variable      |
+-----------------+---------------------+
```

next , we need know every command argument and type.

## Command Arguments

```
+---------+------+---------------------------------+-----------------------+-----------------------+
| Command | NArg | Arg1                            | Arg2                  | Arg3                  |
+---------+------+---------------------------------+-----------------------+-----------------------+
| ls      | 1    | path [string](max 1024)         |                       |                       |
| cwd     | 0    |                                 |                       |                       |
| cp      | 2    | source path [string]            | target path [string]  |                       |
| mkd     | 1    | path [string]                   |                       |                       |
| mv      | 2    | source path [string]            | target path [string]  |                       |
| rm      | 1    | path [string]                   |                       |                       |
| put     | 4    | path [string]                   | start_pos[u64]        | data[stream]          |
| get     | 4    | path [string]                   | start_pos[u64]        |                       |
| info    | 1    | path [string](max 1024)         |                       |                       |
| hash    | 1    | path [string](max 1024)         | end_pos[u64]          |                       |
+---------+------+---------------------------------+-----------------------+-----------------------+
```

## Command Reply

server received command arguments will check if valid and reply a code and arguments count.

```
+-----------+-----------+
|  RETCODE  |  NARG     |
+-----------+-----------+
|  1(u8)    |  4(u32)   |
+-----------+-----------+
```

if check vaild return 0x00 , else return 1~255.

```
+-----------+-----------------------------+
|  RETCODE  |  Reason                     |
+-----------+-----------------------------+
|  1        |  NoSupportVersion           |
+-----------+-----------------------------+
|  2        |  NoSupportCommand           |
+-----------+-----------------------------+
|  3        |  NoPermission               |
+-----------+-----------------------------+
|  4        |  NotFound                   |
+-----------+-----------------------------+
|  5        |  StartPosError              |
+-----------+-----------------------------+
|  6        |  EndPosError                |
+-----------+-----------------------------+
|  7        |  ArgumentSizeError          |
+-----------+-----------------------------+
|  8        |  ArgumentError              |
+-----------+-----------------------------+
|  9        |  ArgumentCountError         |
+-----------+-----------------------------+
|  10       |  ReadFolderFaild            |
+-----------+-----------------------------+
|  11       |  ReadCwdFaild               |
+-----------+-----------------------------+
|  12       |  UTF8FormatError            |
+-----------+-----------------------------+
|  13       |  ReadFileError              |
+-----------+-----------------------------+
|  14       |  WriteFileError             |
+-----------+-----------------------------+
|  15       |  CalcMd5Error               |
+-----------+-----------------------------+
|  16       |  UnknownNetwordError        |
+-----------+-----------------------------+
|  17       |  UnknownError               |
+-----------+-----------------------------+
```

Note : The yaftp protocol is full-duplex, so depending on the command, the returned data may not be returned until the command parameters are completely sent. Therefore, the returned data needs to be processed asynchronously. 

## Command Reply Format

command reply same with command request .

```
+-----------------+---------------------+
| NEXT_ARG_SIZE   |         ARG         |
+-----------------+---------------------+
|      8(u64)     |       Variable      |
+-----------------+---------------------+
```

### ls - 0x01

```
+---------+-----------+-----------------------+
| Command | NArg      |  ArgN                 |
+---------+-----------+-----------------------+
| ls      | 0 or N    | row(string)           |
+---------+-----------+-----------------------+
```

command `ls` will return a table. every row split by `|`.

### cwd - 0x02

```
+---------+-----------+-----------------------+
| Command | NArg      | Arg1                  |
+---------+-----------+-----------------------+
| cwd     | 0 or 1    | path(string)          |
+---------+-----------+-----------------------+
```

command `cwd` return server current work directory.

### cp - 0x03

```
+---------+------+
| Command | NArg |
+---------|------|
| cp      | 0    |
+---------+------+
```

command `cp` return a code tell client if success.

### mkd - 0x04

```
+---------+------+
| Command | NArg |
+---------+------+
| mkd     | 0    |
+---------+------+
```

command `mkd` return a code tell client if success.

### mv - 0x05

```
+---------+------+
| Command | NArg |
+---------+------+
| mv      | 0    |
+---------+------+
```

command `mv` return a code tell client if success.

### rm - 0x06

```
+---------+------+
| Command | NArg |
+---------+------+
| rm      | 0    |
+---------+------+
```

command `rm` return a code tell client if success.

### put - 0x07

```
+---------+-----------+
| Command | NArg      |
+---------+-----------+
| put     | 0         |
+---------+-----------+
```

command `put` just return a code tell client if success.

### get - 0x08

```
+---------+-----------+-----------------------+
| Command | NArg      | Arg1                  |
+---------+-----------+-----------------------+
| get     | 0 or 1    | data(stream)          |
+---------+-----------+-----------------------+
```

command `get` if retcode eq 0 will send client request file data.

### info - 0x09

```
+---------+-----------+-----------------------+-----------------------+-----------------------+-----------------------+-----------------------+
| Command | NArg      | Arg1                  | Arg2                  | Arg3                  | Arg4                  | Arg5                  |
+---------+-----------+-----------------------+-----------------------+-----------------------+-----------------------+-----------------------+
| info    | 0 or 5    | u8                    | u64                   | u64                   | u64                   | path(string)          |
+---------+-----------+-----------------------+-----------------------+-----------------------+-----------------------+-----------------------+
```

command `info` if retcode eq 0 will return arg1 (filetype : 0 is folder , 1 is file , other is others) , arg2(filesize) , arg3 (file last modify timestamp) , arg4 (file last accessed timestamp) , arg5 (absolute path).

### hash - 0x0a

```
+---------+-----------+-----------------------+
| Command | NArg      | Arg1                  |
+---------+-----------+-----------------------+
| hash    | 0 or 1    | md5_32(string)        |
+---------+-----------+-----------------------+
```

command `hash` if retcode eq 0 will return request file data md5 hash.

## Finally

Server will close the connection session.
