# yaftp
Yet Another File Transfer Protocol.

# Build & Run

`$> cargo build --release`

# Features

* C2C
* Lightweight
* Per something per session
* High performence
* Resume broken transfer

# Protocol

## Handshake Request

```
+-------+----------+---------------+
|  VER  | NMETHODS | METHODS       |
+-------+----------+---------------+
| 1(u8) |   1(u8)  | 1 to 255 (u8) |
+-------+----------+---------------+
```

fisrt , client will send client version and support methods . 

In yaftp version 1.0 , only support 8 methods.

```
+-----+-----------+
| CMD |   VALUE   |
+-----+-----------+
| ls  |   0x01    |
+-----+-----------+
| cwd |   0x02    |
+-----+-----------+
| cp  |   0x03    |
+-----+-----------+
| mkd |   0x04    |
+-----+-----------+
| mv  |   0x05    |
+-----+-----------+
| rm  |   0x06    |
+-----+-----------+
| put |   0x07    |
+-----+-----------+
| get |   0x08    |
+-----+-----------+
```

## Handshake Reply

```
+-------+----------+---------------+
|  VER  | NMETHODS | METHODS       |
+-------+----------+---------------+
| 1(u8) |   1(u8)  | 1 to 255 (u8) |
+-------+----------+---------------+
```

yaftp will reply server version and support methods.

## Send Command

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
|      4(u64)     |       Variable      |
+-----------------+---------------------+
```

next , we need know every command argument and type.

## Command Arguments

```
|---------+------+---------------------------------+-----------------------+-----------------------+-----------------------+-----------------------|
| Command | NArg | Arg1                            | Arg2                  | Arg3                  | Arg4                  | Arg5                  |
|---------|------|---------------------------------|-----------------------|-----------------------|-----------------------+-----------------------|
| ls      | 1    | path [string](max 1024)         |                       |                       |                       |                       |
| cwd     | 0    |                                 |                       |                       |                       |                       |
| cp      | 2    | source path [string]            | target path [string]  |                       |                       |                       |
| mkd     | 1    | path [string]                   |                       |                       |                       |                       |
| mv      | 2    | source path [string]            | target path [string]  |                       |                       |                       |
| rm      | 1    | path [string]                   |                       |                       |                       |                       |
| rm      | 1    | path [string]                   |                       |                       |                       |                       |
| put     | 4    | path [string]                   | md5[u32]              | start_pos[u64]        | end_pos[u64]          | data[stream]          |
| get     | 4    | path [string]                   | start_pos[u64]        | end_pos[u64]          |                       |                       |
|---------+------+---------------------------------+-----------------------+-----------------------+-----------------------+-----------------------|
```

Note : all path max size < 1024 bytes. you need check it.

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
|  1        |  not support the version    |
+-----------+-----------------------------+
|  2        |  not support the command    |
+-----------+-----------------------------+
|  3        |  No permission              |
+-----------+-----------------------------+
|  4        |  source path not found      |
+-----------+-----------------------------+
|  5        |  start pos unvalid          |
+-----------+-----------------------------+
|  6        |  end pos unvalid            |
+-----------+-----------------------------+
|  7        |  check hash faild           |
+-----------+-----------------------------+
|  8        |  argument count unvaild     |
+-----------+-----------------------------+
|  9        |  argument unvaild           |
+-----------+-----------------------------+
|  10       |  argument size unvaild      |
+-----------+-----------------------------+
|  11       |  read folder faild          |
+-----------+-----------------------------+
|  12       |  argument size unvaild      |
+-----------+-----------------------------+
```

Note : The yaftp protocol is full-duplex, so depending on the command, the returned data may not be returned until the command parameters are completely sent. Therefore, the returned data needs to be processed asynchronously. For example: after the put method submits the first three parameters, it may directly return a non-zero retcode. 

## Command Reply Arguments

command reply arguments same with command arguments.

```
+-----------------+---------------------+
| NEXT_ARG_SIZE   |         ARG         |
+-----------------+---------------------+
|      4(u64)     |       Variable      |
+-----------------+---------------------+
```

### ls - 0x01

```
|---------+-----------+-----------------------+-----------------------+
| Command | NArg      | Arg1                  |  ArgN                 |
|---------|-----------|-----------------------+-----------------------+
| ls      | 0 or N    | column(string)        | row(string)           |
|---------+-----------+-----------------------+-----------------------+
```

command `ls` will return a table like ls list. First argument is columns use `|` split.

others arguments is rows(use '|' split).

### cwd - 0x02

```
|---------+-----------+-----------------------+
| Command | NArg      | Arg1                  |
|---------|-----------|-----------------------+
| cwd     | 0 or 1    | path(string)          |
|---------+-----------+-----------------------+
```

command `cwd` just return a code tell client if success.

### cp - 0x03

```
|---------+------+
| Command | NArg |
|---------|------|
| cp      | 0    |
|---------+------+
```

command `cp` just return a code tell client if success.

### mkd - 0x04

```
|---------+------+
| Command | NArg |
|---------|------|
| mkd     | 0    |
|---------+------+
```

command `mkd` just return a code tell client if success.

### mv - 0x05

```
|---------+------+
| Command | NArg |
|---------|------|
| mv      | 0    |
|---------+------+
```

command `mv` just return a code tell client if success.

### rm - 0x06

```
|---------+------+
| Command | NArg |
|---------|------|
| rm      | 0    |
|---------+------+
```

command `rm` just return a code tell client if success.

### put - 0x07

```
|---------+-----------+-----------------------+
| Command | NArg      | Arg1                  |
|---------|-----------|-----------------------+
| put     | 0 or 1    | md5(u32)              |
|---------+-----------+-----------------------+
```

command `put` if retcode eq 0 will return a md5 hash after transfer finished.

### get - 0x08

```
|---------+-----------+-----------------------+-----------------------|
| Command | NArg      | Arg1                  | Arg2                  |
|---------|-----------|-----------------------+-----------------------|
| get     | 0 or 2    | md5(u32)              | data[stream]          |
|---------+-----------+-----------------------+-----------------------|
```

command `get` if retcode eq 0 will return request file data md5 hash , then server will send client request data.

## Finally

Server will close the session connection.


