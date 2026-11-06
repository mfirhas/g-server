## Testing Logging and Tracing

| Logging | Tracing         | Result | Desc.                             |
| ------- | --------------- | ------ | --------------------------------- |
| off     | off             | ✅     | both print nothing                |
| ON      | off             | ✅     | tracing not logged                |
| off     | ON(NO TRACE LOG)| ✅     | log not printed                   |
| off     | ON(TRACE LOG)   | ✅     | log printed following trace format|
| ON      | ON(NO TRACE LOG)| ✅     | each printed in their own format  |
| ON      | ON(TRACE LOG)   | ✅     | log printed following trace format|


## Testing Metrics
- Collector: prometheus
- Protocol: http (prometheus only support pull and otel via http)

| Prometheus | OTel | Result | Desc. |
| ---------- | ---- | ------ | ----- |
| off        | off  |   ✅   |  Server starts normally. | 
| ON         | off  |   ✅   |  Server starts normally, prometheus pull metrics from app endpoint, and showed in prometheus dashboard. | 
| off        | ON   |        |       |
| off        | ON   |        |       |
| ON         | ON   |        |       |
| ON         | ON   |        |       |
