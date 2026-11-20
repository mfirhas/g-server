## Testing Logging and Tracing

| logging | tracing | capture_log | result | desc                                                  |
| ------- | ------- | ----------- | ------ | ----------------------------------------------------- |
| false   | false   | false       |   ✅   | nothing printed.                                      |
| false   | false   | true        |   ✅   | nothing printed.                                      |
| false   | true    | false       |   ✅   | traces printed.                                       |
| false   | true    | true        |   ✅   | logs printed following traces format, traces printed. |
| true    | false   | false       |   ✅   | logs printed.                                         |
| true    | false   | true        |   ✅   | logs printed.                                         |
| true    | true    | false       |   ✅   | logs printed, traces printed.                         |
| true    | true    | true        |   ✅   | logs printed, traces printed.                         |

## Testing Metrics
- Collector: prometheus
- Method: pull-based with metrics endpoint
- Status: ✅
- Desc: endpoint served, metrics show in dashboard

## Testing OTel Logging
- Collector: loki
- Method: push-based with otel
- Protocol: http (loki only supports http for otel)
- Dependency: reqwest-blocking-client

| buffered | result         | desc               |
| -------- | -------------- | ------------------ |
| True     |       ✅       | logs showed in loki, not in shell |
| False    |       ❌       | SimpleLogger logs processor only allow logging inside non-tokio context[^1] |

[^1]: https://docs.rs/opentelemetry_sdk/latest/opentelemetry_sdk/logs/struct.SimpleLogProcessor.html

## Testing OTel Tracing
- Collector: tempo
- Method: push-based with otel
- Dependency: reqwest-blocking-client



## Testing OTel Metrics
- Collector: prometheus
- Method: push-based with otel
- Protocol: http (prometheus only supports http for otel)
- Dependency: reqwest-blocking-client
- Status: ✅
- Desc: metrics show up in prometheus dashboard
- Note: putting invalid endpoint: requests work normal, just no metrics recorded, with logs telling not found endpoint.


