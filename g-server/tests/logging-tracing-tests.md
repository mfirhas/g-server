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
- pull-based with metrics endpoint
- status: ✅
- desc: endpoint served, metrics captured in dashboard
