
# To codex
* 目标：生成一个命令行工具，用于不断向 stdout 输出日志，从而可以做日志处理工具的性能压测
* 输出：带命令参数行的 linux 下的执行程序
* 命令行参数约定：
  - sleep_ms: 每次输出日志块后的睡眠的毫秒数。默认为 0
  - batch_bytes: 每次输出日志的字节数。可以写为：1g, 1m, 1k 等。
  - output=stdout/http
  - http.jsonline=http://xxx:9428/insert/jsonline?_time_field=_time&_msg_field=_msg,http_request_query_string&_stream_fields=server_name,http_request_path,status_code&ignore_fields=&decolorize_fields=&AccountID=0&ProjectID=0&debug=false&extra_fields=
* 计算过程：
  - 解析命令行的参数
  - 分配 batch_bytes 指定的 buffer
  - 在死循环中执行 log 生成逻辑
    - 生成一行 json 格式的日志
    - 如果缓冲区的大小足够，加入到缓冲区中，末尾补充 \n 字符
    - 重复生成一行，直到缓冲区没有足够空间
    - 把整个缓冲区写到 stdout
    - 清空缓冲区，继续下一个批次的日志生成

## 生成日志格式要求：

日志的样例如下:

```json
{ "time":"06/Nov/2025:04:53:43.724","client_ip":"2408:8469:27d0:231:c8f4:5fa:c8ff:a05a","bytes_read":"1138","captured_request_headers":"api.xxx.com - 2409:890e:bc8:bf9:1875:29c3:9f67:5d4e -","http_method":"POST","http_request_path":"/game-api/ways/v2/Spin","http_request_query_string":"?traceId=BQEIHM06","http_version":"HTTP/1.1","server_name":"host-190","status_code":"200","ta":"61","tc":"0","termination_state":"--","tr_client":"0","tr_server":"61","tw":"0"}
```

各个字段说明如下：

* time: 使用当前时间，输出格式为 `06/Nov/2025:04:53:43.724`
* client_ip: 随机生成 ipv4 或者 ipv6
* bytes_read: 随机数
* captured_request_headers: 使用 ${host} - ${client_ip} 的格式。 host 随机生成
* http_method: 随机选择 POST/GET/HEAD/PUT 中的一种
* http_request_path: 随机生成一个路径
* http_request_query_string: 写成 `?traceId=${traceId}` 的格式。traceId 为 8 个字符的 A-Za-z0-9 构成的随机字符串
* http_version: 随机选择 1.1/2.0/3.0 中的一种
* server_name: 随机生成
* status_code: 随机选择一种存在的 http 状态码
* 其他字段原样保留

## build 要求

* 交叉编译为 linux amd64 架构可以运行的二进制程序
* 生成 dockerfile，为后续推送到 hub.docker.com 做准备

