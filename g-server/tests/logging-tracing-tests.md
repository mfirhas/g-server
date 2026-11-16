## Testing Logging and Tracing

| logging | capture traces | tracing | capture logs | result |                        desc                          |
| ------- | -------------- | ------- | ------------ | ------ | ---------------------------------------------------- |
| False   | False          | False   | False        |   ✅   | nothing printed                                      |
| False   | False          | False   | True         |   ✅   | nothing printed                                      |
| False   | False          | True    | False        |   ✅   | NO logs printed, traces printed                      |
| False   | False          | True    | True         |   ✅   | logs printed following traces format, traces printed |
| False   | True           | False   | False        |   ✅   | NO logs printed, NO traces printed                   |
| False   | True           | False   | True         |   ✅   | NO logs printed, NO traces printed                   |
| False   | True           | True    | False        |   ✅   | NO logs printed, traces printed                      |
| False   | True           | True    | True         |   ✅   | logs printed following traces format, traces printed |
| True    | False          | False   | False        |   ✅   | logs printed, NO traces printed                      |
| True    | False          | False   | True         |   ✅   | logs printed, NO traces printed                      |
| True    | False          | True    | False        |   ✅   | logs printed, traces printed                         |
| True    | False          | True    | True         |   ✅   | logs printed, traces printed                         |
| True    | True           | False   | False        |   ✅   | logs printed, traces printed following logs format   |
| True    | True           | False   | True         |   ✅   | logs printed, traces printed following logs format   |
| True    | True           | True    | False        |   ✅   | logs printed, traces printed                         |
| True    | True           | True    | True         |   ✅   | logs printed, traces printed                         |

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

## Testing OTel Metrics
- Collector: prometheus
- Method: push-based with otel
- Protocol: http (prometheus only supports http for otel)
- Dependency: reqwest-blocking-client
- Status: ✅
- Desc: metrics show up in prometheus dashboard
- Note: putting invalid endpoint: requests work normal, just no metrics recorded, with logs telling not found endpoint.


