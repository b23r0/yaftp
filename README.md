# yaftp
Yet Another File Transfer Protocol.

# Protocol

## Features

* C2C
* Lightweight
* Per something per session
* High performence
* Resume broken transfer

## Handshake Request

```
+----+----------+----------+
|VER | NMETHODS | METHODS  |
+----+----------+----------+
| 1  |    1     | 1 to 255 |
+----+----------+----------+
```

fisrt , client will send client version and support methods . 

In yaftp version 1.0 , only support 6 methods.

```
+-----+-----------+
| CMD |   VALUE   |
+-----+-----------+
| ls  |   0x01    |
+-----+-----------+
| cp  |   0x02    |
+-----+-----------+
| mv  |   0x03    |
+-----+-----------+
| rm  |   0x04    |
+-----+-----------+
| put |   0x05    |
+-----+-----------+
| get |   0x06    |
+-----+-----------+
```

## Handshake Reply

```
+----+----------+----------+
|VER | NMETHODS | METHODS  |
+----+----------+----------+
| 1  |    1     | 1 to 255 |
+----+----------+----------+
```

yaftp will reply server version and support methods.

## Send Command

```
+-------+-------+
|CMD    | NARG  |
+-------+-------+
| 1(u8) | 1(u8) |
+-------+-------+
```

client send command message to server , tell server command type , arguments count , and next argument size.

if command has two arguments and above,  client will keep send argument message until last one.

```
+-----------------+---------------------+
| NEXT_ARG_SIZE   |         ARG         |
+-----------------+---------------------+
|      4(u32)     |       Variable      |
+-----------------+---------------------+
```

next , we need know every command argument and type.

## Command Arguments

```
|---------+------+-----------------------+-----------------------+-----------------------+-----------------------+-----------------------|
| Command | NArg | Arg1                  | Arg2                  | Arg3                  | Arg4                  | Arg5                  |
|---------|------|-----------------------|-----------------------|-----------------------|-----------------------+-----------------------|
| ls      | 1    | path [string]         |                       |                       |                       |                       |
| cp      | 2    | source path [string]  | target path [string]  |                       |                       |                       |
| mv      | 2    | source path [string]  | target path [string]  |                       |                       |                       |
| rm      | 1    | path [string]         |                       |                       |                       |                       |
| rm      | 1    | path [string]         |                       |                       |                       |                       |
| put     | 4    | path [string]         | md5[u32]              | start_pos[u128]       | end_pos[u128]         | data[stream]          |
| get     | 4    | path [string]         | start_pos[u128]       | end_pos[u128]         |                       |                       |
|---------+------+-----------------------+-----------------------+-----------------------+-----------------------+-----------------------|
```

## Command Reply

server received command arguments will check if valid and reply a code and arguments count.

```
+-----------+-----------+
|  RETCODE  |  NARG     |
+-----------+-----------+
|  1(u8)    |  1(u8)    |
+-------- --+-----------+
```

if check vaild return 0x00 , else return 1~255.

```
+-----------+-----------------------------+
|  RETCODE  |  Reason                     |
+-----------+-----------------------------+
|  1        |  No permission              |
+-------- --+-----------------------------+
|  2        |  source path not found      |
+-------- --+-----------------------------+
|  3        |  start pos unvalid          |
+-------- --+-----------------------------+
|  4        |  end pos unvalid            |
+-------- --+-----------------------------+
|  5        |  check hash faild           |
+-------- --+-----------------------------+
```

Note : The yaftp protocol is full-duplex, so depending on the command, the returned data may not be returned until the command parameters are completely sent. Therefore, the returned data needs to be processed asynchronously. For example: after the put method submits the first three parameters, it may directly return a non-zero retcode. 

## Command Reply Arguments

command reply arguments same with command arguments.

```
+-----------------+---------------------+
| NEXT_ARG_SIZE   |         ARG         |
+-----------------+---------------------+
|      4(u32)     |       Variable      |
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

### cp - 0x02

```
|---------+------+
| Command | NArg |
|---------|------|
| ls      | 0    |
|---------+------+
```

command `cp` just return a code tell client if success.

### mv - 0x03

```
|---------+------+
| Command | NArg |
|---------|------|
| mv      | 0    |
|---------+------+
```

command `mv` just return a code tell client if success.

### rm - 0x04

```
|---------+------+
| Command | NArg |
|---------|------|
| rm      | 0    |
|---------+------+
```

command `rm` just return a code tell client if success.

### put - 0x05

```
|---------+-----------+-----------------------+
| Command | NArg      | Arg1                  |
|---------|-----------|-----------------------+
| put     | 0 or 1    | md5(u32)              |
|---------+-----------+-----------------------+
```

command `put` if retcode eq 0 will return a md5 hash after transfer finished.

### get - 0x06

```
|---------+-----------+-----------------------+-----------------------|
| Command | NArg      | Arg1                  | Arg2                  |
|---------|-----------|-----------------------+-----------------------|
| get     | 0 or 2    | md5(u32)              | data[stream]          |
|---------+-----------+-----------------------+-----------------------|
```

command `get` if retcode eq 0 will return request file data md5 hash , then server will send client request data.

### Finally

Server will close the session connection.


