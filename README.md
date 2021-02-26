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
			"from": "127.0.0.1:3000",
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