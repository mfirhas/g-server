## Testing Logging and Tracing

| Logging | Tracing         | Result | Desc.                             |
| ------- | --------------- | ------ | --------------------------------- |
| off     | off             | ✅     | both print nothing                |
| ON      | off             | ❌     | tracing logged(it should not)     |
| off     | ON(NO TRACE LOG)| ✅     | log not printed                   |
| off     | ON(TRACE LOG)   | ✅     | log printed following trace format|
| ON      | ON(NO TRACE LOG)| ✅     | each printed in their own format  |
| ON      | ON(TRACE LOG)   | ✅     | log printed following trace format|

