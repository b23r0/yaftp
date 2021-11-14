# yaftp
Yet Another File Transfer Protocol.

# Protocol

## Features

* C2C
* Per something per session
* High performence
* Resume broken transfer

## Handshake Request

+----+----------+----------+
|VER | NMETHODS | METHODS  |
+----+----------+----------+
| 1  |    1     | 1 to 255 |
+----+----------+----------+

fisrt , client will send client software version and support methods . 

In yaftp version 1.0 , only support 5 methods.

* ls  0x01
* cp  0x02
* mv  0x03
* put 0x04
* get 0x05

## Handshake Reply

+----+----------+----------+
|VER | NMETHODS | METHODS  |
+----+----------+----------+
| 1  |    1     | 1 to 255 |
+----+----------+----------+

yaftp will reply server software version and support methods.

## Send Command

+----+------+-----------------+---------------------+
|CMD | NARG |  NEXT_ARG_SIZE  |         ARG         |
+----+------+-----------------+---------------------+
| 1  |  1   |       4(u32)    |       Variable      |
+----+------+-----------------+---------------------+

client send command message to server , tell server command type , argument count , and next argument size.

if command has two arguments and above,  client will keep send argument message until last one.

+-----------------+---------------------+
| NEXT_ARG_SIZE   |         ARG         |
+-----------------+---------------------+
|      4(u32)     |       Variable      |
+-----------------+---------------------+

next , we need know every command argument and type.

## Command

| Command | NArg | Arg1                  | Arg2                  | Arg3                  |
| ------- | ---- | --------------------- | --------------------- | --------------------- |
| ls      | 1    | path [cstring]        |                       |                       |
| cp      | 2    | source path [cstring] | target path [cstring] | target path [cstring] |

...