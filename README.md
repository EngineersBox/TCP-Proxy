# TCP-Proxy

![GitHub Workflow Status](https://img.shields.io/github/workflow/status/EngineersBox/TCP-Proxy/Rust?style=for-the-badge)


TCP-Proxy is a Rust based proxy and traffic filtering service. It's built around an allocator-acceptor model to serve, collate and forward requests/responses.
Connections are defined via a JSON file describing:

* Source and destination endpoints
* Egress/Ingress rule sets to apply to requests and responses selectively
* Type of rules and associated parameters or configurations to use as a basis for the rules

## Basc Schema

```json
{
	"bindings": [
		{
			"name": "<STRING>",
			"from": "<ADDR:PORT>",
			"to": "<ADDR:PORT>",
			"rules": {
				"ingress": [
					{
						"kind": "<HEADER | URL | METHOD | VERSION>",
						"header_mappings": [
							{
								"key": "<STRING>",
								"value": "<STRING>"
							}
						],
                        "url_wildcard": "<REGEX>",
                        "method_enum": "<GET | POST | PATCH | PUT | OPTIONS | UPDATE>",
                        "version_float": "<FLOAT>"
					}
				],
				"egress": [
					{
						"kind": "<HEADER | URL | METHOD | VERSION>",
						"header_mappings": [
							{
								"key": "<STRING>",
								"value": "<STRING>"
							}
						],
						"url_wildcard": "<REGEX>",
						"method_enum": "<GET | POST | PATCH | PUT | OPTIONS | UPDATE>",
						"version_float": "<FLOAT>"
					}
				]
			}
		}
	]
}
```

It is important to note that different fields in the rules are required depending on the kind of rule. They are as follows:

| **Kind**  | **Required Field**|
|---------	|-----------------	|
| `HEADER`  | `header_mappings` |
| `URL`     | `url_wildcard`    |
| `METHOD`  | `method_enum`     |
| `VERSION`	| `version_float`   |

## Example Rule Binding JSON

```json
{
	"bindings": [
		{
			"name": "test1",
			"from": "localhost:3000",
			"to": "google.com:80",
			"rules": {
				"ingress": [
					{
						"kind": "HEADER",
						"header_mappings": [
							{
								"key": "content-type",
								"value": "application/json"
							}
						]
					}
				],
				"egress": [
					{
						"kind": "VERSION",
						"version_float": 1.1
					},
					{
						"kind": "URL",
						"url_wildcard": "https://.*\\.instaclustr\\.com"
					}
				]
			}
		}
	]
}
```

## Service Configuration

There are a few configuration properties that can be set to change the way resources are used and what types of handler are used with the service.
These are specified via a `config.properties` in the `config` directory:

| **Property**                 	| **Type**  	| **Description**                                                                                                                                                            	| **Default Value** 	|
|------------------------------	|-----------	|----------------------------------------------------------------------------------------------------------------------------------------------------------------------------	|-------------------	|
| `allocator_thread_pool_size` 	| `INTEGER` 	| Set the amount of threads to reserve in a pool to act as allocators for threads to handle connections                                                                      	| `10`              	|
| `handler_thread_pool_size`   	| `INTEGER` 	| Set the amount of threads to reserve in a pool to handle connections with                                                                                                  	| `50`              	|
| `thread_handler_type`        	| `ENUM`    	| What method of packet handling should be used:<br>* `PROGRESSIVE` = Forward packets as they come in<br>* `CAPTURE` = Buffer all packets and then forward once all collated 	| `PROGRESSIVE`     	|

## Example Logging

TCP-Proxy logs all the activity from within to stdout via the `slog` library. Utilising the above example rule bindings, we can see the output to stdout is as follows:
```log
Feb 26 23:59:47.477 INFO Logging directory already exists, skipping
Feb 26 23:59:47.478 DEBG Creating proxy thread pool of size: 50
Feb 26 23:59:47.479 INFO Initializing proxy 2 binding(s)
Feb 26 23:59:47.482 INFO Multiple SocketAddr resolutions [localhost:3000] -> [127.0.0.1:3000, [::1]:3000], defaulting to [127.0.0.1:3000]
Feb 26 23:59:47.516 DEBG Binding listener [0] to connection: localhost:3000 <-> google.com:80 
Feb 26 23:59:47.516 DEBG Invoked acceptor thread for listener [0] using hadler type [could not read file: PROGRESSIVE]
Feb 26 23:59:47.517 INFO Starting main listener loop
Feb 26 23:59:59.604 DEBG New connection
Feb 26 23:59:59.637 DEBG REQUEST CONTENT [EGRESS]:

GET / HTTP/1.1
Host: 127.0.0.1:3000
User-Agent: curl/7.64.1
Accept: */*
X-Test-Header: somevalue

Feb 26 23:59:59.637 INFO TRAFFIC LOG [EGRESS] [96750fe7-80be-4789-810c-fea6fc951808]
Feb 26 23:59:59.794 DEBG RESPONSE CONTENT [EGRESS]:

HTTP/1.1 301 Moved Permanently
Location: http://www.google.com:3000/
Content-Type: text/html; charset=UTF-8
Date: Fri, 26 Feb 2021 12:59:59 GMT
Expires: Sun, 28 Mar 2021 12:59:59 GMT
Cache-Control: public, max-age=2592000
Server: gws
Content-Length: 224
X-XSS-Protection: 0
X-Frame-Options: SAMEORIGIN

<HTML><HEAD><meta http-equiv="content-type" content="text/html;charset=utf-8">
<TITLE>301 Moved</TITLE></HEAD><BODY>
<H1>301 Moved</H1>
The document has moved
<A HREF="http://www.google.com:3000/">here</A>.
</BODY></HTML>

Feb 26 23:59:59.794 DEBG Client closed connection

```
